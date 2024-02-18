use tide::Request;

pub(crate) enum Permissions {
  Admin = 0,
  Users = 1,
  Metrics = 2,
  Cloud = 4,
  Portainer = 8,
}

pub(crate) fn has_permissions(req: &Request<()> , permissions: i32) -> bool {
  let req_permissions: i32 = req.header("Permissions").unwrap().as_str().parse().unwrap();
  req_permissions & permissions == permissions || req_permissions == Permissions::Admin as i32
}