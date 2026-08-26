pub mod build;
pub mod commands;
pub mod config;
pub mod error;
pub mod gguf;
pub mod git;
pub mod hardware;
pub mod llama;
pub mod logging;
pub mod models;
pub mod platform;
pub mod process;
pub mod profiles;
pub mod runtime;
pub mod server;
pub mod sources;
pub mod state;

use tauri::Manager;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState::initialize(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(generated_command_handler!())
        .run(tauri::generate_context!())
        .expect("error while running Llama Control");
}
