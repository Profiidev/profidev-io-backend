use crate::auth::TokenAuth;

mod auth;
mod metrics;
mod permissions;

#[async_std::main]
async fn main() -> tide::Result<()> {
    let mut app = tide::new();
    app.at("/metrics").with(TokenAuth{}).get(metrics::metrics);
    app.listen("0.0.0.0:8080").await?;
    Ok(())
}