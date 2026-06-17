use actix_web::{HttpMessage, HttpRequest, HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        ApiResponse {
            code: 0,
            message: "success".to_string(),
            data: Some(data),
        }
    }

    pub fn ok_empty() -> Self {
        ApiResponse {
            code: 0,
            message: "success".to_string(),
            data: None,
        }
    }

    pub fn error(code: i32, msg: &str) -> Self {
        ApiResponse {
            code,
            message: msg.to_string(),
            data: None,
        }
    }
}

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    InternalError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::BadRequest(s) => write!(f, "{}", s),
            AppError::Unauthorized(s) => write!(f, "{}", s),
            AppError::Forbidden(s) => write!(f, "{}", s),
            AppError::NotFound(s) => write!(f, "{}", s),
            AppError::Conflict(s) => write!(f, "{}", s),
            AppError::InternalError(s) => write!(f, "{}", s),
        }
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let (status, code) = match self {
            AppError::BadRequest(_) => (400, 4001),
            AppError::Unauthorized(_) => (401, 4002),
            AppError::Forbidden(_) => (403, 4003),
            AppError::NotFound(_) => (404, 4004),
            AppError::Conflict(_) => (409, 4005),
            AppError::InternalError(_) => (500, 5000),
        };
        let msg = match self {
            AppError::BadRequest(s)
            | AppError::Unauthorized(s)
            | AppError::Forbidden(s)
            | AppError::NotFound(s)
            | AppError::Conflict(s)
            | AppError::InternalError(s) => s.clone(),
        };
        HttpResponse::build(actix_web::http::StatusCode::from_u16(status).unwrap())
            .json(ApiResponse::<()>::error(code, &msg))
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        log::error!("Database error: {:?}", err);
        AppError::InternalError("数据库错误".into())
    }
}

impl From<bcrypt::BcryptError> for AppError {
    fn from(err: bcrypt::BcryptError) -> Self {
        log::error!("Bcrypt error: {:?}", err);
        AppError::InternalError("内部错误".into())
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        AppError::Unauthorized(format!("Token 无效: {}", err))
    }
}

pub fn get_user_id(req: &HttpRequest) -> Result<i64, AppError> {
    req.extensions()
        .get::<crate::middleware::AuthUser>()
        .map(|u| u.user_id)
        .ok_or_else(|| AppError::Unauthorized("未登录或 Token 已过期".to_string()))
}
