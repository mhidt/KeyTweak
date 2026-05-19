use crate::{autoreplace, capslock, config::Config, keyboard_hook::KeyboardHook, translate};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

pub struct AppState {
    config: Mutex<Config>,
    caps_paused: AtomicBool,
    keyboard_hook: Mutex<Option<KeyboardHook>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let caps_paused = config.caps_lock.paused;
        capslock::configure(&config.caps_lock);
        autoreplace::configure(&config.auto_replace);
        translate::configure(&config.translate);

        Self {
            config: Mutex::new(config),
            caps_paused: AtomicBool::new(caps_paused),
            keyboard_hook: Mutex::new(None),
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
        translate::configure(&config.translate);
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
}
