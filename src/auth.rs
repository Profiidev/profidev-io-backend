use serde::Deserialize;
use surf::{http::headers::AUTHORIZATION, Client, Error};
use async_trait::async_trait;
use tide::{Middleware, Next, Request, Response};

pub(crate) struct TokenAuth {}

#[derive(Deserialize)]
struct ResponseData {
  record: Record
}

#[derive(Deserialize)]
struct Record {
  permissions: i32
}

async fn validate_token(token: &str) -> Result<i32, Error> {
  let client = Client::new();
  let req = client.post("https://pocketbase.profidev.io/api/collections/users/auth-refresh");
  let req = req.header(AUTHORIZATION, token);
  match req.send().await {
      Ok(mut res) => {
        let ResponseData { record } = match res.body_json().await {
          Ok(user) => user,
          Err(err) => return Err(err),
        };
        Ok(record.permissions)
      }
      Err(err) => Err(err),
  }
}

#[async_trait]
impl Middleware<()> for TokenAuth {
    async fn handle(&self, mut req: Request<()>, next: Next<'_, ()>) -> tide::Result {
        let token = match req.header("Authorization") {
            Some(token) => token,
            None => return Ok(Response::new(401)),
        };
        let permissions = match validate_token(token.as_str()).await {
            Ok(permissions) => permissions,
            Err(_) => return Ok(Response::new(401)),
        };
        req.append_header("Permissions", permissions.to_string());
        Ok(next.run(req).await)
    }
}