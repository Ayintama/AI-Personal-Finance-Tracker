use chrono::{Datelike, Local, NaiveDateTime};
use rust_decimal::Decimal;
use sqlx::{mysql::MySqlPool, FromRow};

#[derive(Clone)]
pub struct AppState {
    pub pool: MySqlPool,
    pub jwt_secret: String,
}

// ==================== User ====================
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub email: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub async fn get_user_by_username(pool: &MySqlPool, username: &str) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
}

pub async fn get_user_by_id(pool: &MySqlPool, id: i64) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn create_user(
    pool: &MySqlPool,
    username: &str,
    password_hash: &str,
    email: Option<&str>,
) -> sqlx::Result<i64> {
    let result = sqlx::query("INSERT INTO users (username, password_hash, email) VALUES (?, ?, ?)")
        .bind(username)
        .bind(password_hash)
        .bind(email)
        .execute(pool)
        .await?;
    Ok(result.last_insert_id() as i64)
}

// ==================== Category ====================
#[derive(Debug, Clone, FromRow)]
pub struct Category {
    pub id: i64,
    pub user_id: Option<i64>,
    pub name: String,
    #[sqlx(rename = "category_type")]
    pub category_type: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub async fn list_categories(pool: &MySqlPool, user_id: i64) -> sqlx::Result<Vec<Category>> {
    sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE user_id IS NULL OR user_id = ? ORDER BY user_id, name")
        .bind(user_id)
        .fetch_all(pool)
        .await
}

pub async fn get_category(pool: &MySqlPool, id: i64) -> sqlx::Result<Option<Category>> {
    sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

// ==================== Transaction ====================
#[derive(Debug, Clone, FromRow)]
pub struct TransactionWithCategory {
    pub id: i64,
    pub user_id: i64,
    pub category_id: i64,
    #[sqlx(rename = "category_name")]
    pub category_name: String,
    pub amount: Decimal,
    #[sqlx(rename = "transaction_type")]
    pub transaction_type: String,
    pub occurred_at: NaiveDateTime,
    pub remark: Option<String>,
}

pub async fn list_transactions(
    pool: &MySqlPool,
    user_id: i64,
    month: Option<&str>,
    tx_type: Option<&str>,
    page: i64,
    page_size: i64,
) -> sqlx::Result<(Vec<TransactionWithCategory>, i64)> {
    let offset = (page - 1) * page_size;

    let mut sql_count = String::from("SELECT COUNT(*) AS total FROM transactions WHERE user_id = ?");
    if month.is_some() { sql_count.push_str(" AND DATE_FORMAT(occurred_at, '%Y-%m') = ?"); }
    if tx_type.is_some() { sql_count.push_str(" AND transaction_type = ?"); }

    let mut q = sqlx::query_scalar::<_, i64>(&sql_count).bind(user_id);
    if let Some(m) = month { q = q.bind(m); }
    if let Some(t) = tx_type { q = q.bind(t); }
    let total = q.fetch_one(pool).await?;

    let mut sql = String::from(
        "SELECT t.id, t.user_id, t.category_id, c.name AS category_name, t.amount, t.transaction_type, t.occurred_at, t.remark \
         FROM transactions t JOIN categories c ON t.category_id = c.id WHERE t.user_id = ?"
    );
    if month.is_some() { sql.push_str(" AND DATE_FORMAT(t.occurred_at, '%Y-%m') = ?"); }
    if tx_type.is_some() { sql.push_str(" AND t.transaction_type = ?"); }
    sql.push_str(" ORDER BY t.occurred_at DESC LIMIT ? OFFSET ?");

    let mut q = sqlx::query_as::<_, TransactionWithCategory>(&sql).bind(user_id);
    if let Some(m) = month { q = q.bind(m); }
    if let Some(t) = tx_type { q = q.bind(t); }
    let items = q.bind(page_size).bind(offset).fetch_all(pool).await?;
    Ok((items, total))
}

pub async fn create_transaction(
    pool: &MySqlPool,
    user_id: i64,
    category_id: i64,
    amount: Decimal,
    transaction_type: &str,
    occurred_at: NaiveDateTime,
    remark: Option<&str>,
) -> sqlx::Result<i64> {
    let result = sqlx::query(
        "INSERT INTO transactions (user_id, category_id, amount, transaction_type, occurred_at, remark) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(user_id)
    .bind(category_id)
    .bind(amount)
    .bind(transaction_type)
    .bind(occurred_at)
    .bind(remark)
    .execute(pool)
    .await?;
    Ok(result.last_insert_id() as i64)
}

pub async fn update_transaction(
    pool: &MySqlPool,
    id: i64,
    category_id: i64,
    amount: Decimal,
    remark: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE transactions SET category_id = ?, amount = ?, remark = ? WHERE id = ?")
        .bind(category_id)
        .bind(amount)
        .bind(remark)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_transaction(pool: &MySqlPool, id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM transactions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_transaction_owner(pool: &MySqlPool, id: i64) -> sqlx::Result<Option<i64>> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT user_id FROM transactions WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}

// ==================== Budget ====================
#[derive(Debug, Clone, FromRow)]
pub struct BudgetRowRaw {
    pub id: i64,
    pub user_id: i64,
    pub category_id: Option<i64>,
    pub amount: Decimal,
    pub month: String,
}

#[derive(Debug, Clone)]
pub struct BudgetRow {
    pub id: i64,
    pub user_id: i64,
    pub category_id: Option<i64>,
    pub category_name: String,
    pub amount: Decimal,
    pub month: String,
    pub spent: Decimal,
}

pub fn current_month() -> String {
    let now = Local::now();
    format!("{:04}-{:02}", now.year(), now.month())
}

pub async fn list_budgets(pool: &MySqlPool, user_id: i64, month: &str) -> sqlx::Result<Vec<BudgetRow>> {
    let raws = sqlx::query_as::<_, BudgetRowRaw>("SELECT id, user_id, category_id, amount, month FROM budgets WHERE user_id = ? AND month = ?")
        .bind(user_id)
        .bind(month)
        .fetch_all(pool)
        .await?;

    let mut out = Vec::with_capacity(raws.len());
    for r in raws {
        let spent = get_budget_spent(pool, user_id, r.category_id, month).await?;
        let category_name = match r.category_id {
            Some(cid) => get_category(pool, cid).await?.map(|c| c.name).unwrap_or_default(),
            None => String::new(),
        };
        out.push(BudgetRow {
            id: r.id,
            user_id: r.user_id,
            category_id: r.category_id,
            category_name,
            amount: r.amount,
            month: r.month,
            spent,
        });
    }
    Ok(out)
}

pub async fn get_budget_spent(
    pool: &MySqlPool,
    user_id: i64,
    category_id: Option<i64>,
    month: &str,
) -> sqlx::Result<Decimal> {
    let start = format!("{}-01 00:00:00", month);
    let next_month_end = if month.len() == 7 {
        let parts: Vec<&str> = month.split('-').collect();
        let y: i32 = parts[0].parse().unwrap_or(2024);
        let m: u32 = parts[1].parse().unwrap_or(1);
        let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
        format!("{:04}-{:02}-01 00:00:00", ny, nm)
    } else {
        start.clone()
    };

    let total = if let Some(cid) = category_id {
        sqlx::query_scalar::<_, Decimal>(
            "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE user_id = ? AND category_id = ? AND transaction_type = 'expense' AND occurred_at >= ? AND occurred_at < ?"
        )
        .bind(user_id)
        .bind(cid)
        .bind(&start)
        .bind(&next_month_end)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar::<_, Decimal>(
            "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE user_id = ? AND transaction_type = 'expense' AND occurred_at >= ? AND occurred_at < ?"
        )
        .bind(user_id)
        .bind(&start)
        .bind(&next_month_end)
        .fetch_one(pool)
        .await?
    };
    Ok(total)
}

pub async fn upsert_budget(
    pool: &MySqlPool,
    user_id: i64,
    category_id: Option<i64>,
    amount: Decimal,
    month: &str,
) -> sqlx::Result<i64> {
    let existing: Option<i64> = match category_id {
        Some(cid) => {
            sqlx::query_scalar("SELECT id FROM budgets WHERE user_id = ? AND category_id = ? AND month = ?")
                .bind(user_id)
                .bind(cid)
                .bind(month)
                .fetch_optional(pool)
                .await?
        }
        None => {
            sqlx::query_scalar("SELECT id FROM budgets WHERE user_id = ? AND category_id IS NULL AND month = ?")
                .bind(user_id)
                .bind(month)
                .fetch_optional(pool)
                .await?
        }
    };

    if let Some(id) = existing {
        sqlx::query("UPDATE budgets SET amount = ? WHERE id = ?")
            .bind(amount)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(id)
    } else {
        let result = sqlx::query("INSERT INTO budgets (user_id, category_id, amount, month) VALUES (?, ?, ?, ?)")
            .bind(user_id)
            .bind(category_id)
            .bind(amount)
            .bind(month)
            .execute(pool)
            .await?;
        Ok(result.last_insert_id() as i64)
    }
}

pub async fn delete_budget(pool: &MySqlPool, id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM budgets WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_budget_owner(pool: &MySqlPool, id: i64) -> sqlx::Result<Option<i64>> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT user_id FROM budgets WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|r| r.0))
}

pub async fn total_budget(pool: &MySqlPool, user_id: i64, month: &str) -> sqlx::Result<Option<Decimal>> {
    let result: Option<Decimal> = sqlx::query_scalar(
        "SELECT amount FROM budgets WHERE user_id = ? AND category_id IS NULL AND month = ?"
    )
    .bind(user_id)
    .bind(month)
    .fetch_optional(pool)
    .await?;
    Ok(result)
}

// ==================== Statistics ====================

/// 根据 "YYYY-MM" 计算该月的起止 DATETIME 字符串
pub fn month_range(month: &str) -> (String, String) {
    let start = format!("{}-01 00:00:00", month);
    let parts: Vec<&str> = month.split('-').collect();
    let y: i32 = parts[0].parse().unwrap_or(2024);
    let m: u32 = parts[1].parse().unwrap_or(1);
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    let end = format!("{:04}-{:02}-01 00:00:00", ny, nm);
    (start, end)
}

/// 当前月份往前推 N 个月，返回起始月份 "YYYY-MM"
pub fn months_ago(n: i32) -> String {
    let now = Local::now();
    let total = now.year() * 12 + (now.month() as i32) - 1 - n + 1;
    let y = (total - 1) / 12;
    let m = ((total - 1) % 12) + 1;
    format!("{:04}-{:02}", y, m)
}

/// 计算某月天数
pub fn days_in_month(month: &str) -> u32 {
    let parts: Vec<&str> = month.split('-').collect();
    let y: i32 = parts[0].parse().unwrap_or(2024);
    let m: u32 = parts[1].parse().unwrap_or(1);
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    if let Some(next_first) = chrono::NaiveDate::from_ymd_opt(ny, nm, 1) {
        if let Some(last_day) = next_first.pred_opt() {
            return last_day.day();
        }
    }
    30
}

pub async fn month_summary(
    pool: &MySqlPool,
    user_id: i64,
    month: &str,
) -> sqlx::Result<(Decimal, Decimal)> {
    let start = format!("{}-01 00:00:00", month);
    let next_month_end = if month.len() == 7 {
        let parts: Vec<&str> = month.split('-').collect();
        let y: i32 = parts[0].parse().unwrap_or(2024);
        let m: u32 = parts[1].parse().unwrap_or(1);
        let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
        format!("{:04}-{:02}-01 00:00:00", ny, nm)
    } else {
        start.clone()
    };

    let income: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE user_id = ? AND transaction_type = 'income' AND occurred_at >= ? AND occurred_at < ?"
    )
    .bind(user_id)
    .bind(&start)
    .bind(&next_month_end)
    .fetch_one(pool)
    .await?;

    let expense: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE user_id = ? AND transaction_type = 'expense' AND occurred_at >= ? AND occurred_at < ?"
    )
    .bind(user_id)
    .bind(&start)
    .bind(&next_month_end)
    .fetch_one(pool)
    .await?;

    Ok((income, expense))
}

pub async fn category_expense_summary(
    pool: &MySqlPool,
    user_id: i64,
    month: &str,
) -> sqlx::Result<Vec<(String, Decimal)>> {
    let start = format!("{}-01 00:00:00", month);
    let next_month_end = if month.len() == 7 {
        let parts: Vec<&str> = month.split('-').collect();
        let y: i32 = parts[0].parse().unwrap_or(2024);
        let m: u32 = parts[1].parse().unwrap_or(1);
        let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
        format!("{:04}-{:02}-01 00:00:00", ny, nm)
    } else {
        start.clone()
    };

    let rows: Vec<(String, Decimal)> = sqlx::query_as(
        "SELECT c.name, COALESCE(SUM(t.amount), 0) AS amount
         FROM categories c
         LEFT JOIN transactions t ON t.category_id = c.id
             AND t.user_id = ? AND t.transaction_type = 'expense'
             AND t.occurred_at >= ? AND t.occurred_at < ?
         WHERE c.category_type = 'expense' AND (c.user_id IS NULL OR c.user_id = ?)
         GROUP BY c.id, c.name
         HAVING amount > 0
         ORDER BY amount DESC"
    )
    .bind(user_id)
    .bind(&start)
    .bind(&next_month_end)
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn category_income_summary(
    pool: &MySqlPool,
    user_id: i64,
    month: &str,
) -> sqlx::Result<Vec<(String, Decimal)>> {
    let (start, end) = month_range(month);
    let rows: Vec<(String, Decimal)> = sqlx::query_as(
        "SELECT c.name, COALESCE(SUM(t.amount), 0) AS amount
         FROM categories c
         LEFT JOIN transactions t ON t.category_id = c.id
             AND t.user_id = ? AND t.transaction_type = 'income'
             AND t.occurred_at >= ? AND t.occurred_at < ?
         WHERE c.category_type = 'income' AND (c.user_id IS NULL OR c.user_id = ?)
         GROUP BY c.id, c.name
         HAVING amount > 0
         ORDER BY amount DESC"
    )
    .bind(user_id)
    .bind(&start)
    .bind(&end)
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn monthly_transaction_count(
    pool: &MySqlPool,
    user_id: i64,
    month: &str,
) -> sqlx::Result<i64> {
    let (start, end) = month_range(month);
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM transactions WHERE user_id = ? AND occurred_at >= ? AND occurred_at < ?"
    )
    .bind(user_id)
    .bind(&start)
    .bind(&end)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

pub async fn max_single_expense(
    pool: &MySqlPool,
    user_id: i64,
    month: &str,
) -> sqlx::Result<Option<Decimal>> {
    let (start, end) = month_range(month);
    let val = sqlx::query_scalar::<_, Decimal>(
        "SELECT MAX(amount) FROM transactions WHERE user_id = ? AND transaction_type = 'expense' AND occurred_at >= ? AND occurred_at < ?"
    )
    .bind(user_id)
    .bind(&start)
    .bind(&end)
    .fetch_optional(pool)
    .await?;
    Ok(val)
}

pub async fn yearly_summary(
    pool: &MySqlPool,
    user_id: i64,
    year: i32,
) -> sqlx::Result<Vec<(String, Decimal, Decimal)>> {
    let start = format!("{:04}-01-01 00:00:00", year);
    let end = format!("{:04}-01-01 00:00:00", year + 1);
    let rows: Vec<(String, Decimal, Decimal)> = sqlx::query_as(
        "SELECT DATE_FORMAT(occurred_at, '%Y-%m') AS mth,
                COALESCE(SUM(CASE WHEN transaction_type = 'income' THEN amount ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN transaction_type = 'expense' THEN amount ELSE 0 END), 0)
         FROM transactions
         WHERE user_id = ? AND occurred_at >= ? AND occurred_at < ?
         GROUP BY DATE_FORMAT(occurred_at, '%Y-%m')
         ORDER BY mth"
    )
    .bind(user_id)
    .bind(&start)
    .bind(&end)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn daily_summary(
    pool: &MySqlPool,
    user_id: i64,
    month: &str,
) -> sqlx::Result<Vec<(String, Decimal, Decimal)>> {
    let (start, end) = month_range(month);
    let rows: Vec<(String, Decimal, Decimal)> = sqlx::query_as(
        "SELECT DATE_FORMAT(occurred_at, '%Y-%m-%d') AS dy,
                COALESCE(SUM(CASE WHEN transaction_type = 'income' THEN amount ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN transaction_type = 'expense' THEN amount ELSE 0 END), 0)
         FROM transactions
         WHERE user_id = ? AND occurred_at >= ? AND occurred_at < ?
         GROUP BY DATE_FORMAT(occurred_at, '%Y-%m-%d')
         ORDER BY dy"
    )
    .bind(user_id)
    .bind(&start)
    .bind(&end)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn trend_summary(
    pool: &MySqlPool,
    user_id: i64,
    months: i32,
) -> sqlx::Result<Vec<(String, Decimal, Decimal)>> {
    let start = months_ago(months);
    let now = Local::now();
    let end = format!("{:04}-{:02}-01 00:00:00", now.year(), if now.month() == 12 { 1 } else { now.month() + 1 });
    let rows: Vec<(String, Decimal, Decimal)> = sqlx::query_as(
        "SELECT DATE_FORMAT(occurred_at, '%Y-%m') AS mth,
                COALESCE(SUM(CASE WHEN transaction_type = 'income' THEN amount ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN transaction_type = 'expense' THEN amount ELSE 0 END), 0)
         FROM transactions
         WHERE user_id = ? AND occurred_at >= ? AND occurred_at < ?
         GROUP BY DATE_FORMAT(occurred_at, '%Y-%m')
         ORDER BY mth"
    )
    .bind(user_id)
    .bind(&start)
    .bind(&end)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
