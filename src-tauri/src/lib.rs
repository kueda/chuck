mod commands;
mod db;
mod dwca;
mod error;

use tauri::menu::MenuItemBuilder;
use tauri::Emitter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::archive::open_archive,
            commands::archive::current_archive,
            commands::archive::search,
            commands::archive::get_occurrence
        ])
        .setup(|app| {
            let open_item = MenuItemBuilder::with_id("open", "Open...")
                .accelerator("CmdOrCtrl+O")
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
            }

            app.on_menu_event(move |app, event| {
                if event.id() == "open" {
                    app.emit("menu-open", ()).unwrap();
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
