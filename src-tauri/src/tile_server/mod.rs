pub mod coords;
pub mod mvt;
pub mod protocol;

use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

pub use protocol::generate_tile;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("tile-server")
        .register_asynchronous_uri_scheme_protocol("tiles", protocol::handle_tile_request)
        .build()
}
