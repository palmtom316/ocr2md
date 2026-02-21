#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ocr2md_desktop::state::AppState;

fn main() {
    let state = AppState::default();
    let state_clone = state.clone();

    tauri::Builder::default()
        .manage(state)
        .setup(|app| {
            ocr2md_desktop::worker::spawn_worker(app.handle().clone(), state_clone);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ocr2md_desktop::commands::enqueue_files,
            ocr2md_desktop::commands::start_queue,
            ocr2md_desktop::commands::retry_job,
            ocr2md_desktop::commands::load_profiles,
            ocr2md_desktop::commands::save_profiles
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
