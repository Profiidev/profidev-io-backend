use serde::Deserialize;
use tide::Request;

#[derive(Deserialize)]
struct APODRes {
  url: String,
}

pub(crate) async fn apod(_req: Request<()>) -> tide::Result {
  let url = format!("https://api.nasa.gov/planetary/apod?api_key={}", *crate::NASA_API_KEY);
  let mut res = surf::get(url).await?;
  let apod: APODRes = res.body_json().await?;
  Ok(apod.url.into())
}