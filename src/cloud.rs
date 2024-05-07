use std::{fs::File, io::{Cursor, Error, Read, Write}};

use async_std::io::ReadExt;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use tide::Request;
use zip::ZipWriter;

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
  if !has_permissions(&req, Permissions::Cloud as i32) {
    return Ok(tide::Response::new(403));
  }
  
  let dir = percent_decode_str(req.param("path").unwrap_or_default()).decode_utf8_lossy().to_string();
  let files: Vec<CloudFileTemp> = match std::fs::read_dir(format!("{}/{}", *crate::CLOUD_DIR, dir)) {
    Ok(f) => f.filter_map(|f| f.ok()).map(|f| CloudFileTemp{name: f.file_name().to_string_lossy().to_string(), dir: f.file_type().unwrap().is_dir()}).collect(),
    Err(_) => return Ok(tide::Response::new(410)),
  };
  
  let final_files = check_files_access(&req, files, dir).await;

  Ok(tide::Response::builder(200).body(tide::Body::from_json(&CloudFiles{files: final_files})?).build())
}

pub(crate) async fn upload_file(mut req: Request<()>) -> tide::Result {
  let (path, dir) = match check_permissions(&req, false, true).await {
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
  let (path, _) = match check_permissions(&req, false, false).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  let data = match async_std::fs::read(format!("{}/{}", *crate::CLOUD_DIR, path)).await {
    Ok(d) => d,
    Err(_) => return Ok(tide::Response::new(410)),
  };

  let mut decoder = GzDecoder::new(&data[..]);
  let mut decomp = Vec::new();
  decoder.read_to_end(&mut decomp).unwrap();

  Ok(tide::Response::builder(200).body(decomp).build())
}

pub(crate) async fn download_multiple(mut req: Request<()>) -> tide::Result {
  let (path, _) = match check_permissions(&req, false, false).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  let files: Vec<String> = req.body_json().await?;
  let comp = pack_zip(&path, files)?;

  Ok(tide::Response::builder(200).body(comp).header("Content-Type", "application/zip").header("Content-Disposition", "attachment; filename=files.zip").build())

}

pub(crate) async fn check_if_exists(req: Request<()>) -> tide::Result {
  let (path, _) = match check_permissions(&req, false, false).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  let exists = async_std::fs::metadata(format!("{}/{}", *crate::CLOUD_DIR, path)).await.is_ok();
  Ok(tide::Response::builder(200).body(tide::Body::from_json(&exists)?).build())
}

pub(crate) async fn check_if_exists_multiple(mut req: Request<()>) -> tide::Result {
  let (_, dir) = match check_permissions(&req, true, false).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  let files: Vec<String> = req.body_json().await?;
  let temp = files.iter().map(|f| CloudFileTemp{name: f.clone(), dir: false}).collect();
  let cloud = check_files_access(&req, temp, dir.clone()).await;

  let parrent_dir = if dir.is_empty() {
    crate::CLOUD_DIR.clone()
  } else {
    format!("{}/{}", *crate::CLOUD_DIR, dir)
  };
  let mut exists = Vec::new();
  for file in cloud {
    exists.push(async_std::fs::metadata(format!("{}/{}", parrent_dir, file.name)).await.is_ok());
  }
  exists.retain(|&e| e);

  Ok(tide::Response::builder(200).body(tide::Body::from_json(&Exists{ count: exists.len() as i32 })?).build())
}

pub(crate) async fn create_dir(req: Request<()>) -> tide::Result {
  let (path, _) = match check_permissions(&req, false, true).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  async_std::fs::create_dir_all(format!("{}/{}", *crate::CLOUD_DIR, path)).await?;
  Ok(tide::Response::new(200))
}

pub(crate) async fn delete_file(req: Request<()>) -> tide::Result {
  let (path, _) = match check_permissions(&req, false, true).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  async_std::fs::remove_file(format!("{}/{}", *crate::CLOUD_DIR, path)).await?;
  Ok(tide::Response::new(200))
}

pub(crate) async fn delete_dir(req: Request<()>) -> tide::Result {
  let (path, _) = match check_permissions(&req, true, true).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  async_std::fs::remove_dir_all(format!("{}/{}", *crate::CLOUD_DIR, path)).await?;
  Ok(tide::Response::new(200))
}

pub(crate) async fn rename_file(req: Request<()>) -> tide::Result {
  let (path, _) = match check_permissions(&req, false, true).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  let new_name = req.param("new_name").unwrap_or_default().to_string();
  let new_path = format!("{}/{}", path.split('/').take(path.split('/').count() - 1).collect::<Vec<&str>>().join("/"), new_name);
  async_std::fs::rename(format!("{}/{}", *crate::CLOUD_DIR, path), format!("{}/{}", *crate::CLOUD_DIR, new_path)).await?;
  Ok(tide::Response::new(200))
}

pub(crate) async fn rename_dir(req: Request<()>) -> tide::Result {
  let (path, _) = match check_permissions(&req, true, true).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  let new_name = req.param("new_name").unwrap_or_default().to_string();
  let new_path = format!("{}/{}", path.split('/').take(path.split('/').count() - 1).collect::<Vec<&str>>().join("/"), new_name);
  async_std::fs::rename(format!("{}/{}", *crate::CLOUD_DIR, path), format!("{}/{}", *crate::CLOUD_DIR, new_path)).await?;
  Ok(tide::Response::new(200))
}

pub(crate) async fn create_direct_link(req: Request<()>) -> tide::Result {
  let (path, _) = match check_permissions(&req, false, true).await {
    Ok(p) => p,
    Err(r) => return Ok(r),
  };

  let direct_link = get_collection_records::<DirectLink>("direct_cloud", Some(&format!("path='{}'", path))).await?;
  if !direct_link.is_empty() {
    let link = format!("{}/{}", *crate::CLOUD_URL, direct_link[0].uuid);
    return Ok(tide::Response::builder(200).body(tide::Body::from_json(&link)?).build());
  }
  
  let random = rand::random::<u128>();
  let link = format!("{}/{}", *crate::CLOUD_URL, random);
  let direct_link = DirectLink{uuid: random.to_string(), path};

  create_record("direct_cloud", direct_link).await?;

  Ok(tide::Response::builder(200).body(tide::Body::from_json(&link)?).build())
}

pub(crate) async fn get_direct_link(req: Request<()>) -> tide::Result {
  let uuid = req.param("uuid").unwrap_or_default().parse::<u128>().unwrap();
  let direct_link = get_collection_records::<DirectLink>("direct_cloud", Some(&format!("uuid=\"{}\"", uuid))).await?;
  if direct_link.is_empty() {
    return Ok(tide::Response::new(404));
  }

  let path = format!("{}/{}", *crate::CLOUD_DIR, direct_link[0].path);
  let mut file_name = path.clone().split('/').last().unwrap().to_string();
  let mut file = File::open(&path)?;
  let decomp = if file.metadata()?.is_dir() {
    let dir = std::fs::read_dir(path)?;
    let files: Vec<String> = dir.filter_map(|f| f.ok()).map(|f| f.file_name().to_string_lossy().to_string()).collect();
    file_name = format!("{}.zip", file_name);
    pack_zip(&direct_link[0].path, files)?
  } else {
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    let mut decoder = GzDecoder::new(&data[..]);
    let mut decomp = Vec::new();
    decoder.read_to_end(&mut decomp).unwrap();
    decomp
  };

  Ok(tide::Response::builder(200).body(decomp).header("Content-Disposition", format!("attachment; filename={}", file_name)).build())
}

async fn check_permissions(req: &Request<()>, is_dir: bool, write: bool) -> Result<(String, String), tide::Response> {
  if !has_permissions(req, Permissions::Cloud as i32) {
    return Err(tide::Response::new(403));
  }

  let path = percent_decode_str(req.param("path").unwrap_or_default()).decode_utf8_lossy().to_string();
  let dir = if is_dir {
    path.clone()
  } else {
    path.split('/').take(path.split('/').count() - 1).collect::<Vec<&str>>().join("/")
  };

  if !check_access(req, &dir, write).await && !is_admin(req) {
    return Err(tide::Response::new(403));
  }

  Ok((path, dir))
}

async fn check_access(req: &Request<()>, dir: &str, write: bool) -> bool {
  let user = req.header("User").unwrap().as_str();
  let access = get_collection_records::<Access>("cloud", Some(&format!("user='{}'", user))).await.unwrap();
  access.iter()
    .filter(|&a| !write || a.write == write)
    .filter(|a| dir.starts_with(&format!("{}/", a.dir)) || dir == a.dir)
    .reduce(|a, x| if a.dir.len() > x.dir.len() {a} else {x})
    .is_some()
}

async fn check_files_access(req: &Request<()>, files: Vec<CloudFileTemp>, dir: String) -> Vec<CloudFile> {
  let user = req.header("User").unwrap().as_str();
  let access = get_collection_records::<Access>("cloud", Some(&format!("user='{}'", user))).await.unwrap();
  let is_admin = is_admin(req);
  let mut final_files = Vec::new();
  for file in files {
    let file_name_format = if dir.is_empty() {
      file.name.clone()
    } else {
      format!("{}/{}", dir, file.name)
    };
    let parent_access = access.iter()
      .filter(|a| file_name_format.starts_with(&format!("{}/", a.dir)) || file_name_format == a.dir)
      .reduce(|a, x| if a.dir.len() > x.dir.len() {a} else {x});
    
    if is_admin {
      final_files.push(CloudFile{name: file.name, dir: file.dir, write: true});
    } else if parent_access.is_some(){
      final_files.push(CloudFile{name: file.name, dir: file.dir, write: parent_access.unwrap().write});
    } else {
      let child_access = access.iter()
        .filter(|&a| a.dir.starts_with(&format!("{}/", file_name_format)) || a.dir == file_name_format)
        .reduce(|a, x| if a.dir.len() > x.dir.len() {a} else {x});
      if child_access.is_some() {
        final_files.push(CloudFile{name: file.name, dir: file.dir, write: false});
      }
    }
  }
  final_files
}

fn pack_zip(path: &str, files: Vec<String>) -> Result<Vec<u8>, Error> {
  let path = format!("{}/{}", *crate::CLOUD_DIR, path);
  let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
  for file_name in files {
    let mut file = File::open(format!("{}/{}", path, file_name))?;
    if file.metadata()?.is_dir() {
      get_files_from_folder(&path, &file_name, &mut zip)?;
    } else {
      zip.start_file(file_name, Default::default())?;
      std::io::copy(&mut file, &mut zip)?;
    }
  }
  let res = zip.finish()?;
  Ok(res.get_ref().to_vec())
}

fn get_files_from_folder(dir_path: &str, relative_path: &str, zip: &mut ZipWriter<Cursor<Vec<u8>>>) -> Result<(), Error> {
  let dir = std::fs::read_dir(format!("{}/{}", dir_path, relative_path))?;
  for entry in dir {
    let entry = entry?;
    let path = entry.path();
    let relative_path = path.strip_prefix(dir_path).unwrap().to_str().unwrap();
    if path.is_dir() {
      zip.add_directory(relative_path, Default::default())?;
      get_files_from_folder(dir_path, relative_path, zip)?;
    } else {
      let mut file = File::open(&path)?;
      let mut data = Vec::new();
      file.read_to_end(&mut data)?;
      let mut decoder = GzDecoder::new(&data[..]);
      let mut decomp = Vec::new();
      decoder.read_to_end(&mut decomp).unwrap();
      let mut cursor = Cursor::new(decomp);
      
      zip.start_file(relative_path, Default::default())?;
      std::io::copy(&mut cursor, zip)?;
    }
  }

  Ok(())
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
struct CloudFileTemp {
  name: String,
  dir: bool,
}

#[derive(Serialize, Deserialize)]
struct CloudFile {
  name: String,
  dir: bool,
  write: bool,
}

#[derive(Serialize)]
struct Exists {
  count: i32,
}

#[derive(Serialize, Deserialize)]
struct DirectLink {
  uuid: String,
  path: String,
}

impl ModifyRecord for AccessUpdate {
  fn id(&self) -> &String {
    &self.id
  } 
}