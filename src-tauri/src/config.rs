use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

pub fn load_config() -> AppConfig {
    let path = get_config_path();
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(config) => return config,
                Err(e) => {
                    eprintln!("Failed to parse config: {}", e);
                }
            },
            Err(e) => {
                eprintln!("Failed to read config: {}", e);
            }
        }
    }
    AppConfig::default()
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let dir = get_config_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    let path = get_config_path();
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}
