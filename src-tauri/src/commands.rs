use crate::config::{load_config, save_config, AppConfig, PauseReason};
use crate::password::{generate_recovery_key, hash_password, verify_password as verify_pwd};
use crate::session;
use crate::shutdown::execute_action;
use serde::Serialize;
use tauri::AppHandle;

#[derive(Serialize, specta::Type)]
pub struct FrontendConfig {
    pub timeout_minutes: u64,
    pub warning_minutes: u64,
    pub action: String,
    pub autostart_enabled: bool,
    pub first_run_complete: bool,
    pub session_start_pending: bool,
    pub timer_start_timestamp: Option<u64>,
    pub timer_paused_at: Option<u64>,
    pub pause_reason: Option<PauseReason>,
    pub session_expired: bool,
    pub warning_notification_sent: bool,
}

impl From<AppConfig> for FrontendConfig {
    fn from(config: AppConfig) -> Self {
        Self {
            timeout_minutes: config.timeout_minutes,
            warning_minutes: config.warning_minutes,
            action: config.action,
            autostart_enabled: config.autostart_enabled,
            first_run_complete: config.first_run_complete,
            session_start_pending: config.session_start_pending,
            timer_start_timestamp: config.timer_start_timestamp,
            timer_paused_at: config.timer_paused_at,
            pause_reason: config.pause_reason,
            session_expired: config.session_expired,
            warning_notification_sent: config.warning_notification_sent,
        }
    }
}

#[derive(Clone, Copy, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ExpiredActionStatus {
    NoActionNeeded,
    ActionStarted,
    LockedOnFailure,
}

fn validate_password(password: &str) -> Result<(), String> {
    if password.chars().count() < 4 {
        Err("Password must be at least 4 characters".to_string())
    } else {
        Ok(())
    }
}

fn validate_timeout_minutes(timeout_minutes: u64) -> Result<(), String> {
    if (5..=180).contains(&timeout_minutes) {
        Ok(())
    } else {
        Err("Screen time limit must be between 5 and 180 minutes".to_string())
    }
}

fn validate_warning_minutes(warning_minutes: u64) -> Result<(), String> {
    if (1..=30).contains(&warning_minutes) {
        Ok(())
    } else {
        Err("Warning time must be between 1 and 30 minutes".to_string())
    }
}

fn validate_action(action: &str) -> Result<(), String> {
    match action {
        "shutdown" | "restart" | "sleep" => Ok(()),
        _ => Err("Invalid action".to_string()),
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_config() -> Result<FrontendConfig, String> {
    Ok(load_config()?.into())
}

#[tauri::command]
#[specta::specta]
pub fn finish_setup() -> Result<(), String> {
    let mut config = load_config()?;
    config.first_run_complete = true;
    save_config(&config)
}

#[tauri::command]
#[specta::specta]
pub fn update_settings(
    timeout_minutes: u64,
    warning_minutes: u64,
    action: String,
    autostart_enabled: bool,
) -> Result<(), String> {
    validate_timeout_minutes(timeout_minutes)?;
    validate_warning_minutes(warning_minutes)?;
    validate_action(&action)?;

    let mut config = load_config()?;
    config.timeout_minutes = timeout_minutes;
    config.warning_minutes = warning_minutes;
    config.action = action;
    config.autostart_enabled = autostart_enabled;
    save_config(&config)
}

#[tauri::command]
#[specta::specta]
pub fn is_first_run() -> Result<bool, String> {
    let config = load_config()?;
    Ok(!config.first_run_complete)
}

#[tauri::command]
#[specta::specta]
pub fn setup_password(password: String, timeout_minutes: u64) -> Result<String, String> {
    validate_password(&password)?;
    validate_timeout_minutes(timeout_minutes)?;

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
        session_expired: false,
        warning_notification_sent: false,
    };

    save_config(&config)?;
    Ok(recovery_key)
}

#[tauri::command]
#[specta::specta]
pub fn verify_password(password: String) -> Result<bool, String> {
    let config = load_config()?;
    verify_pwd(&password, &config.password_hash)
}

#[tauri::command]
#[specta::specta]
pub fn verify_recovery_key(key: String) -> Result<bool, String> {
    let config = load_config()?;
    verify_pwd(&key, &config.recovery_key_hash)
}

#[tauri::command]
#[specta::specta]
pub fn reset_password_with_recovery(key: String, new_password: String) -> Result<bool, String> {
    validate_password(&new_password)?;

    let config = load_config()?;

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
#[specta::specta]
pub fn change_password(current: String, new_password: String) -> Result<bool, String> {
    validate_password(&new_password)?;

    let config = load_config()?;

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
#[specta::specta]
pub fn start_timer() -> Result<(), String> {
    let mut config = load_config()?;
    session::start_session(&mut config, session::current_timestamp());
    save_config(&config)
}

#[tauri::command]
#[specta::specta]
pub fn clear_timer() -> Result<(), String> {
    let mut config = load_config()?;
    session::clear_session(&mut config);
    config.session_start_pending = false;
    save_config(&config)
}

#[tauri::command]
#[specta::specta]
pub fn pause_timer() -> Result<(), String> {
    let mut config = load_config()?;
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
#[specta::specta]
pub fn resume_timer() -> Result<(), String> {
    let mut config = load_config()?;
    if session::resume_session(&mut config, session::current_timestamp()) {
        save_config(&config)
    } else {
        Ok(())
    }
}

#[tauri::command]
#[specta::specta]
pub fn get_remaining_seconds() -> Result<Option<u64>, String> {
    let config = load_config()?;
    Ok(session::get_remaining_seconds(&config))
}

#[tauri::command]
#[specta::specta]
pub fn mark_warning_notification_sent() -> Result<(), String> {
    let mut config = load_config()?;
    if session::mark_warning_notification_sent(&mut config) {
        save_config(&config)
    } else {
        Ok(())
    }
}

#[tauri::command]
#[specta::specta]
pub fn execute_expired_action() -> Result<ExpiredActionStatus, String> {
    let mut config = load_config()?;

    if config.timer_start_timestamp.is_none() || config.session_expired {
        return Ok(ExpiredActionStatus::NoActionNeeded);
    }

    if session::expire_session(&mut config) {
        save_config(&config)?;
    }

    match execute_action(&config.action) {
        Ok(()) => Ok(ExpiredActionStatus::ActionStarted),
        Err(_) => Ok(ExpiredActionStatus::LockedOnFailure),
    }
}

#[tauri::command]
#[specta::specta]
pub fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}
