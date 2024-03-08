use serde::{Deserialize, Serialize};
use tide::Request;

use crate::{db::{create_record, delete_record, get_collection_records, modify_record, ModifyRecord}, permissions::{has_permissions, Permissions}};

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

impl ModifyRecord for UserUpdate {
  fn id(&self) -> &String {
    &self.id
  } 
}

pub(crate) async fn get_users(req: Request<()>) -> tide::Result {
  if !has_permissions(&req, Permissions::Users as i32) {
      return Ok(tide::Response::new(403));
    }

  let users = get_collection_records::<User>("users", None).await?;
  let res = tide::Response::builder(200).body(tide::Body::from_json(&users)?);
  Ok(res.build())
}

pub(crate) async fn create_user(mut req: Request<()>) -> tide::Result {
  if !has_permissions(&req, Permissions::Users as i32) {
    return Ok(tide::Response::new(403));
  }

  let new_user: UserCreate = req.body_json().await?;
  create_record("users", new_user).await?;
  Ok(tide::Response::new(200))
}

pub(crate) async fn delete_user(mut req: Request<()>) -> tide::Result {
  if !has_permissions(&req, Permissions::Users as i32) {
    return Ok(tide::Response::new(403));
  }

  let delete_user: UserDelete = req.body_json().await?;
  delete_record("users", delete_user.id).await?;
  Ok(tide::Response::new(200))
}

pub(crate) async fn update_user(mut req: Request<()>) -> tide::Result {
  if !has_permissions(&req, Permissions::Users as i32) {
    return Ok(tide::Response::new(403));
  }

  let modify_user: UserUpdate = req.body_json().await?;
  modify_record("users", modify_user).await?;
  Ok(tide::Response::new(200))
}