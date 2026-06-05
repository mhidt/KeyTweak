use crate::{
    autoreplace, capslock, config::Config, exclusions, key_remap, keyboard_hook::KeyboardHook,
    sidecar::TranslatorSidecar, translate,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, OnceLock,
};
use tokio::sync::Mutex;

static GLOBAL_APP_STATE: OnceLock<Arc<AppState>> = OnceLock::new();

pub fn init_global_app_state(state: Arc<AppState>) {
    let _ = GLOBAL_APP_STATE.set(state);
}

pub fn global_app_state() -> Option<Arc<AppState>> {
    GLOBAL_APP_STATE.get().cloned()
}

pub struct AppState {
    config: std::sync::Mutex<Config>,
    caps_paused: AtomicBool,
    keyboard_hook: std::sync::Mutex<Option<KeyboardHook>>,
    sidecar: Arc<Mutex<TranslatorSidecar>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let caps_paused = config.caps_lock.paused;
        capslock::configure(&config.caps_lock);
        autoreplace::configure(&config.auto_replace);
        key_remap::configure(&config.key_remap);
        translate::configure(&config.translate);
        exclusions::configure(config.exception_mode, &config.exceptions);

        let mut sidecar = TranslatorSidecar::new();
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                sidecar.set_install_dir(parent.to_path_buf());
            }
        }

        Self {
            config: std::sync::Mutex::new(config),
            caps_paused: AtomicBool::new(caps_paused),
            keyboard_hook: std::sync::Mutex::new(None),
            sidecar: Arc::new(Mutex::new(sidecar)),
        }
    }

    pub fn caps_paused(&self) -> bool {
        self.caps_paused.load(Ordering::Relaxed)
    }

    pub fn config(&self) -> Config {
        self.config.lock().expect("config mutex poisoned").clone()
    }

    pub fn set_config(&self, config: Config) {
        capslock::configure(&config.caps_lock);
        autoreplace::configure(&config.auto_replace);
        key_remap::configure(&config.key_remap);
        translate::configure(&config.translate);
        exclusions::configure(config.exception_mode, &config.exceptions);
        self.caps_paused
            .store(config.caps_lock.paused, Ordering::Relaxed);

        let mut current = self.config.lock().expect("config mutex poisoned");
        *current = config;
    }

    pub fn set_caps_paused(&self, paused: bool) {
        self.caps_paused.store(paused, Ordering::Relaxed);
        capslock::set_paused(paused);

        if let Ok(mut config) = self.config.lock() {
            config.caps_lock.paused = paused;
        }
    }

    #[allow(dead_code)]
    pub fn with_config<T>(&self, f: impl FnOnce(&Config) -> T) -> T {
        let config = self.config.lock().expect("config mutex poisoned");
        f(&config)
    }

    pub fn install_keyboard_hook(&self) -> crate::keyboard_hook::Result<()> {
        let mut hook = self
            .keyboard_hook
            .lock()
            .expect("keyboard hook mutex poisoned");

        if hook.is_none() {
            *hook = Some(KeyboardHook::install()?);
        }

        Ok(())
    }

    pub fn uninstall_keyboard_hook(&self) {
        if let Ok(mut hook) = self.keyboard_hook.lock() {
            hook.take();
        }
    }

    pub async fn start_sidecar(&self) {
        let sidecar = self.sidecar.lock().await;
        if sidecar.is_installed() {
            if let Err(e) = sidecar.start().await {
                log::error!("Failed to start translator sidecar: {e}");
            }
        } else {
            log::info!("Translator sidecar not installed, skipping");
        }
    }

    pub async fn stop_sidecar(&self) {
        let sidecar = self.sidecar.lock().await;
        sidecar.stop().await;
    }

    pub async fn sidecar_translate(
        &self,
        text: &str,
        source: &str,
        target: &str,
    ) -> Result<String, crate::sidecar::SidecarErrorType> {
        let sidecar = self.sidecar.lock().await;
        sidecar.translate(text, source, target).await
    }

    pub fn sidecar_is_installed(&self) -> bool {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                return parent.join("translator").join("translator.exe").exists();
            }
        }
        false
    }

    pub fn models_are_installed(&self) -> bool {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let models_dir = parent.join("translator-models");
                return models_dir.join("translate-en_ru-1_9").exists()
                    && models_dir.join("translate-ru_en-1_9").exists();
            }
        }
        false
    }

    pub fn sidecar_is_running(&self) -> bool {
        if let Ok(sidecar) = self.sidecar.try_lock() {
            sidecar.is_running()
        } else {
            true
        }
    }
}
