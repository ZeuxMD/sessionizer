use crate::control::{self, AdminSessionSnapshot, ExpiredActionStatus, FrontendConfig};
use crate::remote_admin::AdminPanelInfo;
use tauri::AppHandle;

#[tauri::command]
#[specta::specta]
pub fn get_config() -> Result<FrontendConfig, String> {
    control::get_config()
}

#[tauri::command]
#[specta::specta]
pub fn finish_setup() -> Result<(), String> {
    control::finish_setup()
}

#[tauri::command]
#[specta::specta]
pub fn update_settings(
    timeout_minutes: u64,
    warning_minutes: u64,
    action: String,
    autostart_enabled: bool,
) -> Result<(), String> {
    control::update_settings(timeout_minutes, warning_minutes, action, autostart_enabled)
}

#[tauri::command]
#[specta::specta]
pub fn is_first_run() -> Result<bool, String> {
    control::is_first_run()
}

#[tauri::command]
#[specta::specta]
pub fn setup_password(password: String, timeout_minutes: u64) -> Result<String, String> {
    control::setup_password(password, timeout_minutes)
}

#[tauri::command]
#[specta::specta]
pub fn verify_password(password: String) -> Result<bool, String> {
    control::verify_password(password)
}

#[tauri::command]
#[specta::specta]
pub fn verify_recovery_key(key: String) -> Result<bool, String> {
    control::verify_recovery_key(key)
}

#[tauri::command]
#[specta::specta]
pub fn reset_password_with_recovery(key: String, new_password: String) -> Result<bool, String> {
    control::reset_password_with_recovery(key, new_password)
}

#[tauri::command]
#[specta::specta]
pub fn change_password(current: String, new_password: String) -> Result<bool, String> {
    control::change_password(current, new_password)
}

#[tauri::command]
#[specta::specta]
pub fn start_timer() -> Result<(), String> {
    control::start_timer()
}

#[tauri::command]
#[specta::specta]
pub fn clear_timer() -> Result<(), String> {
    control::unlock_session()
}

#[tauri::command]
#[specta::specta]
pub fn pause_timer() -> Result<(), String> {
    control::pause_timer()
}

#[tauri::command]
#[specta::specta]
pub fn resume_timer() -> Result<(), String> {
    control::resume_timer()
}

#[tauri::command]
#[specta::specta]
pub fn get_remaining_seconds() -> Result<Option<u64>, String> {
    control::get_remaining_seconds()
}

#[tauri::command]
#[specta::specta]
pub fn mark_warning_notification_sent() -> Result<(), String> {
    control::mark_warning_notification_sent()
}

#[tauri::command]
#[specta::specta]
pub fn execute_expired_action() -> Result<ExpiredActionStatus, String> {
    control::execute_expired_action()
}

#[tauri::command]
#[specta::specta]
pub fn get_admin_session_snapshot() -> Result<AdminSessionSnapshot, String> {
    control::get_admin_session_snapshot()
}

#[tauri::command]
#[specta::specta]
pub fn get_admin_panel_info() -> Result<AdminPanelInfo, String> {
    control::get_admin_panel_info()
}

#[tauri::command]
#[specta::specta]
pub fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}
