use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::{Mutex, oneshot},
    time::timeout,
};

const PROTOCOL_VERSION: &str = "1.0";
const INIT_TIMEOUT: Duration = Duration::from_secs(30);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(60);
const TRANSLATE_TIMEOUT: Duration = Duration::from_secs(30);
#[allow(dead_code)]
const STATUS_TIMEOUT: Duration = Duration::from_secs(5);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_PENDING: usize = 100;
const PENDING_TTL: Duration = Duration::from_secs(120);
const MAX_RESTART_ATTEMPTS: u32 = 5;
const MAX_BACKOFF: Duration = Duration::from_secs(60);

static ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn generate_id() -> u64 {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed) & 0xFFFFF;
    (ts << 20) | counter
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum Request {
    Init {
        protocol_version: String,
    },
    Translate {
        id: u64,
        q: String,
        source: String,
        target: String,
    },
    Status {
        id: u64,
    },
    Exit {
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SidecarResponse {
    #[serde(default)]
    id: Option<u64>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    response_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    translated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ready: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    capabilities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    languages: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    protocol_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<SidecarError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SidecarError {
    code: String,
    message: String,
    #[serde(default)]
    recoverable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SidecarErrorType {
    #[error("sidecar not installed")]
    NotInstalled,
    #[error("sidecar crashed: {retryable}")]
    Crashed { retryable: bool },
    #[error("sidecar not ready: {0}")]
    NotReady(String),
    #[error("sidecar timeout: {0}")]
    Timeout(String),
    #[error("protocol version mismatch: {0}")]
    ProtocolVersionMismatch(String),
    #[error("too many pending requests")]
    TooManyPending,
    #[error("sidecar error: {0}")]
    Sidecar(String),
    #[error("IO error: {0}")]
    Io(String),
}

struct PendingRequest {
    sender: oneshot::Sender<Result<SidecarResponse, SidecarErrorType>>,
    created_at: Instant,
}

struct SidecarState {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    pending: HashMap<u64, PendingRequest>,
    ready: bool,
    models_ready: bool,
    last_heartbeat: Instant,
    watchdog_enabled: bool,
    languages: Vec<String>,
}

pub struct TranslatorSidecar {
    state: Arc<Mutex<SidecarState>>,
    running: Arc<AtomicBool>,
    install_dir: Option<PathBuf>,
}

impl TranslatorSidecar {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(SidecarState {
                child: None,
                stdin: None,
                pending: HashMap::new(),
                ready: false,
                models_ready: false,
                last_heartbeat: Instant::now(),
                watchdog_enabled: false,
                languages: Vec::new(),
            })),
            running: Arc::new(AtomicBool::new(false)),
            install_dir: None,
        }
    }

    pub fn set_install_dir(&mut self, dir: PathBuf) {
        self.install_dir = Some(dir);
    }

    pub fn is_installed(&self) -> bool {
        if let Some(dir) = &self.install_dir {
            dir.join("translator").join("translator.exe").exists()
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub fn is_models_installed(&self) -> bool {
        if let Some(dir) = &self.install_dir {
            let models_dir = dir.join("translator-models");
            models_dir.join("translate-en_ru-1_9").exists()
                && models_dir.join("translate-ru_en-1_9").exists()
        } else {
            false
        }
    }

    pub async fn start(&self) -> Result<(), SidecarErrorType> {
        if !self.is_installed() {
            return Err(SidecarErrorType::NotInstalled);
        }

        self.running.store(true, Ordering::SeqCst);
        self.spawn_and_init().await?;

        let state = self.state.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut backoff_attempts = 0u32;
            let mut backoff_duration = Duration::from_secs(1);

            while running.load(Ordering::SeqCst) {
                let should_restart = {
                    let mut s = state.lock().await;
                    if let Some(child) = &mut s.child {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                log::warn!("Sidecar exited with status: {status}");
                                true
                            }
                            Ok(None) => false,
                            Err(e) => {
                                log::error!("Failed to check sidecar status: {e}");
                                true
                            }
                        }
                    } else {
                        true
                    }
                };

                if should_restart {
                    let s = state.lock().await;
                    let pending_count = s.pending.len();
                    drop(s);

                    if pending_count > 0 {
                        let mut s = state.lock().await;
                        for (_, req) in s.pending.drain() {
                            let _ = req.sender.send(Err(SidecarErrorType::Crashed {
                                retryable: true,
                            }));
                        }
                    }

                    if backoff_attempts >= MAX_RESTART_ATTEMPTS {
                        log::error!("Sidecar failed {MAX_RESTART_ATTEMPTS} times, giving up");
                        running.store(false, Ordering::SeqCst);
                        break;
                    }

                    log::warn!(
                        "Restarting sidecar (attempt {}/{}) in {:?}...",
                        backoff_attempts + 1,
                        MAX_RESTART_ATTEMPTS,
                        backoff_duration
                    );
                    tokio::time::sleep(backoff_duration).await;

                    match Self::do_spawn_and_init(&state, &running).await {
                        Ok(()) => {
                            backoff_attempts = 0;
                            backoff_duration = Duration::from_secs(1);
                            log::info!("Sidecar restarted successfully");
                        }
                        Err(e) => {
                            log::error!("Sidecar restart failed: {e}");
                            backoff_attempts += 1;
                            backoff_duration =
                                (backoff_duration * 2).min(MAX_BACKOFF);
                        }
                    }
                }

                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        self.start_watchdog();
        self.start_pending_cleanup();        Ok(())
    }

    async fn spawn_and_init(&self) -> Result<(), SidecarErrorType> {
        Self::do_spawn_and_init(&self.state, &self.running).await
    }

    async fn do_spawn_and_init(
        state: &Arc<Mutex<SidecarState>>,
        running: &Arc<AtomicBool>,
    ) -> Result<(), SidecarErrorType> {
        let (mut child, stdin, stdout) = Self::spawn_process()?;
        let stderr = child.stderr.take();

        let mut s = state.lock().await;
        s.child = Some(child);
        s.stdin = Some(stdin);
        s.ready = false;
        s.models_ready = false;
        s.watchdog_enabled = false;
        s.last_heartbeat = Instant::now();
        let (tx, rx) = oneshot::channel();
        s.pending.insert(0, PendingRequest {
            sender: tx,
            created_at: Instant::now(),
        });
        drop(s);

        Self::start_stdout_reader(stdout, state.clone(), running.clone());
        Self::start_stderr_reader(stderr);

        let init_cmd = Request::Init {
            protocol_version: PROTOCOL_VERSION.to_string(),
        };

        let mut s = state.lock().await;
        let _ = Self::do_write_command(&mut s, &init_cmd).await;
        drop(s);

        match timeout(INIT_TIMEOUT, rx).await {
            Ok(Ok(Ok(resp))) => {
                let mut s = state.lock().await;
                if let Some(version) = &resp.protocol_version {
                    if version != PROTOCOL_VERSION {
                        let reason = format!(
                            "Expected {PROTOCOL_VERSION}, got {version}"
                        );
                        let _ = Self::do_write_command(&mut *s, &Request::Exit {
                            reason: Some("protocol_version_mismatch".to_string()),
                        }).await;
                        return Err(SidecarErrorType::ProtocolVersionMismatch(reason));
                    }
                }
                s.models_ready = resp.ready.unwrap_or(false);
                s.ready = s.models_ready;
                s.languages = resp.languages.unwrap_or_default();
                s.watchdog_enabled = true;
                s.last_heartbeat = Instant::now();
                log::info!(
                    "Sidecar init complete: ready={}, languages={:?}",
                    s.models_ready,
                    s.languages
                );
                Ok(())
            }
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err(SidecarErrorType::Crashed { retryable: true }),
            Err(_) => Err(SidecarErrorType::Timeout("init handshake".to_string())),
        }
    }

    fn spawn_process() -> Result<(Child, ChildStdin, ChildStdout), SidecarErrorType> {
        let install_dir = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default();

        let exe_path = install_dir.join("translator").join("translator.exe");
        let models_path = install_dir.join("translator-models");

        if !exe_path.exists() {
            return Err(SidecarErrorType::NotInstalled);
        }

        let mut cmd = Command::new(&exe_path);
        cmd.env("ARGOS_PACKAGES_DIR", &models_path)
            .env(
                "SIDECAR_LOG_LEVEL",
                if cfg!(debug_assertions) {
                    "DEBUG"
                } else {
                    "INFO"
                },
            )
            .current_dir(&install_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        #[cfg(windows)]
        cmd.creation_flags(0x08000000);

        let mut child = cmd.spawn().map_err(|e| SidecarErrorType::Io(e.to_string()))?;

        let stdin = child.stdin.take().ok_or_else(|| {
            SidecarErrorType::Io("failed to capture stdin".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            SidecarErrorType::Io("failed to capture stdout".to_string())
        })?;

        Ok((child, stdin, stdout))
    }

    fn start_stdout_reader(
        stdout: ChildStdout,
        state: Arc<Mutex<SidecarState>>,
        running: Arc<AtomicBool>,
    ) {
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while running.load(Ordering::SeqCst) {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<SidecarResponse>(&line) {
                            Ok(resp) => {
                                Self::handle_response(&state, resp).await;
                            }
                            Err(e) => {
                                log::warn!("Invalid JSON from sidecar: {e}");
                            }
                        }
                    }
                    Ok(None) => {
                        log::warn!("Sidecar stdout EOF");
                        break;
                    }
                    Err(e) => {
                        log::error!("Sidecar stdout read error: {e}");
                        break;
                    }
                }
            }
        });
    }

    fn start_stderr_reader(stderr: Option<tokio::process::ChildStderr>) {
        tokio::spawn(async move {
            if let Some(stderr) = stderr {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    log::debug!("Sidecar stderr: {}", line.trim());
                }
            }
        });
    }

    async fn handle_response(state: &Arc<Mutex<SidecarState>>, resp: SidecarResponse) {
        let mut s = state.lock().await;

        if let Some(response_type) = &resp.response_type {
            match response_type.as_str() {
                "init" => {
                    s.models_ready = resp.ready.unwrap_or(false);
                    s.ready = s.models_ready;
                    s.languages = resp.languages.as_ref().map(|l| l.clone()).unwrap_or_default();
                    s.watchdog_enabled = true;
                    s.last_heartbeat = Instant::now();
                    if let Some(pending) = s.pending.remove(&0) {
                        let _ = pending.sender.send(Ok(resp));
                    }
                    return;
                }
                "heartbeat" => {
                    s.last_heartbeat = Instant::now();
                    return;
                }
                "shutdown" => {
                    log::info!("Sidecar shutdown: {:?}", resp.reason);
                    s.ready = false;
                    return;
                }
                _ => {}
            }
        }

        if let Some(id) = resp.id {
            if let Some(pending) = s.pending.remove(&id) {
                if resp.error.is_some() {
                    let _ = pending.sender.send(Ok(resp));
                } else {
                    let _ = pending.sender.send(Ok(resp));
                }
            }
        }
    }

    async fn write_command(
        &self,
        cmd: &Request,
    ) -> Result<(), SidecarErrorType> {
        let mut s = self.state.lock().await;
        Self::do_write_command(&mut s, cmd).await
    }

    async fn do_write_command(
        s: &mut SidecarState,
        cmd: &Request,
    ) -> Result<(), SidecarErrorType> {
        let json = serde_json::to_string(cmd).map_err(|e| {
            SidecarErrorType::Io(format!("serialize error: {e}"))
        })?;

        if let Some(stdin) = &mut s.stdin {
            stdin.write_all(json.as_bytes()).await.map_err(|e| {
                SidecarErrorType::Io(format!("write error: {e}"))
            })?;
            stdin.write_all(b"\n").await.map_err(|e| {
                SidecarErrorType::Io(format!("write newline error: {e}"))
            })?;
            stdin.flush().await.map_err(|e| {
                SidecarErrorType::Io(format!("flush error: {e}"))
            })?;
            Ok(())
        } else {
            Err(SidecarErrorType::NotReady("stdin not available".to_string()))
        }
    }

    fn start_watchdog(&self) {
        let state = self.state.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_secs(10)).await;

                let s = state.lock().await;
                if !s.watchdog_enabled || !s.ready {
                    continue;
                }
                let elapsed = s.last_heartbeat.elapsed();
                drop(s);

                if elapsed > HEARTBEAT_TIMEOUT {
                    log::warn!(
                        "Sidecar heartbeat timeout ({:?} > {:?}), killing process",
                        elapsed,
                        HEARTBEAT_TIMEOUT
                    );
                    let mut s = state.lock().await;
                    if let Some(child) = &mut s.child {
                        let _ = child.start_kill();
                    }
                    s.ready = false;
                    s.watchdog_enabled = false;
                }
            }
        });
    }

    fn start_pending_cleanup(&self) {
        let state = self.state.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                let mut s = state.lock().await;
                let before = s.pending.len();
                s.pending.retain(|_, req| req.created_at.elapsed() < PENDING_TTL);
                let removed = before - s.pending.len();
                if removed > 0 {
                    log::debug!("Cleaned up {removed} expired pending requests");
                }
            }
        });
    }

    pub async fn translate(
        &self,
        text: &str,
        source: &str,
        target: &str,
    ) -> Result<String, SidecarErrorType> {
        let id = generate_id();
        let request = Request::Translate {
            id,
            q: text.to_string(),
            source: source.to_string(),
            target: target.to_string(),
        };

        {
            let s = self.state.lock().await;
            if !s.ready {
                return Err(SidecarErrorType::NotReady(
                    if !s.models_ready {
                        "models not loaded".to_string()
                    } else {
                        "sidecar not running".to_string()
                    },
                ));
            }
            if s.pending.len() >= MAX_PENDING {
                return Err(SidecarErrorType::TooManyPending);
            }
        }

        let (tx, rx) = oneshot::channel();
        {
            let mut s = self.state.lock().await;
            s.pending.insert(id, PendingRequest {
                sender: tx,
                created_at: Instant::now(),
            });
        }

        self.write_command(&request).await?;

        match timeout(TRANSLATE_TIMEOUT, rx).await {
            Ok(Ok(Ok(resp))) => {
                if let Some(err) = &resp.error {
                    Err(SidecarErrorType::Sidecar(err.message.clone()))
                } else {
                    resp.translated.ok_or_else(|| {
                        SidecarErrorType::Sidecar(
                            "no translated field in response".to_string(),
                        )
                    })
                }
            }
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err(SidecarErrorType::Crashed { retryable: true }),
            Err(_) => {
                let mut s = self.state.lock().await;
                s.pending.remove(&id);
                Err(SidecarErrorType::Timeout("translate".to_string()))
            }
        }
    }

    #[allow(dead_code)]
    pub async fn status(&self) -> Result<SidecarResponse, SidecarErrorType> {
        let id = generate_id();
        let request = Request::Status { id };

        let (tx, rx) = oneshot::channel();
        {
            let mut s = self.state.lock().await;
            s.pending.insert(id, PendingRequest {
                sender: tx,
                created_at: Instant::now(),
            });
        }

        self.write_command(&request).await?;

        match timeout(STATUS_TIMEOUT, rx).await {
            Ok(Ok(Ok(resp))) => Ok(resp),
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err(SidecarErrorType::Crashed { retryable: true }),
            Err(_) => {
                let mut s = self.state.lock().await;
                s.pending.remove(&id);
                Err(SidecarErrorType::Timeout("status".to_string()))
            }
        }
    }

    pub async fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);

        let exit_cmd = Request::Exit {
            reason: Some("graceful".to_string()),
        };

        {
            let mut s = self.state.lock().await;
            let _ = Self::do_write_command(&mut *s, &exit_cmd).await;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;

        let mut s = self.state.lock().await;
        if let Some(child) = &mut s.child {
            match timeout(SHUTDOWN_TIMEOUT, child.wait()).await {
                Ok(Ok(status)) => {
                    log::info!("Sidecar exited gracefully: {status}");
                }
                _ => {
                    log::warn!("Sidecar did not exit gracefully, killing");
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                }
            }
        }

        for (_, req) in s.pending.drain() {
            let _ = req.sender.send(Err(SidecarErrorType::Crashed {
                retryable: false,
            }));
        }

        s.child = None;
        s.stdin = None;
        s.ready = false;
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for TranslatorSidecar {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_id_is_unique() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn generate_id_includes_timestamp() {
        let id = generate_id();
        let ts_part = id >> 20;
        assert!(ts_part > 0);
    }

    #[test]
    fn request_serialization() {
        let req = Request::Translate {
            id: 42,
            q: "Hello".to_string(),
            source: "en".to_string(),
            target: "ru".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"cmd\":\"translate\""));
        assert!(json.contains("\"id\":42"));
    }

    #[test]
    fn response_deserialization() {
        let json = r#"{"id":42,"translated":"Привет"}"#;
        let resp: SidecarResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, Some(42));
        assert_eq!(resp.translated, Some("Привет".to_string()));
    }

    #[test]
    fn init_response_deserialization() {
        let json = r#"{"type":"init","protocol_version":"1.0","ready":true,"capabilities":["translate","status"]}"#;
        let resp: SidecarResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.response_type, Some("init".to_string()));
        assert_eq!(resp.protocol_version, Some("1.0".to_string()));
        assert_eq!(resp.ready, Some(true));
    }

    #[test]
    fn error_response_deserialization() {
        let json = r#"{"id":1,"error":{"code":"MODEL_NOT_FOUND","message":"Not found","recoverable":false}}"#;
        let resp: SidecarResponse = serde_json::from_str(json).unwrap();
        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(err.code, "MODEL_NOT_FOUND");
        assert!(!err.recoverable);
    }

    #[test]
    fn heartbeat_response() {
        let json = r#"{"type":"heartbeat","timestamp":1234567890.0}"#;
        let resp: SidecarResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.response_type, Some("heartbeat".to_string()));
        assert!(resp.id.is_none());
    }
}
