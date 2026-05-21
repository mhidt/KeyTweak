use std::{
    process::{Child, Command, Stdio},
    sync::Mutex,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const LIBRETRANSLATE_COMMAND: &str = "libretranslate";
const LIBRETRANSLATE_ARGS: [&str; 2] = ["--load-only", "en,ru"];

pub struct LibreTranslateServer {
    child: Mutex<Option<Child>>,
}

impl LibreTranslateServer {
    pub fn new() -> Self {
        Self {
            child: Mutex::new(None),
        }
    }

    pub fn start(&self) {
        let Ok(mut child) = self.child.lock() else {
            return;
        };

        if child.is_some() {
            return;
        }

        let spawned_child = spawn_libretranslate_server()
            .map_err(|error| {
                log::error!("failed to start LibreTranslate server: {error}");
                error
            })
            .ok();

        *child = spawned_child;
    }

    pub fn stop(&self) {
        let Ok(mut child) = self.child.lock() else {
            return;
        };

        stop_child(child.take());
    }
}

impl Drop for LibreTranslateServer {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            stop_child(child.take());
        }
    }
}

fn spawn_libretranslate_server() -> std::io::Result<Child> {
    let mut command = Command::new(LIBRETRANSLATE_COMMAND);
    command
        .args(LIBRETRANSLATE_ARGS)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    command.spawn()
}

fn stop_child(child: Option<Child>) {
    let Some(mut child) = child else {
        return;
    };

    match child.try_wait() {
        Ok(Some(_)) => {}
        Ok(None) => {
            if let Err(error) = child.kill() {
                log::warn!("failed to stop LibreTranslate server: {error}");
            }
            let _ = child.wait();
        }
        Err(error) => {
            log::warn!("failed to query LibreTranslate server status: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libretranslate_command_is_scoped_to_english_and_russian() {
        assert_eq!(LIBRETRANSLATE_COMMAND, "libretranslate");
        assert_eq!(LIBRETRANSLATE_ARGS, ["--load-only", "en,ru"]);
    }
}
