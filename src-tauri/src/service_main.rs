use crate::device_controller::DeviceController;
use crate::ipc::build_ipc_router;
use crate::remote_admin::{build_remote_admin_router, AdminPanelInfo};
use crate::session;
use crate::state_store::StateStore;
use rand::distributions::{Alphanumeric, DistString};
use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use tokio::net::TcpListener;
use tokio::runtime::Builder;
use tokio::sync::watch;

const REMOTE_SESSION_TTL_SECONDS: u64 = 12 * 60 * 60;
const LOCAL_SESSION_TTL_SECONDS: u64 = 60;
const FAILED_LOGIN_WINDOW_SECONDS: u64 = 5 * 60;
const FAILED_LOGIN_THRESHOLD: usize = 5;
const FAILED_LOGIN_THROTTLE_SECONDS: u64 = 60;
const DEFAULT_IPC_PORT: u16 = 47_770;
const DEFAULT_REMOTE_PORT: u16 = 47_771;
static EMBEDDED_SERVICE_STARTED: OnceLock<()> = OnceLock::new();

pub type TimeSource = Arc<dyn Fn() -> u64 + Send + Sync>;
pub type StateChangedCallback = Arc<dyn Fn() + Send + Sync>;
pub type RemoteAssetLoader = Arc<dyn Fn(&str) -> Option<RemoteAsset> + Send + Sync>;

#[derive(Debug, Clone)]
pub struct RemoteAsset {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub csp_header: Option<String>,
}

#[derive(Clone)]
pub struct ServiceState {
    pub controller: DeviceController,
    pub auth: ServiceAuthState,
    on_state_changed: Option<StateChangedCallback>,
    remote_asset_loader: Option<RemoteAssetLoader>,
}

#[derive(Clone)]
pub struct ServiceAuthState {
    remote_sessions: Arc<Mutex<HashMap<String, u64>>>,
    local_sessions: Arc<Mutex<HashMap<String, u64>>>,
    failed_remote_logins: Arc<Mutex<VecDeque<u64>>>,
    panel_info: Arc<Mutex<AdminPanelInfo>>,
    time_source: TimeSource,
}

#[derive(Clone)]
pub struct ServiceConfig {
    pub controller: DeviceController,
    pub ipc_bind: SocketAddr,
    pub remote_bind: SocketAddr,
    pub time_source: TimeSource,
    pub on_state_changed: Option<StateChangedCallback>,
    pub remote_asset_loader: Option<RemoteAssetLoader>,
}

#[derive(Debug, Clone, Copy)]
pub struct ServicePorts {
    pub ipc_addr: SocketAddr,
    pub remote_addr: SocketAddr,
}

pub struct ServiceHandle {
    ports: ServicePorts,
    shutdown: Option<watch::Sender<bool>>,
    thread: Option<JoinHandle<()>>,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            controller: DeviceController::new(StateStore::default()),
            ipc_bind: SocketAddr::from((Ipv4Addr::LOCALHOST, DEFAULT_IPC_PORT)),
            remote_bind: SocketAddr::from((Ipv4Addr::UNSPECIFIED, DEFAULT_REMOTE_PORT)),
            time_source: Arc::new(session::current_timestamp),
            on_state_changed: None,
            remote_asset_loader: None,
        }
    }
}

impl ServiceConfig {
    pub fn for_tests(controller: DeviceController, time_source: TimeSource) -> Self {
        Self {
            controller,
            ipc_bind: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            remote_bind: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            time_source,
            on_state_changed: None,
            remote_asset_loader: None,
        }
    }
}

impl ServiceState {
    pub fn new(config: &ServiceConfig) -> Self {
        Self {
            controller: config.controller.clone(),
            auth: ServiceAuthState::new(config.time_source.clone()),
            on_state_changed: config.on_state_changed.clone(),
            remote_asset_loader: config.remote_asset_loader.clone(),
        }
    }

    pub fn notify_state_changed(&self) {
        if let Some(callback) = &self.on_state_changed {
            callback();
        }
    }

    pub fn remote_assets_available(&self) -> bool {
        self.remote_asset_loader.is_some()
    }

    pub fn resolve_remote_asset(&self, path: &str) -> Option<RemoteAsset> {
        self.remote_asset_loader
            .as_ref()
            .and_then(|loader| loader(path))
    }
}

impl ServiceAuthState {
    pub fn new(time_source: TimeSource) -> Self {
        Self {
            remote_sessions: Arc::new(Mutex::new(HashMap::new())),
            local_sessions: Arc::new(Mutex::new(HashMap::new())),
            failed_remote_logins: Arc::new(Mutex::new(VecDeque::new())),
            panel_info: Arc::new(Mutex::new(AdminPanelInfo::default())),
            time_source,
        }
    }

    pub fn now(&self) -> u64 {
        (self.time_source)()
    }

    pub fn issue_remote_session(&self) -> (String, u64) {
        self.issue_session(&self.remote_sessions, REMOTE_SESSION_TTL_SECONDS)
    }

    pub fn issue_local_session(&self) -> (String, u64) {
        self.issue_session(&self.local_sessions, LOCAL_SESSION_TTL_SECONDS)
    }

    pub fn authorize_remote(&self, token: &str) -> bool {
        self.authorize(&self.remote_sessions, token)
    }

    pub fn authorize_local(&self, token: &str) -> bool {
        self.authorize(&self.local_sessions, token)
    }

    pub fn revoke_remote(&self, token: &str) {
        let mut sessions = self
            .remote_sessions
            .lock()
            .expect("remote session lock poisoned");
        sessions.remove(token);
    }

    pub fn revoke_local(&self, token: &str) {
        let mut sessions = self
            .local_sessions
            .lock()
            .expect("local session lock poisoned");
        sessions.remove(token);
    }

    pub fn clear_failed_remote_logins(&self) {
        self.failed_remote_logins
            .lock()
            .expect("remote login throttle lock poisoned")
            .clear();
    }

    pub fn record_failed_remote_login(&self) {
        let now = self.now();
        let mut failures = self
            .failed_remote_logins
            .lock()
            .expect("remote login throttle lock poisoned");
        failures.push_back(now);
        while let Some(oldest) = failures.front() {
            if now.saturating_sub(*oldest) <= FAILED_LOGIN_WINDOW_SECONDS {
                break;
            }
            failures.pop_front();
        }
    }

    pub fn remote_login_retry_after_seconds(&self) -> Option<u64> {
        let now = self.now();
        let mut failures = self
            .failed_remote_logins
            .lock()
            .expect("remote login throttle lock poisoned");

        while let Some(oldest) = failures.front() {
            if now.saturating_sub(*oldest) <= FAILED_LOGIN_WINDOW_SECONDS {
                break;
            }
            failures.pop_front();
        }

        if failures.len() < FAILED_LOGIN_THRESHOLD {
            return None;
        }

        let threshold_index = failures.len().saturating_sub(FAILED_LOGIN_THRESHOLD);
        let throttle_anchor = failures[threshold_index];
        let throttle_until = throttle_anchor.saturating_add(FAILED_LOGIN_THROTTLE_SECONDS);

        (throttle_until > now).then_some(throttle_until - now)
    }

    pub fn panel_snapshot(&self) -> AdminPanelInfo {
        self.panel_info
            .lock()
            .expect("admin panel info lock poisoned")
            .clone()
    }

    pub fn mark_remote_running(&self, listen_address: String, port: u16, urls: Vec<String>) {
        let mut info = self
            .panel_info
            .lock()
            .expect("admin panel info lock poisoned");
        info.running = true;
        info.listen_address = listen_address;
        info.port = port;
        info.urls = urls;
        info.error = None;
    }

    pub fn mark_remote_error(&self, listen_address: String, port: u16, error: String) {
        let mut info = self
            .panel_info
            .lock()
            .expect("admin panel info lock poisoned");
        info.running = false;
        info.listen_address = listen_address;
        info.port = port;
        info.urls.clear();
        info.error = Some(error);
    }

    fn issue_session(
        &self,
        sessions: &Arc<Mutex<HashMap<String, u64>>>,
        ttl_seconds: u64,
    ) -> (String, u64) {
        let now = self.now();
        let mut sessions = sessions.lock().expect("session lock poisoned");
        sessions.retain(|_, expires_at| *expires_at > now);

        let token = Alphanumeric.sample_string(&mut rand::thread_rng(), 48);
        let expires_at = now.saturating_add(ttl_seconds);
        sessions.insert(token.clone(), expires_at);
        (token, expires_at)
    }

    fn authorize(&self, sessions: &Arc<Mutex<HashMap<String, u64>>>, token: &str) -> bool {
        let now = self.now();
        let mut sessions = sessions.lock().expect("session lock poisoned");
        sessions.retain(|_, expires_at| *expires_at > now);
        matches!(sessions.get(token), Some(expires_at) if *expires_at > now)
    }
}

impl ServiceHandle {
    pub fn spawn(config: ServiceConfig) -> Result<Self, String> {
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let thread = std::thread::spawn(move || {
            let runtime = Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to build service runtime");

            runtime.block_on(async move {
                let service_state = ServiceState::new(&config);

                let ipc_listener = match TcpListener::bind(config.ipc_bind).await {
                    Ok(listener) => listener,
                    Err(error) => {
                        let _ = ready_tx.send(Err(format!(
                            "Failed to bind IPC listener on {}: {error}",
                            config.ipc_bind
                        )));
                        return;
                    }
                };

                let remote_listener = match TcpListener::bind(config.remote_bind).await {
                    Ok(listener) => Some(listener),
                    Err(error) => {
                        let message = format!(
                            "Failed to bind remote admin listener on {}: {error}",
                            config.remote_bind
                        );
                        eprintln!("Sessionizer remote admin listener unavailable: {message}");
                        service_state.auth.mark_remote_error(
                            config.remote_bind.ip().to_string(),
                            config.remote_bind.port(),
                            message,
                        );
                        None
                    }
                };

                let ports = ServicePorts {
                    ipc_addr: ipc_listener
                        .local_addr()
                        .expect("ipc listener should have a local address"),
                    remote_addr: remote_listener
                        .as_ref()
                        .and_then(|listener| listener.local_addr().ok())
                        .unwrap_or(config.remote_bind),
                };

                if remote_listener.is_some() {
                    service_state.auth.mark_remote_running(
                        ports.remote_addr.ip().to_string(),
                        ports.remote_addr.port(),
                        crate::remote_admin::admin_urls(ports.remote_addr),
                    );
                }

                let _ = ready_tx.send(Ok(ports));

                let ipc_router = build_ipc_router(service_state.clone());

                let mut ipc_shutdown = shutdown_rx.clone();
                let ipc_server =
                    axum::serve(ipc_listener, ipc_router).with_graceful_shutdown(async move {
                        loop {
                            if *ipc_shutdown.borrow() {
                                break;
                            }

                            if ipc_shutdown.changed().await.is_err() {
                                break;
                            }
                        }
                    });

                let remote_server = async move {
                    if let Some(remote_listener) = remote_listener {
                        let remote_router = build_remote_admin_router(service_state.clone());
                        let mut remote_shutdown = shutdown_rx.clone();

                        axum::serve(remote_listener, remote_router)
                            .with_graceful_shutdown(async move {
                                loop {
                                    if *remote_shutdown.borrow() {
                                        break;
                                    }

                                    if remote_shutdown.changed().await.is_err() {
                                        break;
                                    }
                                }
                            })
                            .await
                    } else {
                        Ok(())
                    }
                };

                let (ipc_result, remote_result) = tokio::join!(ipc_server, remote_server);

                if let Err(error) = ipc_result {
                    eprintln!("Sessionizer IPC server stopped unexpectedly: {error}");
                }

                if let Err(error) = remote_result {
                    eprintln!("Sessionizer remote admin server stopped unexpectedly: {error}");
                }
            });
        });

        let ports = ready_rx
            .recv()
            .map_err(|error| format!("Service startup failed before readiness: {error}"))??;

        Ok(Self {
            ports,
            shutdown: Some(shutdown_tx),
            thread: Some(thread),
        })
    }

    pub fn ports(&self) -> ServicePorts {
        self.ports
    }

    pub fn ipc_base_url(&self) -> String {
        base_url(self.ports.ipc_addr)
    }

    pub fn remote_base_url(&self) -> String {
        base_url(self.ports.remote_addr)
    }

    pub fn shutdown(mut self) {
        self.shutdown_inner();
    }

    fn shutdown_inner(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(true);
        }

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        self.shutdown_inner();
    }
}

pub fn ensure_embedded_service(
    on_state_changed: Option<StateChangedCallback>,
    remote_asset_loader: Option<RemoteAssetLoader>,
) -> Result<(), String> {
    if EMBEDDED_SERVICE_STARTED.get().is_some() {
        return Ok(());
    }

    if let Ok(client) = crate::ipc::ServiceClient::default_local() {
        if client.snapshot().is_ok() {
            let _ = EMBEDDED_SERVICE_STARTED.set(());
            return Ok(());
        }
    }

    let config = ServiceConfig {
        on_state_changed,
        remote_asset_loader,
        ..ServiceConfig::default()
    };
    let handle = ServiceHandle::spawn(config)?;
    let _ = EMBEDDED_SERVICE_STARTED.set(());
    std::mem::forget(handle);
    Ok(())
}

pub fn base_url(address: SocketAddr) -> String {
    let host = match address.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => Ipv4Addr::LOCALHOST.to_string(),
        ip => ip.to_string(),
    };

    format!("http://{host}:{}", address.port())
}
