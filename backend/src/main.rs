use actix_cors::Cors;
use actix_files as fs;
use actix_web::{web, App, HttpServer};
use dotenvy::dotenv;

// 新模块声明 - 注意：不使用与旧目录相同的名字
mod config;
mod data;
mod response;
mod routes;
mod middleware;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let cfg = config::Config::from_env().expect("Failed to load config");

    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&cfg.database_url)
        .await
        .expect("Failed to connect to MySQL");

    log::info!("Database connected");

    let app_state = web::Data::new(data::AppState {
        pool,
        jwt_secret: cfg.jwt_secret.clone(),
        ai_enabled: cfg.ai_enabled,
        ai_api_key: cfg.ai_api_key.clone(),
        ai_model: cfg.ai_model.clone(),
        ai_base_url: cfg.ai_base_url.clone(),
    });

    let host = cfg.server_host.clone();
    let port = cfg.server_port;
    log::info!("Starting server at http://{}:{}", host, port);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        let jwt = middleware::JwtMiddleware::new(cfg.jwt_secret.clone());

        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .wrap(cors)
            .wrap(jwt)
            .app_data(app_state.clone())
            .route("/health", web::get().to(routes::health))
            .service(
                web::scope("/api")
                    .route("/auth/register", web::post().to(routes::register))
                    .route("/auth/login", web::post().to(routes::login))
                    .route("/auth/profile", web::get().to(routes::profile))
                    .route("/categories", web::get().to(routes::list_categories))
                    .route("/transactions", web::get().to(routes::list_transactions))
                    .route("/transactions", web::post().to(routes::create_transaction))
                    .route("/transactions/{id}", web::put().to(routes::update_transaction))
                    .route("/transactions/{id}", web::delete().to(routes::delete_transaction))
                    .route("/budgets", web::get().to(routes::list_budgets))
                    .route("/budgets", web::post().to(routes::create_or_update_budget))
                    .route("/budgets/{id}", web::delete().to(routes::delete_budget))
                    .route("/statistics/monthly", web::get().to(routes::monthly_statistics))
                    .route("/statistics/yearly", web::get().to(routes::yearly_statistics))
                    .route("/statistics/daily", web::get().to(routes::daily_statistics))
                    .route("/statistics/trend", web::get().to(routes::trend_statistics))
                    .route("/ai/report", web::post().to(routes::ai_report))
                    .route("/ai/classify", web::post().to(routes::ai_classify))
            )
            .service(fs::Files::new("/", "../frontend").index_file("index.html"))
    })
    .bind((host, port))?
    .run()
    .await
}
