use crate::device_controller::{AdminSessionSnapshot, SettingsUpdate};
use crate::service_main::{RemoteAsset, ServiceState};
use axum::extract::State;
use axum::http::header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
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
            listen_address: "0.0.0.0".to_string(),
            port: 47_771,
            urls: Vec::new(),
            error: None,
        }
    }
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
    #[serde(default)]
    remote_admin_enabled: Option<bool>,
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

pub fn build_remote_admin_router(state: ServiceState) -> Router {
    Router::new()
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
        .with_state(state)
}

pub(crate) fn admin_urls(bound_address: SocketAddr) -> Vec<String> {
    let mut urls = vec![format!("http://127.0.0.1:{}", bound_address.port())];

    if !bound_address.ip().is_loopback() {
        urls[0] = format!("http://{}:{}", bound_address.ip(), bound_address.port());
    }

    if let Ok(socket) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) {
        if socket.connect((Ipv4Addr::new(1, 1, 1, 1), 80)).is_ok() {
            if let Ok(address) = socket.local_addr() {
                let ip = address.ip();
                if !ip.is_unspecified() && !ip.is_loopback() {
                    urls.push(format!("http://{ip}:{}", bound_address.port()));
                }
            }
        }
    }

    urls.sort();
    urls.dedup();
    urls
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

fn set_default_headers(headers: &mut HeaderMap) {
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
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

fn asset_response(asset: RemoteAsset) -> Response {
    let mut response = Response::new(asset.bytes.into());
    *response.status_mut() = StatusCode::OK;

    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&asset.mime_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    set_default_headers(headers);

    if let Some(csp) = asset.csp_header {
        if let Ok(value) = HeaderValue::from_str(&csp) {
            headers.insert("content-security-policy", value);
        }
    }

    response
}

fn extract_token(headers: &HeaderMap) -> Option<&str> {
    let header = headers.get(AUTHORIZATION)?.to_str().ok()?;
    header.strip_prefix("Bearer ")?.trim().split(' ').next()
}

fn require_remote_auth(headers: &HeaderMap, state: &ServiceState) -> Result<String, String> {
    let token = extract_token(headers)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Authentication required".to_string())?;

    if state.auth.authorize_remote(token) {
        Ok(token.to_string())
    } else {
        Err("Admin session expired or is invalid".to_string())
    }
}

fn remote_admin_disabled_response(state: &ServiceState) -> Option<Response> {
    match state.controller.remote_admin_enabled() {
        Ok(true) => None,
        Ok(false) => Some(json_error(
            StatusCode::FORBIDDEN,
            "Remote admin is disabled",
        )),
        Err(error) => Some(json_error(StatusCode::INTERNAL_SERVER_ERROR, error)),
    }
}

fn session_response(session: AdminSessionSnapshot) -> Response {
    Json(SessionResponse {
        session,
        warning: None,
    })
    .into_response()
}

async fn serve_index(State(state): State<ServiceState>) -> Response {
    serve_asset_path(&state, "admin.html")
}

async fn serve_asset(
    State(state): State<ServiceState>,
    path: axum::extract::Path<String>,
) -> Response {
    let Some(path) = sanitize_asset_path(&path) else {
        return json_error(StatusCode::BAD_REQUEST, "Invalid asset path");
    };

    serve_asset_path(&state, &path)
}

fn serve_asset_path(state: &ServiceState, path: &str) -> Response {
    if !state.remote_assets_available() {
        return json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Remote admin panel assets are unavailable",
        );
    }

    match state.resolve_remote_asset(path) {
        Some(asset) => asset_response(asset),
        None => json_error(StatusCode::NOT_FOUND, "Asset not found"),
    }
}

async fn login(State(state): State<ServiceState>, Json(payload): Json<LoginRequest>) -> Response {
    if let Some(response) = remote_admin_disabled_response(&state) {
        return response;
    }

    if payload.password.trim().is_empty() {
        return json_error(StatusCode::UNAUTHORIZED, "Password is required");
    }

    if let Some(retry_after) = state.auth.remote_login_retry_after_seconds() {
        let mut response = json_error(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many failed logins. Try again later.",
        );
        if let Ok(value) = HeaderValue::from_str(&retry_after.to_string()) {
            response.headers_mut().insert("retry-after", value);
        }
        return response;
    }

    match state.controller.admin_login(&payload.password) {
        Ok(true) => match state.controller.snapshot() {
            Ok(session) => {
                state.auth.clear_failed_remote_logins();
                let (token, expires_at) = state.auth.issue_remote_session();
                Json(LoginResponse {
                    token,
                    expires_at,
                    session,
                    admin_panel: state.auth.panel_snapshot(),
                })
                .into_response()
            }
            Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
        },
        Ok(false) => {
            state.auth.record_failed_remote_login();
            json_error(StatusCode::UNAUTHORIZED, "Incorrect password")
        }
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn logout(State(state): State<ServiceState>, headers: HeaderMap) -> Response {
    if let Some(response) = remote_admin_disabled_response(&state) {
        return response;
    }

    match require_remote_auth(&headers, &state) {
        Ok(token) => {
            state.auth.revoke_remote(&token);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(message) => json_error(StatusCode::UNAUTHORIZED, message),
    }
}

async fn get_state(State(state): State<ServiceState>, headers: HeaderMap) -> Response {
    if let Some(response) = remote_admin_disabled_response(&state) {
        return response;
    }

    if let Err(message) = require_remote_auth(&headers, &state) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match state.controller.snapshot() {
        Ok(session) => session_response(session),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn pause_session(State(state): State<ServiceState>, headers: HeaderMap) -> Response {
    if let Some(response) = remote_admin_disabled_response(&state) {
        return response;
    }

    if let Err(message) = require_remote_auth(&headers, &state) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match state
        .controller
        .pause()
        .and_then(|_| state.controller.snapshot())
    {
        Ok(session) => {
            state.notify_state_changed();
            session_response(session)
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn resume_session(State(state): State<ServiceState>, headers: HeaderMap) -> Response {
    if let Some(response) = remote_admin_disabled_response(&state) {
        return response;
    }

    if let Err(message) = require_remote_auth(&headers, &state) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match state
        .controller
        .resume()
        .and_then(|_| state.controller.snapshot())
    {
        Ok(session) => {
            state.notify_state_changed();
            session_response(session)
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn unlock_session(State(state): State<ServiceState>, headers: HeaderMap) -> Response {
    if let Some(response) = remote_admin_disabled_response(&state) {
        return response;
    }

    if let Err(message) = require_remote_auth(&headers, &state) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match state
        .controller
        .unlock()
        .and_then(|_| state.controller.snapshot())
    {
        Ok(session) => {
            state.notify_state_changed();
            session_response(session)
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn relock_session(State(state): State<ServiceState>, headers: HeaderMap) -> Response {
    if let Some(response) = remote_admin_disabled_response(&state) {
        return response;
    }

    if let Err(message) = require_remote_auth(&headers, &state) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match state
        .controller
        .relock()
        .and_then(|_| state.controller.snapshot())
    {
        Ok(session) => {
            state.notify_state_changed();
            session_response(session)
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn adjust_time(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(payload): Json<AdjustTimeRequest>,
) -> Response {
    if let Some(response) = remote_admin_disabled_response(&state) {
        return response;
    }

    if let Err(message) = require_remote_auth(&headers, &state) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match state.controller.adjust_time(payload.delta_minutes) {
        Ok(session) => {
            state.notify_state_changed();
            session_response(session)
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn update_settings(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(payload): Json<SettingsRequest>,
) -> Response {
    if let Some(response) = remote_admin_disabled_response(&state) {
        return response;
    }

    if let Err(message) = require_remote_auth(&headers, &state) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match state.controller.update_settings(SettingsUpdate {
        timeout_minutes: Some(payload.timeout_minutes),
        warning_minutes: Some(payload.warning_minutes),
        action: Some(payload.action),
        autostart_enabled: Some(payload.autostart_enabled),
        remote_admin_enabled: payload.remote_admin_enabled,
    }) {
        Ok(session) => {
            state.notify_state_changed();
            session_response(session)
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}
