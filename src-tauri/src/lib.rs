mod commands;
mod db;
mod dwca;
mod error;
mod photo_cache;
mod inat_auth;
pub mod tile_server;
mod search_params;

use tauri::menu::{MenuItemBuilder, SubmenuBuilder};
use tauri::{Emitter};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(crate::tile_server::init())
        .invoke_handler(tauri::generate_handler![
            commands::archive::open_archive,
            commands::archive::current_archive,
            commands::archive::search,
            commands::archive::get_autocomplete_suggestions,
            commands::archive::get_occurrence,
            commands::archive::get_photo,
            commands::inat_download::get_observation_count,
            commands::inat_download::generate_inat_archive,
            commands::inat_download::cancel_inat_archive,
            commands::inat_auth::inat_authenticate,
            commands::inat_auth::inat_get_auth_status,
            commands::inat_auth::inat_sign_out,
            commands::inat_auth::inat_get_jwt,
        ])
        .setup(|app| {
            let open_item = MenuItemBuilder::with_id("open", "Open...")
                .accelerator("CmdOrCtrl+O")
                .build(app)?;

            let download_item = MenuItemBuilder::with_id("download-from-inaturalist", "Download from iNaturalist")
                .build(app)?;

            // Get the existing menu and add to the File submenu
            if let Some(menu) = app.menu() {
                for item in menu.items()? {
                    if let Some(submenu) = item.as_submenu()
                        && let Ok(text) = submenu.text()
                        && text == "File"
                    {
                        submenu.prepend(&open_item)?;
                        break
                    }
                }

                // Create Tools submenu if it doesn't exist, or add to existing
                let mut tools_submenu_exists = false;
                for item in menu.items()? {
                    if let Some(submenu) = item.as_submenu() {
                        if let Ok(text) = submenu.text() {
                            if text == "Tools" {
                                submenu.append(&download_item)?;
                                tools_submenu_exists = true;
                                break;
                            }
                        }
                    }
                }

                // If Tools submenu doesn't exist, create it
                if !tools_submenu_exists {
                    let tools_submenu = SubmenuBuilder::new(app, "Tools")
                        .item(&download_item)
                        .build()?;
                    menu.append(&tools_submenu)?;
                }
            }

            app.on_menu_event(move |app, event| {
                if event.id() == "open" {
                    app.emit("menu-open", ()).unwrap();
                } else if event.id() == "download-from-inaturalist" {
                    // Open new window for iNat download
                    let window = tauri::WebviewWindowBuilder::new(
                        app,
                        "inat-download",
                        tauri::WebviewUrl::App("inat-download".into())
                    )
                    .title("Download from iNaturalist")
                    .inner_size(700.0, 800.0)
                    .resizable(true)
                    .build();

                    if let Err(e) = window {
                        log::error!("Failed to open iNat download window: {}", e);
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
