mod basemap;
mod commands;
pub mod db;
pub mod dwca;
pub mod error;
mod photo_cache;
pub mod tile_server;
pub mod search_params;

use chuck_core::auth::AuthCache;
use tauri::image::Image;
use tauri::menu::{AboutMetadata, Menu, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(crate::tile_server::init())
        .plugin(crate::basemap::init())
        .invoke_handler(tauri::generate_handler![
            commands::archive::open_archive,
            commands::archive::current_archive,
            commands::archive::search,
            commands::archive::get_autocomplete_suggestions,
            commands::archive::get_occurrence,
            commands::archive::get_photo,
            commands::archive::aggregate_by_field,
            commands::archive::get_archive_metadata,
            commands::inat_download::get_observation_count,
            commands::inat_download::estimate_photo_count,
            commands::inat_download::generate_inat_archive,
            commands::inat_download::cancel_inat_archive,
            commands::inat_auth::inat_authenticate,
            commands::inat_auth::inat_get_auth_status,
            commands::inat_auth::inat_sign_out,
            commands::inat_auth::inat_get_jwt,
            basemap::commands::list_basemaps,
            basemap::commands::download_basemap,
            basemap::commands::download_regional_basemap,
            basemap::commands::estimate_regional_size,
            basemap::commands::cancel_basemap_download,
            basemap::commands::delete_basemap,
            basemap::commands::reverse_geocode,
        ])
        .setup(|app| {
            // Initialize auth cache (lazy - won't access keychain until first use)
            app.manage(AuthCache::new());

            let open_item = MenuItemBuilder::with_id("open", "Open...")
                .accelerator("CmdOrCtrl+O")
                .build(app)?;

            let download_item = MenuItemBuilder::with_id(
                "download-from-inaturalist",
                "Download from iNaturalist"
            ).build(app)?;

            let basemap_item = MenuItemBuilder::with_id(
                "download-basemap",
                "Download Offline Basemap\u{2026}",
            )
            .build(app)?;

            let metadata_item = MenuItemBuilder::with_id(
                "show-metadata",
                "Show Archive Metadata",
            )
            .accelerator("CmdOrCtrl+I")
            .build(app)?;

            // Get the existing menu or create one for the main window (needed for Windows/Linux)
            let menu = match app.menu() {
                Some(m) => m,
                None => {
                    let m = Menu::default(app.handle())?;
                    // On Windows/Linux there's no app-level menu, so set it on the main window only
                    if let Some(window) = app.get_webview_window("main") {
                        window.set_menu(m.clone())?;
                    }
                    m
                }
            };

            // Replace the default "About chuck-app" with "About Chuck"
            if let Some(first_item) = menu.items()?.first() {
                if let Some(app_submenu) = first_item.as_submenu() {
                    for sub_item in app_submenu.items()? {
                        if let Some(predefined) = sub_item.as_predefined_menuitem()
                            && let Ok(text) = predefined.text()
                            && text.starts_with("About")
                        {
                            app_submenu.remove(predefined)?;
                            let about = PredefinedMenuItem::about(
                                app.handle(),
                                Some("About Chuck"),
                                Some(AboutMetadata {
                                    name: Some("Chuck".into()),
                                    version: Some(
                                        app.package_info().version.to_string()
                                    ),
                                    license: Some("MIT".into()),
                                    website: Some(
                                        "https://github.com/kueda/chuck".into()
                                    ),
                                    website_label: Some(
                                        "GitHub".into()
                                    ),
                                    icon: Image::from_bytes(include_bytes!("../icons/icon.png")).ok(),
                                    ..Default::default()
                                }),
                            )?;
                            app_submenu.prepend(&about)?;
                            break;
                        }
                    }
                }
            }

            // Get or create File submenu and add Open item
            let mut file_submenu_exists = false;
            for item in menu.items()? {
                if let Some(submenu) = item.as_submenu()
                    && let Ok(text) = submenu.text()
                    && text == "File"
                {
                    submenu.prepend(&open_item)?;
                    file_submenu_exists = true;
                    break;
                }
            }

            // If File submenu doesn't exist, create it
            if !file_submenu_exists {
                let file_submenu = SubmenuBuilder::new(app, "File")
                    .item(&open_item)
                    .build()?;
                menu.insert(&file_submenu, 0)?;
            }

            // Create Tools submenu if it doesn't exist, or add to existing
            let mut tools_submenu_exists = false;
            for item in menu.items()? {
                if let Some(submenu) = item.as_submenu() {
                    if let Ok(text) = submenu.text() {
                        if text == "Tools" {
                            submenu.append(&download_item)?;
                            submenu.append(&basemap_item)?;
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
                    .item(&basemap_item)
                    .build()?;
                menu.append(&tools_submenu)?;
            }

            // Get or create View submenu
            let mut view_submenu_exists = false;
            for item in menu.items()? {
                if let Some(submenu) = item.as_submenu() {
                    if let Ok(text) = submenu.text() {
                        if text == "View" {
                            submenu.append(&metadata_item)?;
                            view_submenu_exists = true;
                            break;
                        }
                    }
                }
            }

            // If View submenu doesn't exist, create it
            if !view_submenu_exists {
                let view_submenu = SubmenuBuilder::new(app, "View")
                    .item(&metadata_item)
                    .build()?;
                menu.append(&view_submenu)?;
            }

            // Remove empty submenus (e.g. Help on macOS)
            for item in menu.items()? {
                if let Some(submenu) = item.as_submenu()
                    && (
                        // Remove empty submenus
                        submenu.items()?.is_empty()
                        // Remove Window menu on Linux, which Tauri thinks exists but it doesn't
                        || cfg!(target_os = "linux") && submenu.text()? == "Window"
                    )
                {
                    menu.remove(&item)?;
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
                        log::error!("Failed to open iNat download window: {e}");
                    }
                } else if event.id() == "download-basemap" {
                    let window = tauri::WebviewWindowBuilder::new(
                        app,
                        "offline-basemaps",
                        tauri::WebviewUrl::App("offline-basemaps".into())
                    )
                    .title("Offline Basemaps")
                    .inner_size(800.0, 600.0)
                    .resizable(true)
                    .build();

                    if let Err(e) = window {
                        log::error!(
                            "Failed to open offline basemaps window: {e}"
                        );
                    }
                } else if event.id() == "show-metadata" {
                    // Open new window for archive metadata
                    let window = tauri::WebviewWindowBuilder::new(
                        app,
                        "metadata",
                        tauri::WebviewUrl::App("metadata".into())
                    )
                    .title("Archive Metadata")
                    .inner_size(1024.0, 680.0)
                    .resizable(true)
                    .build();

                    if let Err(e) = window {
                        log::error!("Failed to open metadata window: {e}");
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
