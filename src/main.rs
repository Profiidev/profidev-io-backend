use std::str::FromStr;

use async_std::sync::RwLock;
use tide::{log::LevelFilter, security::{CorsMiddleware, Origin}};

use crate::auth::TokenAuth;

mod auth;
mod metrics;
mod permissions;
mod images;
mod users;
mod iframe_urls;

lazy_static::lazy_static! {
    static ref METRICS_HOST: String = std::env::var("METRICS_HOST").unwrap_or("localhost".to_string());
    static ref NASA_API_KEY: String = std::env::var("NASA_API_KEY").unwrap_or("DEMO_KEY".to_string());

    static ref PORTAINER_URL: String = std::env::var("PORTAINER_URL").unwrap_or("".to_string());
    static ref POCKETBASE_URL: String = std::env::var("POCKETBASE_URL").unwrap_or("".to_string());

    static ref PB_TOKEN: RwLock<String> = RwLock::new("".to_string());
    static ref PB_EMAIL: String = std::env::var("PB_EMAIL").unwrap_or("".to_string());
    static ref PB_PASSWORD: String = std::env::var("PB_PASSWORD").unwrap_or("".to_string());
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    dotenv::dotenv().ok();
    
    let log_level = std::env::var("RUST_LOG_LEVEL").unwrap_or("info".to_string());
    tide::log::with_level(LevelFilter::from_str(&log_level).unwrap_or(LevelFilter::Info));

    users::get_new_token().await?;

    let cors = CorsMiddleware::new()
        .allow_origin(Origin::from("*"));
        
    let mut app = tide::new();
    app.with(cors).with(TokenAuth{});
    app.at("/metrics").post(metrics::metrics);
    app.at("/users/get").get(users::get_users);
    app.at("/users/create").post(users::create_user);
    app.at("/users/delete").post(users::delete_user);
    app.at("/users/update").post(users::update_user);
    app.at("/images/apod").get(images::apod);
    app.at("/iframe/portainer").get(iframe_urls::get_portainer_url);
    app.at("/iframe/pocketbase").get(iframe_urls::get_pocketbase_url);
    app.listen("0.0.0.0:8080").await?;
    Ok(())
}