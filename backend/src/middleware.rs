use actix_web::{
    body::{EitherBody, MessageBody},
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::Method,
    Error, HttpMessage,
};
use chrono::{Duration, Utc};
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::future::{ready, Ready};
use std::rc::Rc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub username: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i64,
    pub username: String,
}

pub fn issue_token(secret: &str, user_id: i64, username: &str) -> Result<String, String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        exp: (now + Duration::hours(24 * 7)).timestamp(),
        iat: now.timestamp(),
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| e.to_string())
}

pub fn parse_token(secret: &str, token: &str) -> Option<Claims> {
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default())
        .map(|d| d.claims)
        .ok()
}

pub struct JwtMiddleware {
    pub secret: Rc<String>,
}

impl JwtMiddleware {
    pub fn new(secret: String) -> Self {
        JwtMiddleware { secret: Rc::new(secret) }
    }
}

impl<S, B> Transform<S, ServiceRequest> for JwtMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = JwtService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtService {
            service: Rc::new(service),
            secret: self.secret.clone(),
        }))
    }
}

pub struct JwtService<S> {
    service: Rc<S>,
    secret: Rc<String>,
}

impl<S, B> Service<ServiceRequest> for JwtService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let secret = (*self.secret).clone();

        let path = req.path().to_string();
        let is_public = req.method() == Method::OPTIONS
            || path == "/health"
            || path == "/api/auth/register"
            || path == "/api/auth/login";

        if is_public {
            let fut = service.call(req);
            return Box::pin(async move { Ok(fut.await?.map_into_left_body()) });
        }

        let token = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(|s| s.trim().to_string());

        let claims = token.and_then(|t| parse_token(&secret, &t));

        if let Some(c) = claims {
            req.extensions_mut().insert(AuthUser {
                user_id: c.sub,
                username: c.username,
            });
            let fut = service.call(req);
            Box::pin(async move { Ok(fut.await?.map_into_left_body()) })
        } else {
            Box::pin(async move {
                let (req_parts, _) = req.into_parts();
                let body = actix_web::web::Json(serde_json::json!({
                    "code": 4002,
                    "message": "未登录或 Token 已过期",
                    "data": null
                }));
                let res = actix_web::HttpResponse::Unauthorized().json(body.into_inner());
                Ok(actix_web::dev::ServiceResponse::new(req_parts, res).map_into_right_body())
            })
        }
    }
}
