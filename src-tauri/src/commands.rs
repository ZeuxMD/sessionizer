use crate::config::{load_config, save_config, AppConfig};
use crate::password::{generate_recovery_key, hash_password, verify_password as verify_pwd};
use crate::shutdown::execute_action;
use chrono::Utc;
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
pub fn is_first_run() -> Result<bool, String> {
    let config = load_config();
    Ok(!config.first_run_complete && config.password_hash.is_empty())
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
        first_run_complete: true,
        timer_start_timestamp: Some(Utc::now().timestamp() as u64),
        timer_paused_at: None,
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
    config.timer_start_timestamp = Some(Utc::now().timestamp() as u64);
    save_config(&config)
}

#[tauri::command]
pub fn clear_timer() -> Result<(), String> {
    let mut config = load_config();
    config.timer_start_timestamp = None;
    config.timer_paused_at = None;
    save_config(&config)
}

#[tauri::command]
pub fn pause_timer() -> Result<(), String> {
    let mut config = load_config();
    if config.timer_start_timestamp.is_some() && config.timer_paused_at.is_none() {
        config.timer_paused_at = Some(Utc::now().timestamp() as u64);
        save_config(&config)
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn resume_timer() -> Result<(), String> {
    let mut config = load_config();
    if let (Some(start_timestamp), Some(paused_at)) =
        (config.timer_start_timestamp, config.timer_paused_at)
    {
        let now = Utc::now().timestamp() as u64;
        let pause_duration = now.saturating_sub(paused_at);
        config.timer_start_timestamp = Some(start_timestamp.saturating_add(pause_duration));
        config.timer_paused_at = None;
        save_config(&config)
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn get_remaining_seconds() -> Result<Option<u64>, String> {
    let config = load_config();

    if let Some(start_timestamp) = config.timer_start_timestamp {
        let now = Utc::now().timestamp() as u64;
        let total_seconds = config.timeout_minutes * 60;

        let effective_now = config.timer_paused_at.unwrap_or(now);
        let elapsed = effective_now.saturating_sub(start_timestamp);

        if elapsed >= total_seconds {
            Ok(Some(0))
        } else {
            Ok(Some(total_seconds - elapsed))
        }
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}
