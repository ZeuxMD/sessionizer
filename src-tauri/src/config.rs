use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum PauseReason {
    Manual,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub password_hash: String,
    pub recovery_key_hash: String,
    pub timeout_minutes: u64,
    pub warning_minutes: u64,
    pub action: String,
    pub autostart_enabled: bool,
    pub first_run_complete: bool,
    #[serde(default = "default_session_start_pending")]
    pub session_start_pending: bool,
    #[serde(rename = "timer_start_timestamp")]
    pub timer_start_timestamp: Option<u64>,
    #[serde(rename = "timer_paused_at")]
    pub timer_paused_at: Option<u64>,
    pub pause_reason: Option<PauseReason>,
    #[serde(default)]
    pub session_expired: bool,
    pub warning_notification_sent: bool,
}

fn default_session_start_pending() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            password_hash: String::new(),
            recovery_key_hash: String::new(),
            timeout_minutes: 60,
            warning_minutes: 5,
            action: "shutdown".to_string(),
            autostart_enabled: true,
            first_run_complete: false,
            session_start_pending: default_session_start_pending(),
            timer_start_timestamp: None,
            timer_paused_at: None,
            pause_reason: None,
            session_expired: false,
            warning_notification_sent: false,
        }
    }
}

fn get_config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let app_data = std::env::var("APPDATA")
            .or_else(|_| std::env::var("USERPROFILE").map(|p| format!("{}\\AppData\\Roaming", p)))
            .unwrap_or_else(|_| "C:\\ProgramData".to_string());
        PathBuf::from(app_data).join("sessionizer")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/var/opt".to_string());
        PathBuf::from(home).join(".config").join("sessionizer")
    }
}

fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

fn load_config_from_path(path: &Path) -> Result<AppConfig, String> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config at {}: {}", path.display(), e))?;

    let config = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config at {}: {}", path.display(), e))?;

    validate_config(config).map_err(|e| format!("Invalid config at {}: {}", path.display(), e))
}

fn validate_config(config: AppConfig) -> Result<AppConfig, String> {
    if config.timeout_minutes == 0 {
        return Err("screen time limit must be at least 1 minute".to_string());
    }

    if config.warning_minutes == 0 {
        return Err("warning time must be at least 1 minute".to_string());
    }

    if !matches!(config.action.as_str(), "shutdown" | "restart" | "sleep") {
        return Err("action must be shutdown, restart, or sleep".to_string());
    }

    if config.first_run_complete
        && (config.password_hash.is_empty() || config.recovery_key_hash.is_empty())
    {
        return Err("completed setup requires stored password and recovery key hashes".to_string());
    }

    if config.timer_paused_at.is_some() && config.timer_start_timestamp.is_none() {
        return Err("paused sessions require a timer start timestamp".to_string());
    }

    if config.pause_reason.is_some() && config.timer_paused_at.is_none() {
        return Err("pause reason requires a paused timestamp".to_string());
    }

    if config.session_expired && config.timer_start_timestamp.is_none() {
        return Err("expired sessions require a timer start timestamp".to_string());
    }

    Ok(config)
}

pub fn load_config() -> Result<AppConfig, String> {
    load_config_from_path(&get_config_path())
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let dir = get_config_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }

    let path = get_config_path();
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    let temp_path = path.with_extension("json.tmp");

    fs::write(&temp_path, content).map_err(|e| e.to_string())?;

    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }

    fs::rename(&temp_path, &path).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        e.to_string()
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_path(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock drifted backwards")
            .as_nanos();

        std::env::temp_dir().join(format!("sessionizer-{label}-{unique}.json"))
    }

    #[test]
    fn load_config_returns_default_for_missing_file() {
        let path = unique_path("missing");

        let config = load_config_from_path(&path).expect("missing config should not fail");

        assert_eq!(config.timeout_minutes, 60);
        assert!(!config.first_run_complete);
    }

    #[test]
    fn load_config_fails_for_invalid_json() {
        let path = unique_path("invalid");
        fs::write(&path, "{not valid json").expect("failed to create invalid config");

        let result = load_config_from_path(&path);

        assert!(result.is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_config_preserves_expired_state() {
        let path = unique_path("expired");
        let content = r#"{
  "first_run_complete": true,
  "password_hash": "hash",
  "recovery_key_hash": "hash",
  "session_start_pending": false,
  "timer_start_timestamp": 123,
  "timer_paused_at": null,
  "pause_reason": null,
  "session_expired": true
}"#;
        fs::write(&path, content).expect("failed to write config");

        let config = load_config_from_path(&path).expect("failed to parse config");

        assert!(config.first_run_complete);
        assert_eq!(config.timer_start_timestamp, Some(123));
        assert!(config.session_expired);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_config_rejects_expired_state_without_timer() {
        let path = unique_path("invalid-expired");
        let content = r#"{
  "first_run_complete": true,
  "password_hash": "hash",
  "recovery_key_hash": "hash",
  "session_expired": true
}"#;
        fs::write(&path, content).expect("failed to write config");

        let result = load_config_from_path(&path);

        assert!(result.is_err());
        let _ = fs::remove_file(path);
    }
}
