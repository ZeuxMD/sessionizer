use reqwest::blocking::Client as HttpClient;
use reqwest::StatusCode;
use serde::Deserialize;
use sessionizer_lib::device_controller::{AdminSessionState, DeviceController, SettingsUpdate};
use sessionizer_lib::ipc::{AuthToken, ServiceClient};
use sessionizer_lib::service_main::{
    RemoteAsset, RemoteAssetLoader, ServiceConfig, ServiceHandle, TimeSource,
};
use sessionizer_lib::state_store::StateStore;
use std::fs;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock drifted backwards")
        .as_nanos();

    std::env::temp_dir().join(format!("sessionizer-{label}-{unique}"))
}

fn time_source(now: &Arc<AtomicU64>) -> TimeSource {
    let now = now.clone();
    Arc::new(move || now.load(Ordering::SeqCst))
}

struct TestService {
    _handle: ServiceHandle,
    ipc_client: ServiceClient,
    http_client: HttpClient,
    remote_base_url: String,
    root: PathBuf,
}

impl TestService {
    fn start(now: Arc<AtomicU64>) -> Self {
        Self::start_with(now, |_| {})
    }

    fn start_with(now: Arc<AtomicU64>, configure: impl FnOnce(&mut ServiceConfig)) -> Self {
        let root = unique_dir("service-root");
        let controller = DeviceController::new(StateStore::new(root.clone(), None));
        let mut config = ServiceConfig::for_tests(controller, time_source(&now));
        configure(&mut config);
        let handle = ServiceHandle::spawn(config).expect("service should start");

        let ipc_client =
            ServiceClient::new(handle.ipc_base_url()).expect("ipc client should build");

        Self {
            remote_base_url: handle.remote_base_url(),
            _handle: handle,
            ipc_client,
            http_client: HttpClient::builder()
                .build()
                .expect("http client should build"),
            root,
        }
    }

    fn prime_locked_device(&self) {
        let recovery_key = self
            .ipc_client
            .setup_password("parent-secret".to_string(), 60)
            .expect("setup should succeed");
        assert!(!recovery_key.is_empty());
        self.ipc_client
            .finish_setup()
            .expect("finish setup should succeed");
        self.ipc_client
            .start_timer()
            .expect("start timer should succeed");
    }
}

fn test_remote_assets() -> RemoteAssetLoader {
    Arc::new(|path| {
        match path {
        "admin.html" => Some(RemoteAsset {
            bytes: br#"<!doctype html><html><head><title>Sessionizer Admin</title></head><body>admin shell</body></html>"#
                .to_vec(),
            mime_type: "text/html; charset=utf-8".to_string(),
            csp_header: Some("default-src 'self'".to_string()),
        }),
        "assets/admin.js" => Some(RemoteAsset {
            bytes: b"console.log('sessionizer-admin');".to_vec(),
            mime_type: "text/javascript; charset=utf-8".to_string(),
            csp_header: None,
        }),
        _ => None,
    }
    })
}

impl Drop for TestService {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponse {
    token: String,
    expires_at: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorResponse {
    error: String,
}

fn remote_login(service: &TestService, password: &str) -> reqwest::blocking::Response {
    service
        .http_client
        .post(format!("{}/api/login", service.remote_base_url))
        .json(&serde_json::json!({ "password": password }))
        .send()
        .expect("remote login request should complete")
}

#[test]
fn local_unlock_requires_validation_and_changes_next_snapshot() {
    let now = Arc::new(AtomicU64::new(10_000));
    let service = TestService::start(now);
    service.prime_locked_device();

    let unauthorized = service
        .ipc_client
        .unlock("missing-token")
        .expect_err("unlock should require local auth");
    assert_eq!(unauthorized.status, Some(StatusCode::UNAUTHORIZED.as_u16()));

    let bad_login = service
        .ipc_client
        .local_login("wrong-secret")
        .expect_err("wrong password should be rejected");
    assert_eq!(bad_login.status, Some(StatusCode::UNAUTHORIZED.as_u16()));

    let auth = service
        .ipc_client
        .local_login("parent-secret")
        .expect("correct password should return a local token");
    service
        .ipc_client
        .unlock(&auth.token)
        .expect("unlock should succeed with local auth");

    let snapshot = service
        .ipc_client
        .snapshot()
        .expect("snapshot should still be available");
    assert!(matches!(
        snapshot.session_state,
        AdminSessionState::Unlocked
    ));
}

#[test]
fn remote_login_then_unlock_updates_shared_controller() {
    let now = Arc::new(AtomicU64::new(20_000));
    let service = TestService::start(now);
    service.prime_locked_device();

    let login = remote_login(&service, "parent-secret");
    assert_eq!(login.status(), StatusCode::OK);
    let login: LoginResponse = login.json().expect("login response should parse");
    assert!(!login.token.is_empty());
    assert!(login.expires_at > 20_000);

    let response = service
        .http_client
        .post(format!("{}/api/unlock", service.remote_base_url))
        .bearer_auth(&login.token)
        .send()
        .expect("remote unlock should complete");
    assert_eq!(response.status(), StatusCode::OK);

    let snapshot = service
        .ipc_client
        .snapshot()
        .expect("shared controller snapshot should load");
    assert!(matches!(
        snapshot.session_state,
        AdminSessionState::Unlocked
    ));
}

#[test]
fn expired_remote_tokens_are_rejected() {
    let now = Arc::new(AtomicU64::new(30_000));
    let service = TestService::start(now.clone());
    service.prime_locked_device();

    let login = remote_login(&service, "parent-secret");
    assert_eq!(login.status(), StatusCode::OK);
    let login: LoginResponse = login.json().expect("login response should parse");

    now.store(login.expires_at + 1, Ordering::SeqCst);

    let response = service
        .http_client
        .post(format!("{}/api/unlock", service.remote_base_url))
        .bearer_auth(&login.token)
        .send()
        .expect("expired-token request should complete");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let error: ErrorResponse = response.json().expect("error response should parse");
    assert!(error.error.contains("expired"));
}

#[test]
fn repeated_bad_logins_are_throttled() {
    let now = Arc::new(AtomicU64::new(40_000));
    let service = TestService::start(now);
    service.prime_locked_device();

    for _ in 0..5 {
        let response = remote_login(&service, "wrong-secret");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let throttled = remote_login(&service, "wrong-secret");
    assert_eq!(throttled.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(throttled.headers().contains_key("retry-after"));
}

#[test]
fn remote_admin_can_be_disabled_without_breaking_local_unlock() {
    let now = Arc::new(AtomicU64::new(50_000));
    let service = TestService::start(now);
    service.prime_locked_device();

    let local_auth: AuthToken = service
        .ipc_client
        .local_login("parent-secret")
        .expect("local auth should still work");
    service
        .ipc_client
        .update_settings(
            SettingsUpdate {
                timeout_minutes: None,
                warning_minutes: None,
                action: None,
                autostart_enabled: None,
                remote_admin_enabled: Some(false),
            },
            &local_auth.token,
        )
        .expect("settings update should disable remote admin");

    let remote_login = remote_login(&service, "parent-secret");
    assert_eq!(remote_login.status(), StatusCode::FORBIDDEN);

    service
        .ipc_client
        .unlock(&local_auth.token)
        .expect("local unlock should not depend on remote admin");

    let snapshot = service
        .ipc_client
        .snapshot()
        .expect("snapshot should load after local unlock");
    assert!(matches!(
        snapshot.session_state,
        AdminSessionState::Unlocked
    ));
}

#[test]
fn remote_admin_serves_panel_assets_when_loader_is_configured() {
    let now = Arc::new(AtomicU64::new(60_000));
    let service = TestService::start_with(now, |config| {
        config.remote_asset_loader = Some(test_remote_assets());
    });

    let index = service
        .http_client
        .get(&service.remote_base_url)
        .send()
        .expect("index request should complete");
    assert_eq!(index.status(), StatusCode::OK);
    assert_eq!(
        index
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
    assert_eq!(
        index
            .headers()
            .get("x-frame-options")
            .and_then(|value| value.to_str().ok()),
        Some("DENY")
    );
    assert!(index
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html")));
    let index_body = index.text().expect("index body should read");
    assert!(index_body.contains("Sessionizer Admin"));

    let asset = service
        .http_client
        .get(format!("{}/assets/admin.js", service.remote_base_url))
        .send()
        .expect("asset request should complete");
    assert_eq!(asset.status(), StatusCode::OK);
    assert!(asset
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/javascript")));
    let asset_body = asset.text().expect("asset body should read");
    assert!(asset_body.contains("sessionizer-admin"));

    let missing = service
        .http_client
        .get(format!("{}/assets/missing.js", service.remote_base_url))
        .send()
        .expect("missing asset request should complete");
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[test]
fn remote_admin_bind_failure_keeps_local_ipc_available() {
    let now = Arc::new(AtomicU64::new(70_000));
    let blocker = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
        .expect("port blocker should bind");
    let blocked_remote_addr = blocker
        .local_addr()
        .expect("blocked listener should expose a local address");

    let service = TestService::start_with(now, |config| {
        config.remote_bind = blocked_remote_addr;
    });
    service.prime_locked_device();

    let panel_info = service
        .ipc_client
        .admin_panel_info()
        .expect("panel info should remain available over IPC");
    assert!(!panel_info.running);
    assert_eq!(panel_info.port, blocked_remote_addr.port());
    assert!(panel_info
        .error
        .as_deref()
        .is_some_and(|error| error.contains("Failed to bind remote admin listener")));

    let auth = service
        .ipc_client
        .local_login("parent-secret")
        .expect("local auth should still work after remote bind failure");
    service
        .ipc_client
        .unlock(&auth.token)
        .expect("unlock should still work after remote bind failure");

    let snapshot = service
        .ipc_client
        .snapshot()
        .expect("snapshot should still be available");
    assert!(matches!(
        snapshot.session_state,
        AdminSessionState::Unlocked
    ));
}
