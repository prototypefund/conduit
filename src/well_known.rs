#[cfg(feature = "conduit_bin")]
use rocket::get;
use rocket::response::content;
use crate::database::DatabaseGuard;
//use serde::Serialize;

//#[derive(Serialize)]
//pub struct WellKnownServerInfo {
//    m_server: String,
//}

#[cfg_attr(feature = "conduit_bin", get("/.well-known/matrix/server"))]
#[tracing::instrument(skip(db))]
pub fn well_known_server(db: DatabaseGuard) -> content::Json<String> {
    // Pull the server_name from config
    let server_name = db.globals.server_name().clone();
    content::Json(format!("{{\"m.server\": \"{}:443\"}}", server_name))
}


#[cfg_attr(feature = "conduit_bin", get("/.well-known/matrix/client"))]
#[tracing::instrument(skip(db))]
pub fn well_known_client(db: DatabaseGuard) -> content::Json<String> {
    // Pull the server_name from config
    let server_name = db.globals.server_name().clone();
    content::Json(format!("{{\"m.homeserver\": {{ \"base_url\": \"https://{}\"}}}}", server_name))
}
