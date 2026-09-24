use crate::config::{AppConfig, PauseReason};
use crate::password::{generate_recovery_key, hash_password, verify_password as verify_pwd};
use crate::session::{self, SessionSignal, StartupAction};
use crate::shutdown::execute_action;
use crate::state_store::StateStore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct DeviceController {
    store: StateStore,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum AdminSessionState {
    Setup,
    Unlocked,
    Locked,
    Paused,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
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

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct AdminSessionSnapshot {
    pub session_state: AdminSessionState,
    pub remaining_seconds: Option<u64>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ExpiredActionStatus {
    NoActionNeeded,
    ActionStarted,
    LockedOnFailure,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdate {
    pub timeout_minutes: Option<u64>,
    pub warning_minutes: Option<u64>,
    pub action: Option<String>,
    pub autostart_enabled: Option<bool>,
    pub remote_admin_enabled: Option<bool>,
}

impl DeviceController {
    pub fn new(store: StateStore) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &StateStore {
        &self.store
    }

    pub fn frontend_config(&self) -> Result<FrontendConfig, String> {
        Ok(self.snapshot()?.into())
    }

    pub fn snapshot(&self) -> Result<AdminSessionSnapshot, String> {
        Ok(build_admin_session_snapshot(self.store.load()?))
    }

    pub fn is_first_run(&self) -> Result<bool, String> {
        Ok(!self.store.load()?.first_run_complete)
    }

    pub fn setup_password(&self, password: String, timeout_minutes: u64) -> Result<String, String> {
        validate_password(&password)?;
        validate_timeout_minutes(timeout_minutes)?;

        self.store.update(|config| {
            if config.first_run_complete {
                return Err(
                    "Setup is already complete. Use password recovery to reset the password."
                        .to_string(),
                );
            }
            let recovery_key = generate_recovery_key();
            *config = AppConfig {
                password_hash: hash_password(&password)?,
                recovery_key_hash: hash_password(&recovery_key)?,
                timeout_minutes,
                ..AppConfig::default()
            };
            Ok(recovery_key)
        })
    }

    pub fn finish_setup(&self) -> Result<(), String> {
        self.update_config(|config| {
            config.first_run_complete = true;
            Ok(())
        })
    }

    pub fn verify_local_unlock(&self, secret: &str) -> Result<bool, String> {
        Ok(self.authenticate(secret)?.is_some())
    }

    pub(crate) fn password_hash(&self) -> Result<String, String> {
        Ok(self.store.load()?.password_hash)
    }

    /// Return the exact credential verified, so a concurrent password change
    /// cannot give an old login a session authorized by the new password.
    pub(crate) fn authenticate(&self, secret: &str) -> Result<Option<String>, String> {
        let hash = self.password_hash()?;
        Ok(verify_pwd(secret, &hash)?.then_some(hash))
    }

    pub fn verify_recovery_key(&self, key: String) -> Result<bool, String> {
        let config = self.store.load()?;
        verify_pwd(&key, &config.recovery_key_hash)
    }

    pub fn reset_password_with_recovery(
        &self,
        key: String,
        new_password: String,
    ) -> Result<bool, String> {
        validate_password(&new_password)?;

        self.store.update(|config| {
            if !verify_pwd(&key, &config.recovery_key_hash)? {
                return Ok(false);
            }
            config.password_hash = hash_password(&new_password)?;
            Ok(true)
        })
    }

    pub fn change_password(&self, current: String, new_password: String) -> Result<bool, String> {
        validate_password(&new_password)?;
        self.store.update(|config| {
            if !verify_pwd(&current, &config.password_hash)? {
                return Ok(false);
            }
            config.password_hash = hash_password(&new_password)?;
            Ok(true)
        })
    }

    pub fn unlock(&self) -> Result<(), String> {
        self.update_config(|config| {
            session::clear_session(config);
            config.session_start_pending = false;
            Ok(())
        })
    }

    pub fn relock(&self) -> Result<(), String> {
        self.update_config(|config| {
            session::restart_session(config, session::current_timestamp());
            Ok(())
        })
    }

    pub fn start_timer(&self) -> Result<(), String> {
        self.update_config(|config| {
            session::start_session(config, session::current_timestamp());
            Ok(())
        })
    }

    pub fn pause(&self) -> Result<(), String> {
        self.update_config(|config| {
            session::pause_session(config, PauseReason::Manual, session::current_timestamp());
            Ok(())
        })
    }

    pub fn resume(&self) -> Result<(), String> {
        self.update_config(|config| {
            session::resume_session(config, session::current_timestamp());
            Ok(())
        })
    }

    pub fn adjust_time(&self, delta_minutes: i64) -> Result<AdminSessionSnapshot, String> {
        validate_adjustment_minutes(delta_minutes)?;
        let delta_seconds = delta_minutes
            .checked_mul(60)
            .ok_or_else(|| "Time adjustment overflowed".to_string())?;

        self.store.update(|config| {
            session::adjust_remaining_seconds(config, delta_seconds, session::current_timestamp())
                .ok_or_else(|| "Only active, non-expired sessions can be adjusted".to_string())?;
            Ok(build_admin_session_snapshot(config.clone()))
        })
    }

    pub fn update_settings(&self, update: SettingsUpdate) -> Result<AdminSessionSnapshot, String> {
        self.store.update(|config| {
            if let Some(timeout_minutes) = update.timeout_minutes {
                validate_timeout_minutes(timeout_minutes)?;
                config.timeout_minutes = timeout_minutes;
            }

            if let Some(warning_minutes) = update.warning_minutes {
                validate_warning_minutes(warning_minutes)?;
                config.warning_minutes = warning_minutes;
            }

            if let Some(action) = update.action {
                validate_action(&action)?;
                config.action = action;
            }

            if let Some(autostart_enabled) = update.autostart_enabled {
                config.autostart_enabled = autostart_enabled;
            }

            if let Some(remote_admin_enabled) = update.remote_admin_enabled {
                config.remote_admin_enabled = remote_admin_enabled;
            }

            Ok(build_admin_session_snapshot(config.clone()))
        })
    }

    pub fn remaining_seconds(&self) -> Result<Option<u64>, String> {
        Ok(self.snapshot()?.remaining_seconds)
    }

    pub fn mark_warning_notification_sent(&self) -> Result<(), String> {
        self.update_config(|config| {
            session::mark_warning_notification_sent(config);
            Ok(())
        })
    }

    pub fn execute_expired_action(&self) -> Result<ExpiredActionStatus, String> {
        let action = self.store.update(|config| {
            if config.timer_paused_at.is_some()
                || session::get_remaining_seconds(config) != Some(0)
                || !session::expire_session(config)
            {
                return Ok(None);
            }
            Ok(Some(config.action.clone()))
        })?;
        let Some(action) = action else {
            return Ok(ExpiredActionStatus::NoActionNeeded);
        };
        match execute_action(&action) {
            Ok(()) => Ok(ExpiredActionStatus::ActionStarted),
            Err(_) => Ok(ExpiredActionStatus::LockedOnFailure),
        }
    }

    pub fn remote_admin_enabled(&self) -> Result<bool, String> {
        Ok(self.store.load()?.remote_admin_enabled)
    }

    pub fn apply_signal(&self, signal: SessionSignal) -> Result<(), String> {
        if signal == SessionSignal::None {
            return Ok(());
        }

        self.update_config(|config| {
            session::apply_signal(config, signal, session::current_timestamp());
            Ok(())
        })
    }

    pub fn apply_startup_policy(&self, is_autostart_launch: bool) -> Result<(), String> {
        self.update_config(|config| {
            match session::decide_startup_action(config, is_autostart_launch) {
                StartupAction::None => {}
                StartupAction::Start => {
                    session::start_session(config, session::current_timestamp())
                }
                StartupAction::Resume => {
                    session::resume_session(config, session::current_timestamp());
                }
            }
            Ok(())
        })
    }

    fn update_config(
        &self,
        mutator: impl FnOnce(&mut AppConfig) -> Result<(), String>,
    ) -> Result<(), String> {
        self.store.update(mutator)
    }
}

impl From<AdminSessionSnapshot> for FrontendConfig {
    fn from(snapshot: AdminSessionSnapshot) -> Self {
        Self {
            timeout_minutes: snapshot.timeout_minutes,
            warning_minutes: snapshot.warning_minutes,
            action: snapshot.action,
            autostart_enabled: snapshot.autostart_enabled,
            first_run_complete: snapshot.first_run_complete,
            session_start_pending: snapshot.session_start_pending,
            timer_start_timestamp: snapshot.timer_start_timestamp,
            timer_paused_at: snapshot.timer_paused_at,
            pause_reason: snapshot.pause_reason,
            session_expired: snapshot.session_expired,
            warning_notification_sent: snapshot.warning_notification_sent,
        }
    }
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

fn validate_adjustment_minutes(delta_minutes: i64) -> Result<(), String> {
    if (-180..=180).contains(&delta_minutes) {
        Ok(())
    } else {
        Err("Time adjustment must be between -180 and 180 minutes".to_string())
    }
}

fn session_state_for(config: &AppConfig) -> AdminSessionState {
    if !config.first_run_complete {
        AdminSessionState::Setup
    } else if config.timer_start_timestamp.is_none() {
        AdminSessionState::Unlocked
    } else if config.session_expired {
        AdminSessionState::Expired
    } else if config.timer_paused_at.is_some() {
        AdminSessionState::Paused
    } else {
        AdminSessionState::Locked
    }
}

fn build_admin_session_snapshot(config: AppConfig) -> AdminSessionSnapshot {
    AdminSessionSnapshot {
        session_state: session_state_for(&config),
        remaining_seconds: session::get_remaining_seconds(&config),
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
