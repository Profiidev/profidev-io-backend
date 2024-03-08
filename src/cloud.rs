use std::io::{Read, Write};

use async_std::io::ReadExt;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use tide::Request;

use crate::{db::{create_record, delete_record, get_collection_records, modify_record, ModifyRecord}, permissions::{has_permissions, is_admin, Permissions}};

pub(crate) async fn get_access(req: Request<()>) -> tide::Result {
  if !has_permissions(&req, Permissions::CloudManage as i32) {
      return Ok(tide::Response::new(403));
    }

  let access = get_collection_records::<Access>("cloud", None).await?;
  let res = tide::Response::builder(200).body(tide::Body::from_json(&access)?);
  Ok(res.build())
}

pub(crate) async fn create_access(mut req: Request<()>) -> tide::Result {
  if !has_permissions(&req, Permissions::CloudManage as i32) {
    return Ok(tide::Response::new(403));
  }

  let new_access: AccessCreate = req.body_json().await?;
  create_record("cloud", new_access).await?;
  Ok(tide::Response::new(200))
}

pub(crate) async fn delete_access(mut req: Request<()>) -> tide::Result {
  if !has_permissions(&req, Permissions::CloudManage as i32) {
    return Ok(tide::Response::new(403));
  }

  let delete_access: AccessDelete = req.body_json().await?;
  delete_record("cloud", delete_access.id).await?;
  Ok(tide::Response::new(200))
}

pub(crate) async fn update_access(mut req: Request<()>) -> tide::Result {
  if !has_permissions(&req, Permissions::CloudManage as i32) {
    return Ok(tide::Response::new(403));
  }

  let modify_access: AccessUpdate = req.body_json().await?;
  modify_record("cloud", modify_access).await?;
  Ok(tide::Response::new(200))
}

pub(crate) async fn get_dir_files(req: Request<()>) -> tide::Result {
  let (dir, _) = match check_permissions(&req, true).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };
  
  let files = match std::fs::read_dir(format!("{}/{}", *crate::CLOUD_DIR, dir)) {
    Ok(f) => f.filter_map(|f| f.ok()).map(|f| CloudFile{name: f.file_name().to_string_lossy().to_string(), dir: f.file_type().unwrap().is_dir()}).collect(),
    Err(_) => return Ok(tide::Response::new(404)),
  };

  Ok(tide::Response::builder(200).body(tide::Body::from_json(&CloudFiles{files})?).build())
}

pub(crate) async fn upload_file(mut req: Request<()>) -> tide::Result {
  let (path, dir) = match check_permissions(&req, false).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  let mut file = req.take_body();
  let mut data = Vec::new();
  file.read_to_end(&mut data).await?;

  let mut encoder = GzEncoder::new(Vec::new(), Compression::new(4));
  encoder.write_all(&data).unwrap();
  let comp = encoder.finish().unwrap();

  async_std::fs::create_dir_all(format!("{}/{}", *crate::CLOUD_DIR, dir)).await?;
  async_std::fs::write(format!("{}/{}", *crate::CLOUD_DIR, path), comp).await?;

  Ok(tide::Response::new(200))
}

pub(crate) async fn download_file(req: Request<()>) -> tide::Result {
  let (path, _) = match check_permissions(&req, false).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  let data = match async_std::fs::read(format!("{}/{}", *crate::CLOUD_DIR, path)).await {
    Ok(d) => d,
    Err(_) => return Ok(tide::Response::new(404)),
  };
  let mut decoder = GzDecoder::new(&data[..]);
  let mut decomp = Vec::new();
  decoder.read_to_end(&mut decomp).unwrap();  

  Ok(tide::Response::builder(200).body(decomp).build())
}

pub(crate) async fn check_if_exists(req: Request<()>) -> tide::Result {
  let (path, _) = match check_permissions(&req, false).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  let exists = async_std::fs::metadata(format!("{}/{}", *crate::CLOUD_DIR, path)).await.is_ok();
  Ok(tide::Response::builder(200).body(tide::Body::from_json(&exists)?).build())
}

pub(crate) async fn create_dir(req: Request<()>) -> tide::Result {
  let (path, _) = match check_permissions(&req, false).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  async_std::fs::create_dir_all(format!("{}/{}", *crate::CLOUD_DIR, path)).await?;
  Ok(tide::Response::new(200))
}

pub(crate) async fn delete_file(req: Request<()>) -> tide::Result {
  let (path, _) = match check_permissions(&req, false).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  async_std::fs::remove_file(format!("{}/{}", *crate::CLOUD_DIR, path)).await?;
  Ok(tide::Response::new(200))
}

pub(crate) async fn delete_dir(req: Request<()>) -> tide::Result {
  let (path, _) = match check_permissions(&req, true).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  async_std::fs::remove_dir_all(format!("{}/{}", *crate::CLOUD_DIR, path)).await?;
  Ok(tide::Response::new(200))
}

async fn check_permissions(req: &Request<()>, is_dir: bool) -> Result<(String, String), tide::Response> {
  if !has_permissions(&req, Permissions::Cloud as i32) {
    return Err(tide::Response::new(403));
  }

  let path = req.param("path").unwrap_or_default().to_string();
  let dir = if is_dir {
    path.clone()
  } else {
    path.split('/').take(path.split('/').count() - 1).collect::<Vec<&str>>().join("/")
  };

  if !check_access(&req, &dir, true).await && !is_admin(&req) {
    return Err(tide::Response::new(403));
  }

  Ok((path, dir))
}

async fn check_access(req: &Request<()>, dir: &str, write: bool) -> bool {
  let user = req.header("User").unwrap().as_str();
  let access = get_collection_records::<Access>("cloud", Some(&format!("user='{}'", user))).await.unwrap();
  access.iter().filter(|&a| !write || a.write == write).any(|a| dir.starts_with(&a.dir))
}

#[derive(Serialize, Deserialize)]
struct Access {
  id: String,
  user: String,
  dir: String,
  write: bool,
}

#[derive(Serialize, Deserialize)]
struct AccessCreate {
  user: String,
  dir: String,
  write: bool,
}

#[derive(Serialize, Deserialize)]
struct AccessDelete {
  id: String,
}

#[derive(Serialize, Deserialize)]
struct AccessUpdate {
  id: String,
  user: String,
  dir: String,
  write: bool,
}

#[derive(Serialize)]
struct CloudFiles {
  files: Vec<CloudFile>,
}

#[derive(Serialize)]
struct CloudFile {
  name: String,
  dir: bool,
}

impl ModifyRecord for AccessUpdate {
  fn id(&self) -> &String {
    &self.id
  } 
}