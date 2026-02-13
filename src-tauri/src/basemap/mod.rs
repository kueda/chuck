pub mod commands;
pub mod protocol;

use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("basemap-server")
        .register_asynchronous_uri_scheme_protocol(
            "basemap",
            protocol::handle_basemap_request,
        )
        .build()
}
