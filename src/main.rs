use std::str::FromStr;

use tide::{log::LevelFilter, security::{CorsMiddleware, Origin}};

use crate::auth::TokenAuth;

mod auth;
mod metrics;
mod permissions;
mod images;

lazy_static::lazy_static! {
    static ref METRICS_HOST: String = std::env::var("METRICS_HOST").unwrap_or("localhost".to_string());
    static ref NASA_API_KEY: String = std::env::var("NASA_API_KEY").unwrap_or("DEMO_KEY".to_string());
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    dotenv::dotenv().ok();
    
    let log_level = std::env::var("RUST_LOG_LEVEL").unwrap_or("info".to_string());
    tide::log::with_level(LevelFilter::from_str(&log_level).unwrap_or(LevelFilter::Info));

    let cors = CorsMiddleware::new()
        .allow_origin(Origin::from("*"));
        
    let mut app = tide::new();
    app.with(cors).with(TokenAuth{});
    app.at("/metrics").post(metrics::metrics);
    app.at("/images/apod").get(images::apod);
    app.listen("0.0.0.0:8080").await?;
    Ok(())
}