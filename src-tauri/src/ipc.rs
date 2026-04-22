use crate::device_controller::{
    AdminSessionSnapshot, ExpiredActionStatus, FrontendConfig, SettingsUpdate,
};
use crate::remote_admin::AdminPanelInfo;
use crate::service_main::ServiceState;
use crate::session::SessionSignal;
use axum::extract::State;
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthToken {
    pub token: String,
    pub expires_at: u64,
}

#[derive(Debug)]
pub struct ServiceClientError {
    pub status: Option<u16>,
    pub message: String,
}

#[derive(Clone)]
pub struct ServiceClient {
    base_url: String,
    client: Client,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiError {
    error: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetupPasswordRequest {
    password: String,
    timeout_minutes: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalLoginRequest {
    secret: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifyRecoveryKeyRequest {
    key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResetPasswordRequest {
    key: String,
    new_password: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangePasswordRequest {
    current: String,
    new_password: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdjustTimeRequest {
    delta_minutes: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartupPolicyRequest {
    is_autostart_launch: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignalRequest {
    signal: SessionSignal,
}

pub fn build_ipc_router(state: ServiceState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/snapshot", get(snapshot))
        .route("/config", get(config))
        .route("/remaining-seconds", get(remaining_seconds))
        .route("/first-run", get(first_run))
        .route("/admin-panel-info", get(admin_panel_info))
        .route("/setup-password", post(setup_password))
        .route("/finish-setup", post(finish_setup))
        .route("/local-login", post(local_login))
        .route("/verify-recovery-key", post(verify_recovery_key))
        .route(
            "/reset-password-with-recovery",
            post(reset_password_with_recovery),
        )
        .route("/change-password", post(change_password))
        .route("/start-timer", post(start_timer))
        .route("/unlock", post(unlock))
        .route("/relock", post(relock))
        .route("/pause", post(pause))
        .route("/resume", post(resume))
        .route("/adjust-time", post(adjust_time))
        .route("/update-settings", post(update_settings))
        .route(
            "/mark-warning-notification-sent",
            post(mark_warning_notification_sent),
        )
        .route("/execute-expired-action", post(execute_expired_action))
        .route("/persist-signal", post(persist_signal))
        .route("/apply-startup-policy", post(apply_startup_policy))
        .with_state(state)
}

impl ServiceClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|error| error.to_string())?;

        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client,
        })
    }

    pub fn default_local() -> Result<Self, String> {
        let base_url = std::env::var("SESSIONIZER_SERVICE_IPC_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:47770".to_string());
        Self::new(base_url)
    }

    pub fn frontend_config(&self) -> Result<FrontendConfig, ServiceClientError> {
        self.request(Method::GET, "/config", Option::<&()>::None, None)
    }

    pub fn snapshot(&self) -> Result<AdminSessionSnapshot, ServiceClientError> {
        self.request(Method::GET, "/snapshot", Option::<&()>::None, None)
    }

    pub fn remaining_seconds(&self) -> Result<Option<u64>, ServiceClientError> {
        self.request(Method::GET, "/remaining-seconds", Option::<&()>::None, None)
    }

    pub fn is_first_run(&self) -> Result<bool, ServiceClientError> {
        self.request(Method::GET, "/first-run", Option::<&()>::None, None)
    }

    pub fn admin_panel_info(&self) -> Result<AdminPanelInfo, ServiceClientError> {
        self.request(Method::GET, "/admin-panel-info", Option::<&()>::None, None)
    }

    pub fn setup_password(
        &self,
        password: String,
        timeout_minutes: u64,
    ) -> Result<String, ServiceClientError> {
        self.request(
            Method::POST,
            "/setup-password",
            Some(&SetupPasswordRequest {
                password,
                timeout_minutes,
            }),
            None,
        )
    }

    pub fn finish_setup(&self) -> Result<(), ServiceClientError> {
        self.request(Method::POST, "/finish-setup", Option::<&()>::None, None)
    }

    pub fn local_login(&self, secret: &str) -> Result<AuthToken, ServiceClientError> {
        self.request(
            Method::POST,
            "/local-login",
            Some(&LocalLoginRequest {
                secret: secret.to_string(),
            }),
            None,
        )
    }

    pub fn verify_recovery_key(&self, key: String) -> Result<bool, ServiceClientError> {
        self.request(
            Method::POST,
            "/verify-recovery-key",
            Some(&VerifyRecoveryKeyRequest { key }),
            None,
        )
    }

    pub fn reset_password_with_recovery(
        &self,
        key: String,
        new_password: String,
    ) -> Result<bool, ServiceClientError> {
        self.request(
            Method::POST,
            "/reset-password-with-recovery",
            Some(&ResetPasswordRequest { key, new_password }),
            None,
        )
    }

    pub fn change_password(
        &self,
        current: String,
        new_password: String,
        token: &str,
    ) -> Result<bool, ServiceClientError> {
        self.request(
            Method::POST,
            "/change-password",
            Some(&ChangePasswordRequest {
                current,
                new_password,
            }),
            Some(token),
        )
    }

    pub fn start_timer(&self) -> Result<(), ServiceClientError> {
        self.request(Method::POST, "/start-timer", Option::<&()>::None, None)
    }

    pub fn unlock(&self, token: &str) -> Result<(), ServiceClientError> {
        self.request(Method::POST, "/unlock", Option::<&()>::None, Some(token))
    }

    pub fn relock(&self) -> Result<(), ServiceClientError> {
        self.request(Method::POST, "/relock", Option::<&()>::None, None)
    }

    pub fn pause(&self, token: &str) -> Result<(), ServiceClientError> {
        self.request(Method::POST, "/pause", Option::<&()>::None, Some(token))
    }

    pub fn resume(&self) -> Result<(), ServiceClientError> {
        self.request(Method::POST, "/resume", Option::<&()>::None, None)
    }

    pub fn adjust_time(
        &self,
        delta_minutes: i64,
        token: &str,
    ) -> Result<AdminSessionSnapshot, ServiceClientError> {
        self.request(
            Method::POST,
            "/adjust-time",
            Some(&AdjustTimeRequest { delta_minutes }),
            Some(token),
        )
    }

    pub fn update_settings(
        &self,
        update: SettingsUpdate,
        token: &str,
    ) -> Result<AdminSessionSnapshot, ServiceClientError> {
        self.request(Method::POST, "/update-settings", Some(&update), Some(token))
    }

    pub fn mark_warning_notification_sent(&self) -> Result<(), ServiceClientError> {
        self.request(
            Method::POST,
            "/mark-warning-notification-sent",
            Option::<&()>::None,
            None,
        )
    }

    pub fn execute_expired_action(&self) -> Result<ExpiredActionStatus, ServiceClientError> {
        self.request(
            Method::POST,
            "/execute-expired-action",
            Option::<&()>::None,
            None,
        )
    }

    pub fn persist_signal(&self, signal: SessionSignal) -> Result<(), ServiceClientError> {
        self.request(
            Method::POST,
            "/persist-signal",
            Some(&SignalRequest { signal }),
            None,
        )
    }

    pub fn apply_startup_policy(
        &self,
        is_autostart_launch: bool,
    ) -> Result<(), ServiceClientError> {
        self.request(
            Method::POST,
            "/apply-startup-policy",
            Some(&StartupPolicyRequest {
                is_autostart_launch,
            }),
            None,
        )
    }

    fn request<T, B>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        token: Option<&str>,
    ) -> Result<T, ServiceClientError>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let mut request = self
            .client
            .request(method, format!("{}{}", self.base_url, path));

        if let Some(token) = token {
            request = request.header(AUTHORIZATION.as_str(), format!("Bearer {token}"));
        }

        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request.send().map_err(|error| ServiceClientError {
            status: None,
            message: error.to_string(),
        })?;

        if response.status() == StatusCode::NO_CONTENT {
            return serde_json::from_value(serde_json::Value::Null).map_err(|error| {
                ServiceClientError {
                    status: Some(StatusCode::NO_CONTENT.as_u16()),
                    message: error.to_string(),
                }
            });
        }

        let status = response.status();
        let payload = response.text().map_err(|error| ServiceClientError {
            status: Some(status.as_u16()),
            message: error.to_string(),
        })?;

        if !status.is_success() {
            let message = serde_json::from_str::<ApiError>(&payload)
                .map(|error| error.error)
                .unwrap_or(payload);
            return Err(ServiceClientError {
                status: Some(status.as_u16()),
                message,
            });
        }

        serde_json::from_str(&payload).map_err(|error| ServiceClientError {
            status: Some(status.as_u16()),
            message: error.to_string(),
        })
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

fn require_local_auth(headers: &HeaderMap, state: &ServiceState) -> Result<String, String> {
    let token = extract_token(headers)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Local authorization required".to_string())?;

    if state.auth.authorize_local(token) {
        Ok(token.to_string())
    } else {
        Err("Local authorization expired or is invalid".to_string())
    }
}

async fn health(State(state): State<ServiceState>) -> Response {
    match state.controller.snapshot() {
        Ok(snapshot) => Json(snapshot).into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn snapshot(State(state): State<ServiceState>) -> Response {
    match state.controller.snapshot() {
        Ok(snapshot) => Json(snapshot).into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn config(State(state): State<ServiceState>) -> Response {
    match state.controller.frontend_config() {
        Ok(config) => Json(config).into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn remaining_seconds(State(state): State<ServiceState>) -> Response {
    match state.controller.remaining_seconds() {
        Ok(remaining_seconds) => Json(remaining_seconds).into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn first_run(State(state): State<ServiceState>) -> Response {
    match state.controller.is_first_run() {
        Ok(is_first_run) => Json(is_first_run).into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn admin_panel_info(State(state): State<ServiceState>) -> Response {
    Json(state.auth.panel_snapshot()).into_response()
}

async fn setup_password(
    State(state): State<ServiceState>,
    Json(payload): Json<SetupPasswordRequest>,
) -> Response {
    match state
        .controller
        .setup_password(payload.password, payload.timeout_minutes)
    {
        Ok(recovery_key) => Json(recovery_key).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn finish_setup(State(state): State<ServiceState>) -> Response {
    match state.controller.finish_setup() {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn local_login(
    State(state): State<ServiceState>,
    Json(payload): Json<LocalLoginRequest>,
) -> Response {
    if payload.secret.trim().is_empty() {
        return json_error(StatusCode::UNAUTHORIZED, "Password is required");
    }

    match state.controller.verify_local_unlock(&payload.secret) {
        Ok(true) => {
            let (token, expires_at) = state.auth.issue_local_session();
            Json(AuthToken { token, expires_at }).into_response()
        }
        Ok(false) => json_error(StatusCode::UNAUTHORIZED, "Incorrect password"),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn verify_recovery_key(
    State(state): State<ServiceState>,
    Json(payload): Json<VerifyRecoveryKeyRequest>,
) -> Response {
    match state.controller.verify_recovery_key(payload.key) {
        Ok(valid) => Json(valid).into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn reset_password_with_recovery(
    State(state): State<ServiceState>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Response {
    match state
        .controller
        .reset_password_with_recovery(payload.key, payload.new_password)
    {
        Ok(changed) => Json(changed).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn change_password(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(payload): Json<ChangePasswordRequest>,
) -> Response {
    if let Err(message) = require_local_auth(&headers, &state) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match state
        .controller
        .change_password(payload.current, payload.new_password)
    {
        Ok(changed) => Json(changed).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn start_timer(State(state): State<ServiceState>) -> Response {
    match state.controller.start_timer() {
        Ok(()) => {
            state.notify_state_changed();
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn unlock(State(state): State<ServiceState>, headers: HeaderMap) -> Response {
    if let Err(message) = require_local_auth(&headers, &state) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match state.controller.unlock() {
        Ok(()) => {
            state.notify_state_changed();
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn relock(State(state): State<ServiceState>) -> Response {
    match state.controller.relock() {
        Ok(()) => {
            state.notify_state_changed();
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn pause(State(state): State<ServiceState>, headers: HeaderMap) -> Response {
    if let Err(message) = require_local_auth(&headers, &state) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match state.controller.pause() {
        Ok(()) => {
            state.notify_state_changed();
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn resume(State(state): State<ServiceState>) -> Response {
    match state.controller.resume() {
        Ok(()) => {
            state.notify_state_changed();
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn adjust_time(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(payload): Json<AdjustTimeRequest>,
) -> Response {
    if let Err(message) = require_local_auth(&headers, &state) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match state.controller.adjust_time(payload.delta_minutes) {
        Ok(snapshot) => {
            state.notify_state_changed();
            Json(snapshot).into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn update_settings(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(payload): Json<SettingsUpdate>,
) -> Response {
    if let Err(message) = require_local_auth(&headers, &state) {
        return json_error(StatusCode::UNAUTHORIZED, message);
    }

    match state.controller.update_settings(payload) {
        Ok(snapshot) => {
            state.notify_state_changed();
            Json(snapshot).into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn mark_warning_notification_sent(State(state): State<ServiceState>) -> Response {
    match state.controller.mark_warning_notification_sent() {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn execute_expired_action(State(state): State<ServiceState>) -> Response {
    match state.controller.execute_expired_action() {
        Ok(status) => {
            state.notify_state_changed();
            Json(status).into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn persist_signal(
    State(state): State<ServiceState>,
    Json(payload): Json<SignalRequest>,
) -> Response {
    match state.controller.apply_signal(payload.signal) {
        Ok(()) => {
            state.notify_state_changed();
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn apply_startup_policy(
    State(state): State<ServiceState>,
    Json(payload): Json<StartupPolicyRequest>,
) -> Response {
    match state
        .controller
        .apply_startup_policy(payload.is_autostart_launch)
    {
        Ok(()) => {
            state.notify_state_changed();
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}
