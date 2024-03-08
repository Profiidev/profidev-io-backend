use tide::Request;

pub(crate) enum Permissions {
  Admin = 1,
  Users = 2,
  Metrics = 4,
  Cloud = 8,
  Portainer = 16,
  Database = 32,
  CloudManage = 64,
}

pub(crate) fn has_permissions(req: &Request<()> , permissions: i32) -> bool {
  let req_permissions: i32 = req.header("Permissions").unwrap().as_str().parse().unwrap();
  req_permissions & permissions == permissions || (req_permissions & Permissions::Admin as i32) == Permissions::Admin as i32
}

pub(crate) fn is_admin(req: &Request<()>) -> bool {
  let req_permissions: i32 = req.header("Permissions").unwrap().as_str().parse().unwrap();
  (req_permissions & Permissions::Admin as i32) == Permissions::Admin as i32
}