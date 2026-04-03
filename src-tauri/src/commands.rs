use crate::config::{load_config, save_config, AppConfig, PauseReason};
use crate::password::{generate_recovery_key, hash_password, verify_password as verify_pwd};
use crate::session;
use crate::shutdown::execute_action;
use tauri::AppHandle;

#[tauri::command]
pub fn get_config() -> Result<AppConfig, String> {
    Ok(load_config())
}

#[tauri::command]
pub fn save_config_cmd(config: AppConfig) -> Result<(), String> {
    save_config(&config)
}

#[tauri::command]
pub fn update_settings(
    timeout_minutes: u64,
    warning_minutes: u64,
    action: String,
    autostart_enabled: bool,
) -> Result<(), String> {
    let mut config = load_config();
    config.timeout_minutes = timeout_minutes;
    config.warning_minutes = warning_minutes;
    config.action = action;
    config.autostart_enabled = autostart_enabled;
    save_config(&config)
}

#[tauri::command]
pub fn is_first_run() -> Result<bool, String> {
    let config = load_config();
    Ok(!config.first_run_complete)
}

#[tauri::command]
pub fn setup_password(password: String, timeout_minutes: u64) -> Result<String, String> {
    let recovery_key = generate_recovery_key();
    let password_hash = hash_password(&password)?;
    let recovery_key_hash = hash_password(&recovery_key)?;

    let config = AppConfig {
        password_hash,
        recovery_key_hash,
        timeout_minutes,
        warning_minutes: 5,
        action: "shutdown".to_string(),
        autostart_enabled: true,
        first_run_complete: false,
        session_start_pending: true,
        timer_start_timestamp: None,
        timer_paused_at: None,
        pause_reason: None,
        warning_notification_sent: false,
    };

    save_config(&config)?;
    Ok(recovery_key)
}

#[tauri::command]
pub fn verify_password(password: String) -> Result<bool, String> {
    let config = load_config();
    verify_pwd(&password, &config.password_hash)
}

#[tauri::command]
pub fn verify_recovery_key(key: String) -> Result<bool, String> {
    let config = load_config();
    verify_pwd(&key, &config.recovery_key_hash)
}

#[tauri::command]
pub fn reset_password_with_recovery(key: String, new_password: String) -> Result<bool, String> {
    let config = load_config();

    if !verify_pwd(&key, &config.recovery_key_hash)? {
        return Ok(false);
    }

    let new_hash = hash_password(&new_password)?;
    let mut new_config = config;
    new_config.password_hash = new_hash;

    save_config(&new_config)?;
    Ok(true)
}

#[tauri::command]
pub fn change_password(current: String, new_password: String) -> Result<bool, String> {
    let config = load_config();

    if !verify_pwd(&current, &config.password_hash)? {
        return Ok(false);
    }

    let new_hash = hash_password(&new_password)?;
    let mut new_config = config;
    new_config.password_hash = new_hash;

    save_config(&new_config)?;
    Ok(true)
}

#[tauri::command]
pub fn execute_shutdown(action: String) -> Result<(), String> {
    execute_action(&action)
}

#[tauri::command]
pub fn start_timer() -> Result<(), String> {
    let mut config = load_config();
    session::start_session(&mut config, session::current_timestamp());
    save_config(&config)
}

#[tauri::command]
pub fn clear_timer() -> Result<(), String> {
    let mut config = load_config();
    session::clear_session(&mut config);
    config.session_start_pending = false;
    save_config(&config)
}

#[tauri::command]
pub fn clear_timer_for_next_login() -> Result<(), String> {
    let mut config = load_config();
    session::clear_session(&mut config);
    config.session_start_pending = true;
    save_config(&config)
}

#[tauri::command]
pub fn pause_timer() -> Result<(), String> {
    let mut config = load_config();
    if session::pause_session(
        &mut config,
        PauseReason::Manual,
        session::current_timestamp(),
    ) {
        save_config(&config)
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn resume_timer() -> Result<(), String> {
    let mut config = load_config();
    if session::resume_session(&mut config, session::current_timestamp()) {
        save_config(&config)
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn get_remaining_seconds() -> Result<Option<u64>, String> {
    let config = load_config();
    Ok(session::get_remaining_seconds(&config))
}

#[tauri::command]
pub fn mark_warning_notification_sent() -> Result<(), String> {
    let mut config = load_config();
    if session::mark_warning_notification_sent(&mut config) {
        save_config(&config)
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}
