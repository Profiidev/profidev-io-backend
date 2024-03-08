use serde::{de::DeserializeOwned, Deserialize, Serialize};
use surf::{Client, Result};

#[derive(Deserialize, Debug)]
struct RecordsResponse<T> {
  items: Vec<T>,
}

pub(crate) async fn get_collection_records<T>(collection: &str, filter: Option<&str>) -> Result<Vec<T>> where T: DeserializeOwned {
  let filter = match filter {
    Some(f) => format!("&filter=({})", f),
    None => "".to_string(),
  };

  let client = Client::new();
  let res = client.get(format!("https://pocketbase.profidev.io/api/collections/{}/records?perPage=100{}", collection, filter))
    .header("Authorization", format!("{}", *crate::PB_TOKEN.read().await))
    .recv_json::<RecordsResponse<T>>().await?;

  Ok(res.items)
}

pub(crate) async fn create_record<T>(collection: &str, new_record: T) -> Result<()> where T: Serialize {
  let client = Client::new();
  client.post(format!("https://pocketbase.profidev.io/api/collections/{}/records", collection))
    .header("Authorization", format!("{}", *crate::PB_TOKEN.read().await))
    .body_json(&new_record).unwrap().await?;

  Ok(())
}

pub(crate) async fn delete_record(collection: &str, delete_record_id: String) -> Result<()> {
  let client = Client::new();
  client.delete(format!("https://pocketbase.profidev.io/api/collections/{}/records/{}", collection, delete_record_id))
    .header("Authorization", format!("{}", *crate::PB_TOKEN.read().await))
    .send().await?;

  Ok(())
}

pub(crate) async fn modify_record<T>(collection: &str, modify_record: T) -> Result<()> where T: Serialize + ModifyRecord {
  let client = Client::new();
  client.patch(format!("https://pocketbase.profidev.io/api/collections/{}/records/{}", collection, modify_record.id()))
    .header("Authorization", format!("{}", *crate::PB_TOKEN.read().await))
    .body_json(&modify_record).unwrap().await?;

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

#[derive(Deserialize)]
struct Token {
  token: String,
}

#[derive(Serialize)]
struct TokenReq {
  identity: String,
  password: String,
}

pub(crate) trait ModifyRecord {
  fn id(&self) -> &String;
}