use serde::{Deserialize, Serialize};
use surf::{Client, Result};
use tide::Request;

use crate::permissions::{has_permissions, Permissions};

#[derive(Deserialize)]
struct UserList {
  items: Vec<User>,
}

#[derive(Deserialize, Debug, Serialize)]
struct User {
  id: String,
  name: String,
  username: String,
  permissions: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct UserCreate {
  name: String,
  username: String,
  permissions: i32,
  password: String,
  #[serde(rename = "passwordConfirm")]
  password_confirm: String,
  verified: bool,
}

#[derive(Serialize, Deserialize)]
struct UserDelete {
  id: String,
}

#[derive(Serialize, Deserialize)]
struct UserUpdate {
  id: String,
  name: String,
  username: String,
  permissions: i32,
  password: Option<String>,
  #[serde(rename = "passwordConfirm")]
  password_confirm: Option<String>,
}

#[derive(Deserialize)]
struct Token {
  token: String,
}

#[derive(Serialize)]
struct TokenReq {
  identity: String,
  password: String,
}

pub(crate) async fn get_users(req: Request<()>) -> tide::Result {
  if !has_permissions(&req, Permissions::Users as i32) {
      return Ok(tide::Response::new(403));
    }

  let users = get_users_list_pb().await?;
  let res = tide::Response::builder(200).body(tide::Body::from_json(&users)?);
  Ok(res.build())
}

pub(crate) async fn create_user(mut req: Request<()>) -> tide::Result {
  if !has_permissions(&req, Permissions::Users as i32) {
    return Ok(tide::Response::new(403));
  }

  let new_user: UserCreate = req.body_json().await?;
  create_user_pb(new_user).await?;
  Ok(tide::Response::new(200))
}

pub(crate) async fn delete_user(mut req: Request<()>) -> tide::Result {
  if !has_permissions(&req, Permissions::Users as i32) {
    return Ok(tide::Response::new(403));
  }

  let new_user: UserDelete = req.body_json().await?;
  delete_user_pb(new_user).await?;
  Ok(tide::Response::new(200))
}

pub(crate) async fn update_user(mut req: Request<()>) -> tide::Result {
  if !has_permissions(&req, Permissions::Users as i32) {
    return Ok(tide::Response::new(403));
  }

  let new_user: UserUpdate = req.body_json().await?;
  modify_user_pb(new_user).await?;
  Ok(tide::Response::new(200))
}

async fn get_users_list_pb() -> Result<Vec<User>> {
  let client = Client::new();
  let res = client.get("https://pocketbase.profidev.io/api/collections/users/records")
    .header("Authorization", format!("{}", *crate::PB_TOKEN.read().await))
    .recv_json::<UserList>().await?;

  Ok(res.items)
}

async fn create_user_pb(new_user: UserCreate) -> Result<()> {
  let client = Client::new();
  client.post("https://pocketbase.profidev.io/api/collections/users/records")
    .header("Authorization", format!("{}", *crate::PB_TOKEN.read().await))
    .body_json(&new_user).unwrap().await?;

  Ok(())
}

async fn delete_user_pb(new_user: UserDelete) -> Result<()> {
  let client = Client::new();
  client.delete(format!("https://pocketbase.profidev.io/api/collections/users/records/{}", new_user.id))
    .header("Authorization", format!("{}", *crate::PB_TOKEN.read().await))
    .send().await?;

  Ok(())
}

async fn modify_user_pb(new_user: UserUpdate) -> Result<()> {
  let client = Client::new();
  client.patch(format!("https://pocketbase.profidev.io/api/collections/users/records/{}", new_user.id))
    .header("Authorization", format!("{}", *crate::PB_TOKEN.read().await))
    .body_json(&new_user).unwrap().await?;

  Ok(())
}

pub(crate) async fn get_new_token() -> Result<()> {
  let client = Client::new();
  
  let mut res = client.post("https://pocketbase.profidev.io/api/admins/auth-with-password")
    .body_json(&TokenReq {
      identity: (*crate::PB_EMAIL.clone()).to_string(),
      password: (*crate::PB_PASSWORD.clone()).to_string(),
    }).unwrap().await?;

  let Token { token } = res.body_json().await?;
  let mut new_token = crate::PB_TOKEN.write().await;
  *new_token = token;
  Ok(())
}