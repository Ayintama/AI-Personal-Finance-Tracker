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

fn validate_month(month: &str) -> Result<(), AppError> {
    if month.len() != 7 {
        return Err(AppError::BadRequest("month 格式必须为 YYYY-MM".into()));
    }
    let bytes = month.as_bytes();
    if bytes[4] != b'-'
        || !bytes[0..4].iter().all(|c| c.is_ascii_digit())
        || !bytes[5..7].iter().all(|c| c.is_ascii_digit())
    {
        return Err(AppError::BadRequest("month 格式必须为 YYYY-MM".into()));
    }
    let m: u32 = std::str::from_utf8(&bytes[5..7]).unwrap_or("0").parse().unwrap_or(0);
    if m < 1 || m > 12 {
        return Err(AppError::BadRequest("月份必须在 01-12 之间".into()));
    }
    Ok(())
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
    if body.amount > Decimal::new(9999999, 2) {
        return Err(AppError::BadRequest("金额超出最大限制 (9999999.99)".into()));
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

    if amount > Decimal::new(9999999, 2) {
        return Err(AppError::BadRequest("金额超出最大限制 (9999999.99)".into()));
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

    validate_month(&month)?;

    let (income, expense) = data::month_summary(&state.pool, user_id, &month).await?;
    let expense_cats = data::category_expense_summary(&state.pool, user_id, &month).await?;
    let income_cats = data::category_income_summary(&state.pool, user_id, &month).await?;
    let total_budget = data::total_budget(&state.pool, user_id, &month).await?;
    let max_expense = data::max_single_expense(&state.pool, user_id, &month).await?;
    let tx_count = data::monthly_transaction_count(&state.pool, user_id, &month).await?;
    let days = data::days_in_month(&month);
    let daily_avg = if days > 0 && expense > Decimal::ZERO {
        expense / Decimal::from(days)
    } else {
        Decimal::ZERO
    };

    // 支出分类 -> 金额
    let mut category_totals = serde_json::Map::new();
    for (name, amt) in &expense_cats {
        category_totals.insert(name.clone(), json!(amt));
    }

    // 收入分类 -> 金额
    let mut income_category_totals = serde_json::Map::new();
    for (name, amt) in &income_cats {
        income_category_totals.insert(name.clone(), json!(amt));
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
        "income_category_totals": income_category_totals,
        "budget": budget_json,
        "daily_avg_spending": daily_avg,
        "max_expense": max_expense,
        "transaction_count": tx_count,
    }))))
}

// ========================= yearly / daily / trend =========================

#[derive(Debug, Deserialize)]
pub struct YearlyStatsQuery {
    pub year: i32,
}

pub async fn yearly_statistics(
    state: web::Data<data::AppState>,
    req: HttpRequest,
    q: web::Query<YearlyStatsQuery>,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;
    let year = q.year;

    if year < 2000 || year > 2100 {
        return Err(AppError::BadRequest("年份超出合理范围 (2000-2100)".into()));
    }

    let rows = data::yearly_summary(&state.pool, user_id, year).await?;

    let mut data_map: std::collections::HashMap<String, (Decimal, Decimal)> = std::collections::HashMap::new();
    for (mth, inc, exp) in &rows {
        data_map.insert(mth.clone(), (*inc, *exp));
    }

    let mut months: Vec<serde_json::Value> = Vec::with_capacity(12);
    let mut yearly_income = Decimal::ZERO;
    let mut yearly_expense = Decimal::ZERO;

    for m in 1..=12 {
        let key = format!("{:04}-{:02}", year, m);
        let (inc, exp) = data_map.get(&key).copied().unwrap_or((Decimal::ZERO, Decimal::ZERO));
        yearly_income += inc;
        yearly_expense += exp;
        months.push(json!({
            "month": key,
            "income": inc,
            "expense": exp,
            "balance": inc - exp,
        }));
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success(json!({
        "year": year,
        "months": months,
        "yearly_income": yearly_income,
        "yearly_expense": yearly_expense,
        "yearly_balance": yearly_income - yearly_expense,
    }))))
}

#[derive(Debug, Deserialize)]
pub struct DailyStatsQuery {
    pub month: String,
}

pub async fn daily_statistics(
    state: web::Data<data::AppState>,
    req: HttpRequest,
    q: web::Query<DailyStatsQuery>,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;
    let month = q.month.clone();

    validate_month(&month)?;

    let rows = data::daily_summary(&state.pool, user_id, &month).await?;

    let mut data_map: std::collections::HashMap<String, (Decimal, Decimal)> = std::collections::HashMap::new();
    for (dy, inc, exp) in &rows {
        data_map.insert(dy.clone(), (*inc, *exp));
    }

    let parts: Vec<&str> = month.split('-').collect();
    let y: i32 = parts[0].parse().unwrap_or(2024);
    let m: u32 = parts[1].parse().unwrap_or(1);
    let total_days = data::days_in_month(&month);

    let mut days: Vec<serde_json::Value> = Vec::with_capacity(total_days as usize);
    for d in 1..=total_days {
        let key = format!("{:04}-{:02}-{:02}", y, m, d);
        let (inc, exp) = data_map.get(&key).copied().unwrap_or((Decimal::ZERO, Decimal::ZERO));
        days.push(json!({
            "date": key,
            "income": inc,
            "expense": exp,
        }));
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success(json!({
        "month": month,
        "days": days,
        "total_days": total_days,
    }))))
}

#[derive(Debug, Deserialize)]
pub struct TrendStatsQuery {
    pub months: Option<i32>,
}

pub async fn trend_statistics(
    state: web::Data<data::AppState>,
    req: HttpRequest,
    q: web::Query<TrendStatsQuery>,
) -> Result<HttpResponse, AppError> {
    let user_id = uid(&req)?;
    let months = q.months.unwrap_or(6).max(1).min(24);

    let rows = data::trend_summary(&state.pool, user_id, months).await?;

    let mut data_map: std::collections::HashMap<String, (Decimal, Decimal)> = std::collections::HashMap::new();
    for (mth, inc, exp) in &rows {
        data_map.insert(mth.clone(), (*inc, *exp));
    }

    let start = data::months_ago(months);
    let parts: Vec<&str> = start.split('-').collect();
    let mut cy: i32 = parts[0].parse().unwrap_or(2024);
    let mut cm: u32 = parts[1].parse().unwrap_or(1);

    let mut data: Vec<serde_json::Value> = Vec::with_capacity(months as usize);
    for _ in 0..months {
        let key = format!("{:04}-{:02}", cy, cm);
        let (inc, exp) = data_map.get(&key).copied().unwrap_or((Decimal::ZERO, Decimal::ZERO));
        data.push(json!({
            "month": key,
            "income": inc,
            "expense": exp,
            "balance": inc - exp,
        }));
        let (ny, nm) = if cm == 12 { (cy + 1, 1) } else { (cy, cm + 1) };
        cy = ny;
        cm = nm;
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success(json!({
        "months_count": months,
        "data": data,
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

const RULES: &[(&str, &[&str])] = &[
    ("餐饮", &["午餐", "晚餐", "早餐", "外卖", "饭", "奶茶", "咖啡", "食堂"]),
    ("交通", &["地铁", "打车", "滴滴", "公交", "高铁", "机票", "出租", "加油"]),
    ("购物", &["淘宝", "京东", "购物", "衣服", "超市", "拼多多"]),
    ("学习", &["书", "课程", "学费", "资料", "考试"]),
    ("娱乐", &["电影", "游戏", "会员", "旅游", "演唱会"]),
    ("医疗", &["药", "医院", "门诊", "体检"]),
    ("住房", &["房租", "水电", "物业", "宽带"]),
];

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
        "suggestions": suggestions,
        "generated_by_ai": false,
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
    let tx_type = if body.transaction_type == "income" { "income" } else { "expense" };

    if tx_type == "expense" {
        for &(name, keywords) in RULES {
            for kw in keywords {
                if remark_lower.contains(&kw.to_lowercase()) {
                    if let Some(c) = cats.iter().find(|c| c.category_type == "expense" && c.name == name) {
                        return Ok(HttpResponse::Ok().json(ApiResponse::success(json!({
                            "category_id": c.id,
                            "category_name": c.name,
                            "confidence": 0.9,
                            "source": "rule",
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
        })))),
        None => Err(AppError::NotFound("未找到匹配的分类".into())),
    }
}
