use tauri::State;

use crate::config::Settings;
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.get()
}

#[tauri::command]
pub fn update_settings(state: State<'_, AppState>, settings: Settings) -> AppResult<Settings> {
    let _work = state.work.enter()?;
    state.settings.replace(settings)
}

#[tauri::command]
pub fn reset_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    let _work = state.work.enter()?;
    state.settings.replace(Settings::default())
}
