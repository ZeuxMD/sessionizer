use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tauri_plugin_autostart::ManagerExt as _;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};

use crate::control::{self, AdminSessionSnapshot};
use crate::log_error;

const ADMIN_PANEL_PORT: u16 = 47_771;
const ADMIN_PANEL_LISTEN_ADDRESS: &str = "0.0.0.0";
const SESSION_TTL_SECONDS: u64 = 12 * 60 * 60;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct AdminPanelInfo {
    pub running: bool,
    pub listen_address: String,
    pub port: u16,
    pub urls: Vec<String>,
    pub error: Option<String>,
}

impl Default for AdminPanelInfo {
    fn default() -> Self {
        Self {
            running: false,
            listen_address: ADMIN_PANEL_LISTEN_ADDRESS.to_string(),
            port: ADMIN_PANEL_PORT,
            urls: Vec::new(),
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RemoteAdminState {
    sessions: Arc<Mutex<HashMap<String, u64>>>,
    panel_info: Arc<Mutex<AdminPanelInfo>>,
}

impl Default for RemoteAdminState {
    fn default() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            panel_info: Arc::new(Mutex::new(AdminPanelInfo::default())),
        }
    }
}

#[derive(Clone)]
struct HttpState {
    app_handle: AppHandle,
    remote_admin: RemoteAdminState,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginRequest {
    password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdjustTimeRequest {
    delta_minutes: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsRequest {
    timeout_minutes: u64,
    warning_minutes: u64,
    action: String,
    autostart_enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiError {
    error: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    session: AdminSessionSnapshot,
    warning: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponse {
    token: String,
    expires_at: u64,
    session: AdminSessionSnapshot,
    admin_panel: AdminPanelInfo,
}

impl RemoteAdminState {
    pub fn snapshot(&self) -> AdminPanelInfo {
        self.panel_info
            .lock()
            .expect("admin panel info lock poisoned")
            .clone()
    }

    fn issue_session_at(&self, now: u64) -> (String, u64) {
        let mut sessions = self.sessions.lock().expect("admin session lock poisoned");
        sessions.retain(|_, expires_at| *expires_at > now);

        let token = Alphanumeric.sample_string(&mut rand::thread_rng(), 48);
        let expires_at = now.saturating_add(SESSION_TTL_SECONDS);
        sessions.insert(token.clone(), expires_at);
        (token, expires_at)
    }

    fn is_authorized_at(&self, token: &str, now: u64) -> bool {
        let mut sessions = self.sessions.lock().expect("admin session lock poisoned");
        sessions.retain(|_, expires_at| *expires_at > now);
        matches!(sessions.get(token), Some(expires_at) if *expires_at > now)
    }

    fn revoke(&self, token: &str) {
        let mut sessions = self.sessions.lock().expect("admin session lock poisoned");
        sessions.remove(token);
    }

    fn mark_running(&self, urls: Vec<String>) {
        let mut info = self
            .panel_info
            .lock()
            .expect("admin panel info lock poisoned");
        info.running = true;
        info.urls = urls;
        info.error = None;
    }

    fn mark_error(&self, message: String) {
        let mut info = self
            .panel_info
            .lock()
            .expect("admin panel info lock poisoned");
        info.running = false;
        info.urls.clear();
        info.error = Some(message);
    }
}

fn json_error(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(ApiError {
            error: message.into(),
        }),
    )
        .into_response()
}

fn extract_token(headers: &HeaderMap) -> Option<&str> {
    let header = headers.get(AUTHORIZATION)?.to_str().ok()?;
    header.strip_prefix("Bearer ")?.trim().split(' ').next()
}

fn require_auth(headers: &HeaderMap, remote_admin: &RemoteAdminState) -> Result<String, String> {
    let token = extract_token(headers)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Authentication required".to_string())?;

    if remote_admin.is_authorized_at(token, crate::session::current_timestamp()) {
        Ok(token.to_string())
    } else {
        Err("Admin session expired or is invalid".to_string())
    }
}

fn set_default_headers(headers: &mut HeaderMap) {
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
}

fn admin_urls(port: u16) -> Vec<String> {
    let mut urls = vec![format!("http://127.0.0.1:{port}")];

    if let Ok(socket) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) {
        if socket.connect((Ipv4Addr::new(1, 1, 1, 1), 80)).is_ok() {
            if let Ok(address) = socket.local_addr() {
                let ip = address.ip();
                if !ip.is_unspecified() && !ip.is_loopback() {
                    urls.push(format!("http://{ip}:{port}"));
                }
            }
        }
    }

    urls
}

fn sync_autostart_setting(app_handle: &AppHandle, enabled: bool) -> Option<String> {
    let result = if enabled {
        app_handle.autolaunch().enable()
    } else {
        app_handle.autolaunch().disable()
    };

    match result {
        Ok(()) => None,
        Err(error) => {
            let message = if enabled {
                "Settings were saved, but Sessionizer could not enable Start with Windows."
            } else {
                "Settings were saved, but Sessionizer could not disable Start with Windows."
            };
            log_error(&format!("Failed to sync autostart setting: {error}"));
            Some(message.to_string())
        }
    }
}

pub fn emit_runtime_state_changed(app_handle: &AppHandle) {
    let _ = app_handle.emit("runtime-state-changed", ());
}

fn asset_response(app_handle: &AppHandle, path: &str) -> Response {
    let Some(asset) = app_handle.asset_resolver().get(path.to_string()) else {
        return json_error(StatusCode::NOT_FOUND, "Asset not found");
    };

    let mime_type = asset.mime_type().to_string();
    let csp_header = asset.csp_header().map(ToOwned::to_owned);
    let mut response = Response::new(asset.bytes.into());
    *response.status_mut() = StatusCode::OK;

    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&mime_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    set_default_headers(headers);

    if let Some(csp) = csp_header {
        if let Ok(value) = HeaderValue::from_str(&csp) {
            headers.insert("content-security-policy", value);
        }
    }

    response
}

fn sanitize_asset_path(path: &str) -> Option<String> {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return Some("admin.html".to_string());
    }

    let mut segments = Vec::new();
    for segment in trimmed.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return None;
        }
        segments.push(segment);
    }

    Some(segments.join("/"))
}

fn session_response(session: AdminSessionSnapshot, warning: Option<String>) -> Response {
    let mut response = Json(SessionResponse { session, warning }).into_response();
    set_default_headers(response.headers_mut());
    response
}

async fn login(State(state): State<HttpState>, Json(payload): Json<LoginRequest>) -> Response {
    if payload.password.trim().is_empty() {
        return json_error(StatusCode::UNAUTHORIZED, "Password is required");
    }

    match control::verify_password(payload.password) {
        Ok(true) => match control::get_admin_session_snapshot() {
            Ok(session) => {
                let (token, expires_at) = state
                    .remote_admin
                    .issue_session_at(crate::session::current_timestamp());
                let mut response = Json(LoginResponse {
                    token,
                    expires_at,
                    session,
                    admin_panel: state.remote_admin.snapshot(),
                })
                .into_response();
                set_default_headers(response.headers_mut());
                response
            }
            Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
        },
        Ok(false) => {
            sleep(Duration::from_millis(750)).await;
            json_error(StatusCode::UNAUTHORIZED, "Incorrect password")
        }
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn logout(State(state): State<HttpState>, headers: HeaderMap) -> Response {
    match require_auth(&headers, &state.remote_admin) {
        Ok(token) => {
            state.remote_admin.revoke(&token);
            let mut response = StatusCode::NO_CONTENT.into_response();
            set_default_headers(response.headers_mut());
            response
        }
        Err(message) => json_error(StatusCode::UNAUTHORIZED, message),
    }
}

async fn get_state(State(state): State<HttpState>, headers: HeaderMap) -> Response {
    if let Err(message) = require_auth(&headers, &state.remote_admin) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match control::get_admin_session_snapshot() {
        Ok(session) => session_response(session, None),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn pause_session(State(state): State<HttpState>, headers: HeaderMap) -> Response {
    if let Err(message) = require_auth(&headers, &state.remote_admin) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match control::pause_timer().and_then(|_| control::get_admin_session_snapshot()) {
        Ok(session) => {
            emit_runtime_state_changed(&state.app_handle);
            session_response(session, None)
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn resume_session(State(state): State<HttpState>, headers: HeaderMap) -> Response {
    if let Err(message) = require_auth(&headers, &state.remote_admin) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match control::resume_timer().and_then(|_| control::get_admin_session_snapshot()) {
        Ok(session) => {
            emit_runtime_state_changed(&state.app_handle);
            session_response(session, None)
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn unlock_session(State(state): State<HttpState>, headers: HeaderMap) -> Response {
    if let Err(message) = require_auth(&headers, &state.remote_admin) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match control::unlock_session().and_then(|_| control::get_admin_session_snapshot()) {
        Ok(session) => {
            emit_runtime_state_changed(&state.app_handle);
            session_response(session, None)
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn relock_session(State(state): State<HttpState>, headers: HeaderMap) -> Response {
    if let Err(message) = require_auth(&headers, &state.remote_admin) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match control::relock_session().and_then(|_| control::get_admin_session_snapshot()) {
        Ok(session) => {
            emit_runtime_state_changed(&state.app_handle);
            session_response(session, None)
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn adjust_time(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(payload): Json<AdjustTimeRequest>,
) -> Response {
    if let Err(message) = require_auth(&headers, &state.remote_admin) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match control::adjust_timer_minutes(payload.delta_minutes) {
        Ok(session) => {
            emit_runtime_state_changed(&state.app_handle);
            session_response(session, None)
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn update_settings(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(payload): Json<SettingsRequest>,
) -> Response {
    if let Err(message) = require_auth(&headers, &state.remote_admin) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match control::update_settings(
        payload.timeout_minutes,
        payload.warning_minutes,
        payload.action,
        payload.autostart_enabled,
    )
    .and_then(|_| control::get_admin_session_snapshot())
    {
        Ok(session) => {
            let warning = sync_autostart_setting(&state.app_handle, session.autostart_enabled);
            emit_runtime_state_changed(&state.app_handle);
            session_response(session, warning)
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn serve_index(State(state): State<HttpState>) -> Response {
    asset_response(&state.app_handle, "admin.html")
}

async fn serve_asset(Path(path): Path<String>, State(state): State<HttpState>) -> Response {
    let Some(path) = sanitize_asset_path(&path) else {
        return json_error(StatusCode::BAD_REQUEST, "Invalid asset path");
    };
    asset_response(&state.app_handle, &path)
}

pub fn start_server(app_handle: AppHandle, remote_admin: RemoteAdminState) {
    let shared_state = HttpState {
        app_handle: app_handle.clone(),
        remote_admin: remote_admin.clone(),
    };

    tauri::async_runtime::spawn(async move {
        let address = SocketAddr::from(([0, 0, 0, 0], ADMIN_PANEL_PORT));
        let listener = match TcpListener::bind(address).await {
            Ok(listener) => listener,
            Err(error) => {
                let message =
                    format!("Remote admin panel failed to bind on {ADMIN_PANEL_LISTEN_ADDRESS}:{ADMIN_PANEL_PORT}: {error}");
                remote_admin.mark_error(message.clone());
                log_error(&message);
                return;
            }
        };

        remote_admin.mark_running(admin_urls(ADMIN_PANEL_PORT));
        log_error(&format!(
            "Remote admin panel listening on {ADMIN_PANEL_LISTEN_ADDRESS}:{ADMIN_PANEL_PORT}"
        ));

        let app = Router::new()
            .route("/", get(serve_index))
            .route("/api/login", post(login))
            .route("/api/logout", post(logout))
            .route("/api/state", get(get_state))
            .route("/api/pause", post(pause_session))
            .route("/api/resume", post(resume_session))
            .route("/api/unlock", post(unlock_session))
            .route("/api/relock", post(relock_session))
            .route("/api/adjust-time", post(adjust_time))
            .route("/api/settings", put(update_settings))
            .route("/{*path}", get(serve_asset))
            .with_state(shared_state);

        if let Err(error) = axum::serve(listener, app).await {
            let message = format!("Remote admin panel stopped unexpectedly: {error}");
            remote_admin.mark_error(message.clone());
            log_error(&message);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issued_sessions_expire_after_ttl() {
        let state = RemoteAdminState::default();
        let (token, expires_at) = state.issue_session_at(1_000);

        assert!(state.is_authorized_at(&token, 1_000));
        assert_eq!(expires_at, 1_000 + SESSION_TTL_SECONDS);
        assert!(!state.is_authorized_at(&token, expires_at));
    }

    #[test]
    fn sanitize_asset_path_rejects_parent_segments() {
        assert_eq!(
            sanitize_asset_path("assets/admin.js"),
            Some("assets/admin.js".to_string())
        );
        assert_eq!(sanitize_asset_path("../secret"), None);
        assert_eq!(sanitize_asset_path(""), Some("admin.html".to_string()));
    }
}
