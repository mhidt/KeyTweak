mod engine;
mod protocol;
mod sbd;

use protocol::{ErrorInfo, Request, Response, PROTOCOL_VERSION};
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static RUNNING: AtomicBool = AtomicBool::new(true);

struct Sidecar {
    engine: Option<engine::Engine>,
    stdout: Mutex<std::io::Stdout>,
    init_received: AtomicBool,
}

impl Sidecar {
    fn send(&self, resp: &Response) {
        let mut stdout = self.stdout.lock().unwrap();
        if let Ok(json) = serde_json::to_string(resp) {
            let _ = writeln!(stdout, "{json}");
            let _ = stdout.flush();
        }
    }

    fn is_ready(&self) -> bool {
        self.engine.as_ref().map_or(false, |e| e.is_ready())
    }

    fn languages(&self) -> Vec<String> {
        self.engine
            .as_ref()
            .map(|e| e.languages().to_vec())
            .unwrap_or_default()
    }

    fn handle_init(&self, client_version: &str) {
        if client_version != PROTOCOL_VERSION {
            log::error!(
                "Protocol version mismatch: host={client_version}, sidecar={PROTOCOL_VERSION}"
            );
            self.send(&Response::init_error(
                "PROTOCOL_VERSION_MISMATCH",
                &format!("Expected {PROTOCOL_VERSION}, got {client_version}"),
            ));
            self.send(&Response::shutdown("protocol_version_mismatch"));
            RUNNING.store(false, Ordering::SeqCst);
            return;
        }

        let ready = self.is_ready();
        let languages = if ready {
            self.languages()
        } else {
            Vec::new()
        };

        self.send(&Response::init(ready, languages));
        self.init_received.store(true, Ordering::SeqCst);
        log::info!("Init handshake complete: ready={ready}");
    }

    fn handle_translate(&self, id: u64, q: &str, source: &str, target: &str) {
        if q.is_empty() || source.is_empty() || target.is_empty() {
            self.send(&Response::translate_error(
                id,
                ErrorInfo::invalid_request("Missing required fields: q, source, target"),
            ));
            return;
        }

        let engine = match self.engine.as_ref() {
            Some(e) => e,
            None => {
                self.send(&Response::translate_error(
                    id,
                    ErrorInfo::translation_error("Engine not loaded"),
                ));
                return;
            }
        };

        match engine.translate(q, source, target) {
            Ok(translated) => {
                self.send(&Response::translate_result(id, translated));
            }
            Err(err) => {
                self.send(&Response::translate_error(id, err));
            }
        }
    }

    fn handle_status(&self, id: u64) {
        self.send(&Response::status(id, self.is_ready(), self.languages()));
    }

    fn handle_exit(&self, reason: &str) {
        log::info!("Exit requested: {reason}");
        self.send(&Response::shutdown(reason));
        RUNNING.store(false, Ordering::SeqCst);
    }
}

fn heartbeat_loop(sidecar: &Sidecar) {
    while RUNNING.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_secs(protocol::HEARTBEAT_INTERVAL_SECS));

        if !RUNNING.load(Ordering::SeqCst) {
            break;
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();

        sidecar.send(&Response::heartbeat(timestamp));
    }
}

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::new()
            .filter("SIDECAR_LOG_LEVEL")
            .default_filter_or("info"),
    )
    .format_timestamp(Some(env_logger::fmt::TimestampPrecision::Millis))
    .init();

    let packages_dir = std::env::var("ARGOS_PACKAGES_DIR").unwrap_or_default();
    let packages_path = std::path::PathBuf::from(&packages_dir);

    log::info!("ARGOS_PACKAGES_DIR={}", packages_dir);

    let engine = if packages_dir.is_empty() {
        log::warn!("ARGOS_PACKAGES_DIR not set, no models will be loaded");
        None
    } else {
        match engine::Engine::load(&packages_path) {
            Ok(e) => Some(e),
            Err(e) => {
                log::error!("Failed to load engine: {e}");
                None
            }
        }
    };

    let sidecar = Sidecar {
        engine,
        stdout: Mutex::new(std::io::stdout()),
        init_received: AtomicBool::new(false),
    };

    let hb_sidecar: &Sidecar = &sidecar;
    std::thread::scope(|s| {
        s.spawn(|| heartbeat_loop(hb_sidecar));

        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    log::error!("Stdin read error: {e}");
                    break;
                }
            };

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let req: Request = match serde_json::from_str(trimmed) {
                Ok(r) => r,
                Err(e) => {
                    log::warn!("Invalid JSON from host: {e}");
                    continue;
                }
            };

            match req {
                Request::Init { protocol_version } => {
                    sidecar.handle_init(&protocol_version);
                    if !RUNNING.load(Ordering::SeqCst) {
                        break;
                    }
                }
                Request::Translate {
                    id,
                    q,
                    source,
                    target,
                } => {
                    if !sidecar.init_received.load(Ordering::SeqCst) {
                        log::warn!("translate before init, ignoring");
                        continue;
                    }
                    sidecar.handle_translate(id, &q, &source, &target);
                }
                Request::Status { id } => {
                    if !sidecar.init_received.load(Ordering::SeqCst) {
                        log::warn!("status before init, ignoring");
                        continue;
                    }
                    sidecar.handle_status(id);
                }
                Request::Exit { reason } => {
                    sidecar.handle_exit(reason.as_deref().unwrap_or("graceful"));
                    break;
                }
            }
        }

        RUNNING.store(false, Ordering::SeqCst);
    });

    log::info!("Sidecar exiting");
}
