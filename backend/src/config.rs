use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub server_host: String,
    pub server_port: u16,
    pub ai_enabled: bool,
    pub ai_api_key: Option<String>,
    pub ai_model: String,
    pub ai_base_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let ai_api_key = env::var("AI_API_KEY")
            .ok()
            .or_else(|| env::var("DEEPSEEK_API_KEY").ok())
            .or_else(|| env::var("OPENAI_API_KEY").ok())
            .filter(|value| {
                let value = value.trim();
                !value.is_empty()
                    && value != "your-openai-api-key"
                    && value != "your-deepseek-api-key"
            });

        Ok(Config {
            database_url: env::var("DATABASE_URL")
                .map_err(|_| "DATABASE_URL must be set".to_string())?,
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "super-secret-jwt-key-change-in-production".to_string()),
            jwt_expiry_hours: env::var("JWT_EXPIRY_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24),
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            ai_enabled: env::var("AI_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            ai_api_key,
            ai_model: env::var("AI_MODEL")
                .or_else(|_| env::var("OPENAI_MODEL"))
                .unwrap_or_else(|_| "deepseek-chat".to_string()),
            ai_base_url: env::var("AI_BASE_URL")
                .unwrap_or_else(|_| "https://api.deepseek.com".to_string()),
        })
    }
}
