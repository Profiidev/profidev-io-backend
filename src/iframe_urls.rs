use tide::{convert::json, Request};

use crate::permissions::{has_permissions, Permissions};

pub(crate) async fn get_portainer_url(req: Request<()>) -> tide::Result {
  if !has_permissions(&req, Permissions::Portainer as i32) {
    return Ok(tide::Response::new(403));
  }
  
  return_url(&crate::PORTAINER_URL)
}

pub(crate) async fn get_pocketbase_url(req: Request<()>) -> tide::Result {
  if !has_permissions(&req, Permissions::Database as i32) {
    return Ok(tide::Response::new(403));
  }
  
  return_url(&crate::POCKETBASE_URL)
}

fn return_url(url: &str) -> tide::Result {
  let body = json!({
    "url": url
  });
  let res = tide::Response::builder(200).body(tide::Body::from_json(&body)?);
  Ok(res.build())
}