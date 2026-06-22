use actix_web::{web, HttpRequest, HttpResponse};
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::data::{self, BudgetRow, TransactionWithCategory};
use crate::middleware::issue_token;
use crate::response::{ApiResponse, AppError};

// ========================= helpers =========================
fn uid(req: &HttpRequest) -> Result<i64, AppError> {
    crate::response::get_user_id(req)
}

// ========================= health =========================
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(json!({ "status": "ok" }))
}

// ========================= auth =========================
#[derive(Debug, Deserialize)]
pub struct RegisterReq {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResp {
    pub token: String,
    pub user: UserInfo,
}

pub async fn register(
    state: web::Data<data::AppState>,
    body: web::Json<RegisterReq>,
) -> Result<HttpResponse, AppError> {
    if body.username.len() < 3 || body.username.len() > 50 {
        return Err(AppError::BadRequest("用户名长度需在 3-50 之间".into()));
    }
    if body.password.len() < 6 {
        return Err(AppError::BadRequest("密码至少 6 位".into()));
    }
    if let Some(email) = &body.email {
        if !email.contains('@') {
            return Err(AppError::BadRequest("邮箱格式不正确".into()));
        }
    }

    if data::get_user_by_username(&state.pool, &body.username).await?.is_some() {
        return Err(AppError::Conflict("用户名已存在".into()));
    }

    let hash = bcrypt::hash(&body.password, bcrypt::DEFAULT_COST)?;
    let user_id = data::create_user(&state.pool, &body.username, &hash, body.email.as_deref()).await?;
    let user = data::get_user_by_id(&state.pool, user_id).await?
        .ok_or_else(|| AppError::InternalError("用户创建失败".into()))?;

    let token = issue_token(&state.jwt_secret, user.id, &user.username)
        .map_err(|e| AppError::InternalError(format!("Token 生成失败: {}", e)))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(LoginResp {
        token,
        user: UserInfo { id: user.id, username: user.username, email: user.email },
    })))
}

pub async fn login(
    state: web::Data<data::AppState>,
    body: web::Json<LoginReq>,
) -> Result<HttpResponse, AppError> {
    let user = data::get_user_by_username(&state.pool, &body.username).await?
        .ok_or_else(|| AppError::Unauthorized("用户名或密码错误".into()))?;

    if !bcrypt::verify(&body.password, &user.password_hash).unwrap_or(false) {
        return Err(AppError::Unauthorized("用户名或密码错误".into()));
    }

    let token = issue_token(&state.jwt_secret, user.id, &user.username)
        .map_err(|e| AppError::InternalError(format!("Token 生成失败: {}", e)))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(LoginResp {
        token,
        user: UserInfo { id: user.id, username: user.username, email: user.email },
    })))
}

pub async fn profile(
    state: web::Data<data::AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;
    let user = data::get_user_by_id(&state.pool, user_id).await?
        .ok_or_else(|| AppError::NotFound("用户不存在".into()))?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(UserInfo {
        id: user.id, username: user.username, email: user.email,
    })))
}

// ========================= categories =========================
#[derive(Debug, Serialize)]
pub struct CategoryView {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub category_type: String,
    pub is_system: bool,
}

pub async fn list_categories(
    state: web::Data<data::AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;
    let cats = data::list_categories(&state.pool, user_id).await?;
    let out: Vec<CategoryView> = cats.into_iter().map(|c| CategoryView {
        id: c.id, name: c.name, category_type: c.category_type, is_system: c.user_id.is_none(),
    }).collect();
    Ok(HttpResponse::Ok().json(ApiResponse::success(out)))
}

// ========================= transactions =========================
#[derive(Debug, Deserialize)]
pub struct CreateTxReq {
    pub category_id: i64,
    pub amount: Decimal,
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub occurred_at: String,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTxReq {
    pub category_id: Option<i64>,
    pub amount: Option<Decimal>,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListTxQuery {
    pub month: Option<String>,
    #[serde(rename = "type")]
    pub tx_type: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TxView {
    pub id: i64,
    pub category_id: i64,
    pub category_name: String,
    pub amount: Decimal,
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub occurred_at: String,
    pub remark: Option<String>,
}

fn to_tx_view(t: TransactionWithCategory) -> TxView {
    TxView {
        id: t.id,
        category_id: t.category_id,
        category_name: t.category_name,
        amount: t.amount,
        transaction_type: t.transaction_type,
        occurred_at: t.occurred_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        remark: t.remark,
    }
}

pub async fn list_transactions(
    state: web::Data<data::AppState>,
    req: HttpRequest,
    q: web::Query<ListTxQuery>,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).min(100);

    let (items, total) = data::list_transactions(
        &state.pool, user_id, q.month.as_deref(), q.tx_type.as_deref(), page, page_size
    ).await?;

    let views: Vec<TxView> = items.into_iter().map(to_tx_view).collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(json!({
        "items": views,
        "total": total,
        "page": page,
        "page_size": page_size,
        "total_pages": (total + page_size - 1) / page_size,
    }))))
}

pub async fn create_transaction(
    state: web::Data<data::AppState>,
    req: HttpRequest,
    body: web::Json<CreateTxReq>,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;

    if body.amount <= Decimal::ZERO {
        return Err(AppError::BadRequest("金额必须大于 0".into()));
    }
    if body.transaction_type != "income" && body.transaction_type != "expense" {
        return Err(AppError::BadRequest("type 必须是 income 或 expense".into()));
    }

    let cat = data::get_category(&state.pool, body.category_id).await?
        .ok_or_else(|| AppError::BadRequest("分类不存在".into()))?;
    if cat.category_type != body.transaction_type {
        return Err(AppError::BadRequest("分类类型与账单类型不匹配".into()));
    }

    let occurred_at = NaiveDateTime::parse_from_str(&body.occurred_at, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(&body.occurred_at, "%Y-%m-%dT%H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(&body.occurred_at, "%Y-%m-%dT%H:%M"))
        .map_err(|_| AppError::BadRequest("时间格式错误，应为 YYYY-MM-DD HH:MM:SS".into()))?;

    let id = data::create_transaction(
        &state.pool, user_id, body.category_id, body.amount,
        &body.transaction_type, occurred_at, body.remark.as_deref(),
    ).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(json!({ "id": id }))))
}

pub async fn update_transaction(
    state: web::Data<data::AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<UpdateTxReq>,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;
    let id = path.into_inner();

    let owner = data::get_transaction_owner(&state.pool, id).await?;
    if owner.is_none() { return Err(AppError::NotFound("账单不存在".into())); }
    if owner.unwrap() != user_id { return Err(AppError::Forbidden("无权修改".into())); }

    let (existing_items, _) = data::list_transactions(&state.pool, user_id, None, None, 1, 100).await?;
    let existing = existing_items.into_iter().find(|t| t.id == id)
        .ok_or_else(|| AppError::NotFound("账单不存在".into()))?;

    let category_id = body.category_id.unwrap_or(existing.category_id);
    let amount = body.amount.unwrap_or(existing.amount);
    let remark = body.remark.clone().or(existing.remark);

    if amount <= Decimal::ZERO {
        return Err(AppError::BadRequest("金额必须大于 0".into()));
    }

    if body.category_id.is_some() {
        let cat = data::get_category(&state.pool, category_id).await?
            .ok_or_else(|| AppError::BadRequest("分类不存在".into()))?;
        if let Some(uid_cat) = cat.user_id {
            if uid_cat != user_id {
                return Err(AppError::Forbidden("无权使用该分类".into()));
            }
        }
    }

    data::update_transaction(&state.pool, id, category_id, amount, remark.as_deref()).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok_empty()))
}

pub async fn delete_transaction(
    state: web::Data<data::AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;
    let id = path.into_inner();

    let owner = data::get_transaction_owner(&state.pool, id).await?;
    if owner.is_none() { return Err(AppError::NotFound("账单不存在".into())); }
    if owner.unwrap() != user_id { return Err(AppError::Forbidden("无权删除".into())); }

    data::delete_transaction(&state.pool, id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok_empty()))
}

// ========================= budgets =========================
#[derive(Debug, Deserialize)]
pub struct CreateBudgetReq {
    pub category_id: Option<i64>,
    pub amount: Decimal,
    pub month: String,
}

#[derive(Debug, Deserialize)]
pub struct ListBudgetQuery {
    pub month: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BudgetView {
    pub id: i64,
    pub category_id: Option<i64>,
    pub category_name: String,
    pub amount: Decimal,
    pub spent: Decimal,
    pub remaining: Decimal,
    pub usage_percent: i64,
    pub month: String,
}

fn to_budget_view(b: BudgetRow) -> BudgetView {
    let usage = if b.amount <= Decimal::ZERO {
        0i64
    } else {
        let percent = (b.spent / b.amount) * Decimal::from(100);
        percent.to_string().parse::<f64>().unwrap_or(0.0).round() as i64
    };
    BudgetView {
        id: b.id,
        category_id: b.category_id,
        category_name: if b.category_id.is_none() { "总预算".to_string() } else { b.category_name },
        amount: b.amount,
        spent: b.spent,
        remaining: b.amount - b.spent,
        usage_percent: usage,
        month: b.month,
    }
}

pub async fn list_budgets(
    state: web::Data<data::AppState>,
    req: HttpRequest,
    q: web::Query<ListBudgetQuery>,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;
    let month = q.month.clone().unwrap_or_else(data::current_month);

    let budgets = data::list_budgets(&state.pool, user_id, &month).await?;
    let views: Vec<BudgetView> = budgets.into_iter().map(to_budget_view).collect();
    Ok(HttpResponse::Ok().json(ApiResponse::success(views)))
}

pub async fn create_or_update_budget(
    state: web::Data<data::AppState>,
    req: HttpRequest,
    body: web::Json<CreateBudgetReq>,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;

    if body.amount < Decimal::ZERO {
        return Err(AppError::BadRequest("预算金额不能为负数".into()));
    }
    if body.month.len() != 7 || !body.month.contains('-') {
        return Err(AppError::BadRequest("月份格式应为 YYYY-MM".into()));
    }

    if let Some(cid) = body.category_id {
        let cat = data::get_category(&state.pool, cid).await?
            .ok_or_else(|| AppError::BadRequest("分类不存在".into()));
        let cat = cat?;
        if let Some(u) = cat.user_id {
            if u != user_id { return Err(AppError::Forbidden("无权使用该分类".into())); }
        }
    }

    let id = data::upsert_budget(&state.pool, user_id, body.category_id, body.amount, &body.month).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(json!({ "id": id, "month": body.month }))))
}

pub async fn delete_budget(
    state: web::Data<data::AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;
    let id = path.into_inner();

    let owner = data::get_budget_owner(&state.pool, id).await?;
    if owner.is_none() { return Err(AppError::NotFound("预算不存在".into())); }
    if owner.unwrap() != user_id { return Err(AppError::Forbidden("无权删除".into())); }

    data::delete_budget(&state.pool, id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok_empty()))
}

// ========================= statistics =========================
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub month: String,
}

pub async fn monthly_statistics(
    state: web::Data<data::AppState>,
    req: HttpRequest,
    q: web::Query<StatsQuery>,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;
    let month = q.month.clone();

    let (income, expense) = data::month_summary(&state.pool, user_id, &month).await?;
    let cats = data::category_expense_summary(&state.pool, user_id, &month).await?;
    let total_budget = data::total_budget(&state.pool, user_id, &month).await?;

    // 构造分类 -> 金额 的 map
    let mut category_totals = serde_json::Map::new();
    for (name, amt) in &cats {
        category_totals.insert(name.clone(), json!(amt));
    }

    let budget_json = if let Some(b) = total_budget {
        let usage_percent = if b > Decimal::ZERO {
            let percent = (expense / b) * Decimal::from(100);
            percent.to_string().parse::<f64>().unwrap_or(0.0).round() as i64
        } else { 0 };
        json!({
            "amount": b,
            "spent": expense,
            "remaining": b - expense,
            "usage_percent": usage_percent,
            "is_over": expense > b
        })
    } else {
        serde_json::Value::Null
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(json!({
        "income": income,
        "expense": expense,
        "balance": income - expense,
        "month": month,
        "category_totals": category_totals,
        "budget": budget_json,
    }))))
}

// ========================= AI =========================
#[derive(Debug, Deserialize)]
pub struct ReportReq {
    pub month: String,
}

#[derive(Debug, Deserialize)]
pub struct ClassifyReq {
    pub remark: String,
    pub amount: Decimal,
    #[serde(rename = "type")]
    pub transaction_type: String,
}

struct ClassifyRule {
    tx_type: &'static str,
    category_name: &'static str,
    keywords: &'static [&'static str],
    confidence: f64,
}

const CLASSIFY_RULES: &[ClassifyRule] = &[
    ClassifyRule {
        tx_type: "expense",
        category_name: "餐饮",
        keywords: &[
            "早餐", "午餐", "晚餐", "外卖", "饭", "奶茶", "咖啡", "食堂", "火锅",
        ],
        confidence: 0.90,
    },
    ClassifyRule {
        tx_type: "expense",
        category_name: "交通",
        keywords: &[
            "地铁", "公交", "打车", "滴滴", "出租", "高铁", "火车", "机票", "加油",
        ],
        confidence: 0.90,
    },
    ClassifyRule {
        tx_type: "expense",
        category_name: "购物",
        keywords: &["淘宝", "京东", "拼多多", "购物", "衣服", "鞋", "超市"],
        confidence: 0.88,
    },
    ClassifyRule {
        tx_type: "expense",
        category_name: "学习",
        keywords: &["书", "课程", "学费", "资料", "考试", "培训"],
        confidence: 0.88,
    },
    ClassifyRule {
        tx_type: "expense",
        category_name: "娱乐",
        keywords: &["电影", "游戏", "会员", "旅游", "演唱会", "KTV"],
        confidence: 0.86,
    },
    ClassifyRule {
        tx_type: "expense",
        category_name: "医疗",
        keywords: &["药", "医院", "门诊", "体检", "挂号"],
        confidence: 0.90,
    },
    ClassifyRule {
        tx_type: "expense",
        category_name: "住房",
        keywords: &["房租", "水电", "物业", "宽带", "燃气"],
        confidence: 0.92,
    },
    ClassifyRule {
        tx_type: "income",
        category_name: "工资",
        keywords: &["工资", "薪资", "薪水", "工资到账"],
        confidence: 0.95,
    },
    ClassifyRule {
        tx_type: "income",
        category_name: "奖金",
        keywords: &["奖金", "绩效", "年终奖"],
        confidence: 0.92,
    },
    ClassifyRule {
        tx_type: "income",
        category_name: "兼职",
        keywords: &["兼职", "外包", "稿费", "劳务费"],
        confidence: 0.88,
    },
    ClassifyRule {
        tx_type: "income",
        category_name: "报销",
        keywords: &["报销", "补贴"],
        confidence: 0.88,
    },
    ClassifyRule {
        tx_type: "income",
        category_name: "理财",
        keywords: &["利息", "基金", "分红", "收益", "理财"],
        confidence: 0.86,
    },
];

#[derive(Debug, Deserialize)]
struct AiClassifyOutput {
    category_name: String,
    confidence: Option<f64>,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AiReportOutput {
    summary: Option<String>,
    highlights: Option<Vec<String>>,
    risks: Option<Vec<String>>,
    saving_tips: Option<Vec<String>>,
    budget_status: Option<String>,
}

fn chat_completion_url(base_url: &str) -> String {
    format!("{}/chat/completions", base_url.trim_end_matches('/'))
}

fn parse_ai_json(content: &str) -> Option<serde_json::Value> {
    let mut text = content.trim();
    if text.starts_with("```") {
        text = text
            .trim_start_matches("```json")
            .trim_start_matches("```JSON")
            .trim_start_matches("```")
            .trim();
        if let Some(end) = text.rfind("```") {
            text = &text[..end];
        }
    }
    serde_json::from_str(text.trim()).ok()
}

fn clamp_confidence(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

async fn call_ai_classify(
    api_key: &str,
    base_url: &str,
    model: &str,
    tx_type: &str,
    amount: Decimal,
    remark: &str,
    category_names: &[String],
) -> Option<(String, f64, String)> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .ok()?;

    let prompt = format!(
        "你是个人记账分类助手。请根据账单类型、金额、备注，从候选分类中选择最合适的一个分类。
只能返回 JSON，不要 Markdown，不要解释性长文本。
只能从候选分类中选择，不能新增分类，不能返回候选分类之外的名称。

账单类型：{}
金额：{}
备注：{}
候选分类：{}

JSON 返回格式：
{{\"category_name\":\"分类名\",\"confidence\":0.0到1.0之间,\"reason\":\"简短原因\"}}",
        tx_type,
        amount,
        remark,
        category_names.join("、")
    );

    let resp = client
        .post(chat_completion_url(base_url))
        .bearer_auth(api_key.trim())
        .json(&json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": "你是个人财务记账分类助手。必须输出 JSON，且只能从用户给出的候选分类中选择一个分类。"
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.2,
            "response_format": { "type": "json_object" }
        }))
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let value: serde_json::Value = resp.json().await.ok()?;
    let content = value["choices"][0]["message"]["content"].as_str()?.trim();
    let parsed: AiClassifyOutput = serde_json::from_value(parse_ai_json(content)?).ok()?;
    let category_name = parsed.category_name.trim().to_string();
    if !category_names.iter().any(|name| name == &category_name) {
        return None;
    }

    Some((
        category_name,
        clamp_confidence(parsed.confidence.unwrap_or(0.75)),
        parsed.reason.unwrap_or_else(|| "AI 推荐分类".to_string()),
    ))
}

fn report_items(report: &AiReportOutput) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(summary) = &report.summary {
        if !summary.trim().is_empty() {
            out.push(summary.trim().to_string());
        }
    }
    if let Some(items) = &report.highlights {
        out.extend(items.iter().filter(|s| !s.trim().is_empty()).cloned());
    }
    if let Some(items) = &report.risks {
        out.extend(items.iter().filter(|s| !s.trim().is_empty()).cloned());
    }
    if let Some(items) = &report.saving_tips {
        out.extend(items.iter().filter(|s| !s.trim().is_empty()).cloned());
    }
    if let Some(status) = &report.budget_status {
        if !status.trim().is_empty() {
            out.push(status.trim().to_string());
        }
    }
    out
}

async fn call_ai_report(
    api_key: &str,
    base_url: &str,
    model: &str,
    month: &str,
    income: Decimal,
    expense: Decimal,
    balance: Decimal,
    category_totals: &[(String, Decimal)],
    budget_usage: Option<i64>,
    suggestions: &[String],
) -> Option<AiReportOutput> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .ok()?;

    let category_summary = if category_totals.is_empty() {
        "暂无".to_string()
    } else {
        category_totals
            .iter()
            .map(|(name, amount)| format!("{}：{}元", name, amount))
            .collect::<Vec<_>>()
            .join("；")
    };

    let prompt = format!(
        "你是个人财务分析助手。请根据下面的月度财务摘要，生成中文 JSON 财务分析报告。
要求：
1. 只根据给定统计摘要分析，不要编造数据；
2. 不要出现用户名、邮箱、用户 ID、JWT、API key；
3. 不给投资、贷款、医疗、法律等高风险建议；
4. 输出必须是 JSON，不要 Markdown；
5. JSON 字段固定为 summary、highlights、risks、saving_tips、budget_status。

月份：{}
收入：{}
支出：{}
结余：{}
分类支出汇总：{}
预算使用率：{}
本地规则建议：{}

JSON 返回格式：
{{\"summary\":\"一句话总结\",\"highlights\":[\"消费亮点\"],\"risks\":[\"风险提醒\"],\"saving_tips\":[\"省钱建议\"],\"budget_status\":\"预算状态\"}}",
        month,
        income,
        expense,
        balance,
        category_summary,
        budget_usage
            .map(|v| format!("{}%", v))
            .unwrap_or_else(|| "未设置预算".to_string()),
        suggestions.join("；")
    );

    let resp = client
        .post(chat_completion_url(base_url))
        .bearer_auth(api_key.trim())
        .json(&json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": "你是个人财务分析助手。必须输出 JSON，只能基于用户提供的统计摘要生成建议，不要编造不存在的数据。"
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.4,
            "response_format": { "type": "json_object" }
        }))
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let value: serde_json::Value = resp.json().await.ok()?;
    let content = value["choices"][0]["message"]["content"].as_str()?.trim();
    let report: AiReportOutput = serde_json::from_value(parse_ai_json(content)?).ok()?;
    if report_items(&report).is_empty() {
        None
    } else {
        Some(report)
    }
}

pub async fn ai_report(
    state: web::Data<data::AppState>,
    req: HttpRequest,
    body: web::Json<ReportReq>,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;
    let month = body.month.clone();

    let (income, expense) = data::month_summary(&state.pool, user_id, &month).await?;
    let cats = data::category_expense_summary(&state.pool, user_id, &month).await?;
    let total_budget = data::total_budget(&state.pool, user_id, &month).await?;

    let mut suggestions: Vec<String> = Vec::new();
    if expense == Decimal::ZERO && income == Decimal::ZERO {
        suggestions.push("本月暂无账单，可先录入收支以获取分析。".into());
    }

    if let Some(top) = cats.first() {
        suggestions.push(format!("最大支出类别为「{}」，请注意控制该类别的消费。", top.0));
    }

    if let Some(b) = total_budget {
        if b > Decimal::ZERO {
            let percent = (expense / b) * Decimal::from(100);
            let p = percent.to_string().parse::<f64>().unwrap_or(0.0).round();
            if p >= 100.0 {
                suggestions.push(format!("本月预算已使用 {:.0}%，超出预算，请控制支出。", p));
            } else if p >= 80.0 {
                suggestions.push(format!("本月预算已使用 {:.0}%，建议谨慎消费。", p));
            } else {
                suggestions.push(format!("预算使用进度 {:.0}%，整体可控。", p));
            }
        }
    } else {
        suggestions.push("暂未设置本月总预算，可设置总预算以获得更精准的提醒。".into());
    }

    if income - expense > Decimal::ZERO && expense != Decimal::ZERO {
        suggestions.push("本月结余为正，可考虑将部分资金用于储蓄或学习投入。".into());
    } else if income - expense < Decimal::ZERO && income != Decimal::ZERO {
        suggestions.push("本月支出超过收入，建议回顾大额账单并调整。".into());
    }

    if suggestions.is_empty() {
        suggestions.push("继续保持良好的记账习惯！".into());
    }

    let budget_usage = if let Some(b) = total_budget {
        if b > Decimal::ZERO {
            let percent = (expense / b) * Decimal::from(100);
            Some(percent.to_string().parse::<f64>().unwrap_or(0.0).round() as i64)
        } else {
            None
        }
    } else {
        None
    };

    let local_summary = format!(
        "{} 月财务总结：本月收入 {} 元，支出 {} 元，结余 {} 元。{}",
        month,
        income,
        expense,
        income - expense,
        suggestions.join("；")
    );
    let mut summary_text = local_summary.clone();
    let mut source = "fallback";
    let mut ai_status = if state.ai_enabled {
        "missing_api_key"
    } else {
        "disabled"
    };
    let mut generated_by_ai = false;

    if state.ai_enabled {
        if let Some(api_key) = &state.ai_api_key {
            ai_status = "request_failed";
            if let Some(ai_report) = call_ai_report(
                api_key,
                &state.ai_base_url,
                &state.ai_model,
                &month,
                income,
                expense,
                income - expense,
                &cats,
                budget_usage,
                &suggestions,
            )
            .await
            {
                let ai_items = report_items(&ai_report);
                if !ai_items.is_empty() {
                    suggestions = ai_items;
                }
                summary_text = ai_report.summary.unwrap_or(local_summary);
                source = "deepseek";
                ai_status = "success";
                generated_by_ai = true;
            }
        }
    }

    let budget_json = if let Some(b) = total_budget {
        let percent = (expense / b) * Decimal::from(100);
        let usage_percent = percent.to_string().parse::<f64>().unwrap_or(0.0).round() as i64;
        json!({ "amount": b, "spent": expense, "usage_percent": usage_percent })
    } else {
        serde_json::Value::Null
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(json!({
        "month": month,
        "income": income,
        "expense": expense,
        "balance": income - expense,
        "top_category": cats.first().map(|c| c.0.clone()),
        "budget": budget_json,
        "summary_text": summary_text,
        "source": source,
        "ai_status": ai_status,
        "suggestions": suggestions,
        "generated_by_ai": generated_by_ai,
    }))))
}

pub async fn ai_classify(
    state: web::Data<data::AppState>,
    req: HttpRequest,
    body: web::Json<ClassifyReq>,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;
    let cats = data::list_categories(&state.pool, user_id).await?;

    let remark_lower = body.remark.to_lowercase();
    let tx_type = match body.transaction_type.as_str() {
        "income" => "income",
        "expense" => "expense",
        _ => return Err(AppError::BadRequest("type 必须是 income 或 expense".into())),
    };

    for rule in CLASSIFY_RULES {
        if rule.tx_type != tx_type {
            continue;
        }

        for kw in rule.keywords {
            if remark_lower.contains(&kw.to_lowercase()) {
                if let Some(c) = cats
                    .iter()
                    .find(|c| c.category_type == tx_type && c.name == rule.category_name)
                {
                    return Ok(HttpResponse::Ok().json(ApiResponse::success(json!({
                        "category_id": c.id,
                        "category_name": c.name,
                        "confidence": rule.confidence,
                        "source": "rule",
                        "reason": format!("备注命中关键词：{}，推荐分类为{}", kw, c.name),
                    }))));
                }
            }
        }
    }

    if state.ai_enabled {
        if let Some(api_key) = &state.ai_api_key {
            let candidate_names: Vec<String> = cats
                .iter()
                .filter(|c| c.category_type == tx_type)
                .map(|c| c.name.clone())
                .collect();

            if !candidate_names.is_empty() {
                if let Some((ai_category_name, confidence, reason)) = call_ai_classify(
                    api_key,
                    &state.ai_base_url,
                    &state.ai_model,
                    tx_type,
                    body.amount,
                    &body.remark,
                    &candidate_names,
                )
                .await
                {
                    if let Some(c) = cats
                        .iter()
                        .find(|c| c.category_type == tx_type && c.name == ai_category_name)
                    {
                        return Ok(HttpResponse::Ok().json(ApiResponse::success(json!({
                            "category_id": c.id,
                            "category_name": c.name,
                            "confidence": confidence,
                            "source": "deepseek",
                            "reason": reason,
                        }))));
                    }
                }
            }
        }
    }

    // fallback: 找"其他"或类型相符的第一个分类
    let fallback_name = if tx_type == "income" { "其他收入" } else { "其他" };
    let fallback = cats.iter().find(|c| c.category_type == tx_type && c.name == fallback_name)
        .or_else(|| cats.iter().find(|c| c.category_type == tx_type));

    match fallback {
        Some(c) => Ok(HttpResponse::Ok().json(ApiResponse::success(json!({
            "category_id": c.id,
            "category_name": c.name,
            "confidence": 0.5,
            "source": "fallback",
            "reason": "本地规则未命中，AI 不可用或返回无效，使用默认分类",
        })))),
        None => Err(AppError::NotFound("未找到匹配的分类".into())),
    }
}
