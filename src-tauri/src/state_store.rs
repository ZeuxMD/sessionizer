use crate::config::{validate_config, AppConfig};
use std::fs;
use std::path::{Path, PathBuf};

const MACHINE_STATE_FILE: &str = "device-state.json";
const LEGACY_STATE_FILE: &str = "config.json";

#[derive(Debug, Clone)]
pub struct StateStore {
    machine_root: PathBuf,
    machine_file_name: &'static str,
    legacy_user_root: Option<PathBuf>,
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new(default_machine_root(), Some(default_legacy_user_root()))
    }
}

impl StateStore {
    pub fn new(machine_root: PathBuf, legacy_user_root: Option<PathBuf>) -> Self {
        Self {
            machine_root,
            machine_file_name: MACHINE_STATE_FILE,
            legacy_user_root,
        }
    }

    pub fn legacy_profile(root: PathBuf) -> Self {
        Self {
            machine_root: root,
            machine_file_name: LEGACY_STATE_FILE,
            legacy_user_root: None,
        }
    }

    pub fn machine_config_path(&self) -> PathBuf {
        self.machine_root.join(self.machine_file_name)
    }

    pub fn legacy_config_path(&self) -> Option<PathBuf> {
        self.legacy_user_root
            .as_ref()
            .map(|root| root.join(LEGACY_STATE_FILE))
    }

    pub fn load(&self) -> Result<AppConfig, String> {
        let machine_path = self.machine_config_path();
        if machine_path.exists() {
            return load_config_from_path(&machine_path);
        }

        if let Some(legacy_path) = self.legacy_config_path() {
            if legacy_path.exists() {
                let config = load_config_from_path(&legacy_path)?;
                self.save(&config)?;
                return Ok(config);
            }
        }

        Ok(AppConfig::default())
    }

    pub fn save(&self, config: &AppConfig) -> Result<(), String> {
        let config = validate_config(config.clone())?;
        write_config_atomic(&self.machine_config_path(), &config)
    }
}

fn default_machine_root() -> PathBuf {
    if let Ok(override_root) = std::env::var("SESSIONIZER_STATE_DIR") {
        return PathBuf::from(override_root);
    }

    #[cfg(target_os = "windows")]
    {
        let program_data =
            std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData".into());
        PathBuf::from(program_data).join("Sessionizer")
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/var/opt".to_string());
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("sessionizer")
    }
}

fn default_legacy_user_root() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let app_data = std::env::var("APPDATA")
            .or_else(|_| {
                std::env::var("USERPROFILE").map(|path| format!("{path}\\AppData\\Roaming"))
            })
            .unwrap_or_else(|_| "C:\\ProgramData".to_string());
        PathBuf::from(app_data).join("sessionizer")
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/var/opt".to_string());
        PathBuf::from(home).join(".config").join("sessionizer")
    }
}

fn load_config_from_path(path: &Path) -> Result<AppConfig, String> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read config at {}: {error}", path.display()))?;
    let config = serde_json::from_str(&content)
        .map_err(|error| format!("Failed to parse config at {}: {error}", path.display()))?;

    validate_config(config)
        .map_err(|error| format!("Invalid config at {}: {error}", path.display()))
}

fn write_config_atomic(path: &Path, config: &AppConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let content = serde_json::to_string_pretty(config).map_err(|error| error.to_string())?;
    let temp_path = path.with_extension("json.tmp");

    fs::write(&temp_path, content).map_err(|error| error.to_string())?;

    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }

    fs::rename(&temp_path, path).map_err(|error| {
        let _ = fs::remove_file(&temp_path);
        error.to_string()
    })?;

    Ok(())
}
