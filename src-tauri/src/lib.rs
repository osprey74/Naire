mod commands;
mod filter;
mod macro_io;
mod rename;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::list_entries,
            commands::preview_rename,
            commands::execute_rename,
            commands::init_macro_items,
            commands::apply_macro_step,
            commands::apply_rename_to_filesystem,
            commands::undo_rename,
            commands::compute_group_preview,
            commands::execute_group,
            commands::export_macros,
            commands::import_macros,
            commands::list_folder_tree,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
