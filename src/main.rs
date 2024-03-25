use std::str::FromStr;

use async_std::sync::RwLock;
use surf::http::headers::HeaderValue;
use tide::{log::LevelFilter, security::{CorsMiddleware, Origin}};

use crate::auth::TokenAuth;

mod auth;
mod metrics;
mod permissions;
mod images;
mod users;
mod iframe_urls;
mod cloud;
mod db;

lazy_static::lazy_static! {
    static ref METRICS_HOST: String = std::env::var("METRICS_HOST").unwrap_or("localhost".to_string());
    static ref NASA_API_KEY: String = std::env::var("NASA_API_KEY").unwrap_or("DEMO_KEY".to_string());

    static ref PORTAINER_URL: String = std::env::var("PORTAINER_URL").unwrap_or("".to_string());
    static ref POCKETBASE_URL: String = std::env::var("POCKETBASE_URL").unwrap_or("".to_string());

    static ref PB_TOKEN: RwLock<String> = RwLock::new("".to_string());
    static ref PB_EMAIL: String = std::env::var("PB_EMAIL").unwrap_or("".to_string());
    static ref PB_PASSWORD: String = std::env::var("PB_PASSWORD").unwrap_or("".to_string());
    static ref PB_URL: String = std::env::var("PB_URL").unwrap_or("localhost:8090".to_string());

    static ref CLOUD_DIR: String = std::env::var("CLOUD_DIR").unwrap_or("cloud".to_string());
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    dotenv::dotenv().ok();
    
    let log_level = std::env::var("RUST_LOG_LEVEL").unwrap_or("info".to_string());
    tide::log::with_level(LevelFilter::from_str(&log_level).unwrap_or(LevelFilter::Info));

    db::get_new_token().await?;

    let cors = CorsMiddleware::new()
        .allow_origin(Origin::from("*"))
        .allow_methods("GET, POST, OPTIONS, PUT, DELETE, PATCH".parse::<HeaderValue>().unwrap());
        
    let mut app = tide::new();

    app.with(cors).with(TokenAuth{});
    app.at("/metrics").post(metrics::metrics);
    app.at("/users").get(users::get_users);
    app.at("/users").post(users::create_user);
    app.at("/users").delete(users::delete_user);
    app.at("/users").patch(users::update_user);
    app.at("/images/apod").get(images::apod);
    app.at("/iframe/portainer").get(iframe_urls::get_portainer_url);
    app.at("/iframe/pocketbase").get(iframe_urls::get_pocketbase_url);
    app.at("/cloud/access").get(cloud::get_access);
    app.at("/cloud/access").post(cloud::create_access);
    app.at("/cloud/access").delete(cloud::delete_access);
    app.at("/cloud/access").patch(cloud::update_access);
    app.at("/cloud/dirs").get(cloud::get_dir_files);
    app.at("/cloud/dirs").put(cloud::download_multiple);
    app.at("/cloud/dirs/*path").get(cloud::get_dir_files);
    app.at("/cloud/dirs/*path").post(cloud::create_dir);
    app.at("/cloud/dirs/*path").delete(cloud::delete_dir);
    app.at("/cloud/dirs/*path").patch(cloud::rename_dir);
    app.at("/cloud/dirs/*path").put(cloud::download_multiple);
    app.at("/cloud/files/*path").post(cloud::upload_file);
    app.at("/cloud/files/*path").get(cloud::download_file);
    app.at("/cloud/files/*path").delete(cloud::delete_file);
    app.at("/cloud/files/*path").patch(cloud::rename_file);
    app.at("/cloud/check/*path").get(cloud::check_if_exists);
    app.at("/cloud/check_multiple").post(cloud::check_if_exists_multiple);
    app.at("/cloud/check_multiple/*path").post(cloud::check_if_exists_multiple);

    app.listen("0.0.0.0:8080").await?;
    Ok(())
}