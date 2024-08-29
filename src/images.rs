use serde::{Deserialize, Serialize};
use tide::Request;

#[derive(Deserialize, Serialize)]
struct APODRes {
  url: String,
  media_type: String,
}

pub(crate) async fn apod(_req: Request<()>) -> tide::Result {
  let url = format!("https://api.nasa.gov/planetary/apod?api_key={}", *crate::NASA_API_KEY);
  let mut res = surf::get(url).await?;
  let apod: APODRes = res.body_json().await?;
  let res = tide::Response::builder(200).body(tide::Body::from_json(&apod)?);
  Ok(res.build())
}

pub(crate) async fn apod_direct(req: Request<()>) -> tide::Result {
  let token = req.param("token").unwrap_or_default().to_string();

  let url = format!("https://api.nasa.gov/planetary/apod?api_key={}", token);
  let mut res = surf::get(url).await?;
  let apod: APODRes = res.body_json().await?;
  let res = tide::Response::builder(200).body(tide::Body::from_json(&apod)?);
  Ok(res.build())
}
