use crate::device_controller::SettingsUpdate;
pub use crate::device_controller::{AdminSessionSnapshot, ExpiredActionStatus, FrontendConfig};
use crate::ipc::{ServiceClient, ServiceClientError};
use crate::remote_admin::AdminPanelInfo;
use crate::session::SessionSignal;
use std::sync::{Mutex, OnceLock};

static CLIENT: OnceLock<ServiceClient> = OnceLock::new();
static LOCAL_AUTH_TOKEN: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn client() -> Result<&'static ServiceClient, String> {
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }

    let created = ServiceClient::default_local()?;
    let _ = CLIENT.set(created);
    CLIENT
        .get()
        .ok_or_else(|| "Failed to initialize service client".to_string())
}

fn local_auth_token() -> &'static Mutex<Option<String>> {
    LOCAL_AUTH_TOKEN.get_or_init(|| Mutex::new(None))
}

fn set_local_auth_token(token: Option<String>) {
    *local_auth_token()
        .lock()
        .expect("local auth token lock poisoned") = token;
}

fn with_local_auth<T>(
    action: impl FnOnce(&ServiceClient, &str) -> Result<T, ServiceClientError>,
) -> Result<T, String> {
    let token = local_auth_token()
        .lock()
        .expect("local auth token lock poisoned")
        .clone()
        .ok_or_else(|| "Local authorization required".to_string())?;

    match action(client()?, &token) {
        Ok(value) => Ok(value),
        Err(error) => {
            if error.status == Some(401) {
                set_local_auth_token(None);
            }
            Err(format_service_error(error))
        }
    }
}

fn format_service_error(error: ServiceClientError) -> String {
    match error.status {
        Some(status) => format!("Service request failed ({status}): {}", error.message),
        None => error.message,
    }
}

pub fn get_config() -> Result<FrontendConfig, String> {
    client()?.frontend_config().map_err(format_service_error)
}

pub fn get_admin_session_snapshot() -> Result<AdminSessionSnapshot, String> {
    client()?.snapshot().map_err(format_service_error)
}

pub fn get_admin_panel_info() -> Result<AdminPanelInfo, String> {
    client()?.admin_panel_info().map_err(format_service_error)
}

pub fn finish_setup() -> Result<(), String> {
    client()?.finish_setup().map_err(format_service_error)
}

pub fn update_settings(
    timeout_minutes: u64,
    warning_minutes: u64,
    action: String,
    autostart_enabled: bool,
) -> Result<(), String> {
    with_local_auth(|client, token| {
        client.update_settings(
            SettingsUpdate {
                timeout_minutes: Some(timeout_minutes),
                warning_minutes: Some(warning_minutes),
                action: Some(action),
                autostart_enabled: Some(autostart_enabled),
                remote_admin_enabled: None,
            },
            token,
        )
    })
    .map(|_| ())
}

pub fn is_first_run() -> Result<bool, String> {
    client()?.is_first_run().map_err(format_service_error)
}

pub fn setup_password(password: String, timeout_minutes: u64) -> Result<String, String> {
    client()?
        .setup_password(password, timeout_minutes)
        .map_err(format_service_error)
}

pub fn verify_password(password: String) -> Result<bool, String> {
    match client()?.local_login(&password) {
        Ok(auth) => {
            set_local_auth_token(Some(auth.token));
            Ok(true)
        }
        Err(error) if error.status == Some(401) => {
            set_local_auth_token(None);
            Ok(false)
        }
        Err(error) => Err(format_service_error(error)),
    }
}

pub fn verify_recovery_key(key: String) -> Result<bool, String> {
    client()?
        .verify_recovery_key(key)
        .map_err(format_service_error)
}

pub fn reset_password_with_recovery(key: String, new_password: String) -> Result<bool, String> {
    client()?
        .reset_password_with_recovery(key, new_password)
        .map_err(format_service_error)
}

pub fn change_password(current: String, new_password: String) -> Result<bool, String> {
    with_local_auth(|client, token| client.change_password(current, new_password, token))
}

pub fn start_timer() -> Result<(), String> {
    client()?.start_timer().map_err(format_service_error)
}

pub fn unlock_session() -> Result<(), String> {
    with_local_auth(|client, token| client.unlock(token))
}

#[allow(dead_code)]
pub fn relock_session() -> Result<(), String> {
    client()?.relock().map_err(format_service_error)
}

pub fn pause_timer() -> Result<(), String> {
    with_local_auth(|client, token| client.pause(token))
}

pub fn resume_timer() -> Result<(), String> {
    client()?.resume().map_err(format_service_error)
}

pub fn get_remaining_seconds() -> Result<Option<u64>, String> {
    client()?.remaining_seconds().map_err(format_service_error)
}

pub fn mark_warning_notification_sent() -> Result<(), String> {
    client()?
        .mark_warning_notification_sent()
        .map_err(format_service_error)
}

pub fn execute_expired_action() -> Result<ExpiredActionStatus, String> {
    client()?
        .execute_expired_action()
        .map_err(format_service_error)
}

pub fn apply_startup_policy(is_autostart_launch: bool) -> Result<(), String> {
    client()?
        .apply_startup_policy(is_autostart_launch)
        .map_err(format_service_error)
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn persist_signal(signal: SessionSignal) -> Result<(), String> {
    client()?
        .persist_signal(signal)
        .map_err(format_service_error)
}
