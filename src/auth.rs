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
  permissions: i32,
  id: String,
}

async fn validate_token(token: &str) -> Result<Record, Error> {
  let client = Client::new();
  let req = client.post(format!("http://{}/api/collections/users/auth-refresh", *crate::PB_URL));
  let req = req.header(AUTHORIZATION, token);
  match req.send().await {
      Ok(mut res) => {
        let ResponseData { record } = match res.body_json().await {
          Ok(user) => user,
          Err(err) => return Err(err),
        };
        Ok(record)
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
        let record = match validate_token(token.as_str()).await {
            Ok(record) => record,
            Err(_) => return Ok(Response::new(401)),
        };
        req.insert_header("Permissions", record.permissions.to_string());
        req.insert_header("User", record.id);
        Ok(next.run(req).await)
    }
}