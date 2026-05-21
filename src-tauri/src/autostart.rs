use std::{
    io,
    path::Path,
    process::{Command, Stdio},
};
use thiserror::Error;

const TASK_NAME: &str = "KeyTweak";

#[derive(Debug, Error)]
pub enum AutoStartError {
    #[error("failed to run Windows Task Scheduler command: {0}")]
    Command(io::Error),
    #[error("Windows Task Scheduler returned an error: {0}")]
    TaskScheduler(String),
}

pub type Result<T> = std::result::Result<T, AutoStartError>;

pub fn set_auto_start(enabled: bool, exe_path: &Path) -> Result<()> {
    delete_legacy_run_entry();

    if enabled {
        create_elevated_logon_task(exe_path)
    } else {
        delete_logon_task()
    }
}

pub fn is_auto_start() -> Result<bool> {
    let output = schtasks()
        .args(["/Query", "/TN", TASK_NAME])
        .output()
        .map_err(AutoStartError::Command)?;

    Ok(output.status.success())
}

fn create_elevated_logon_task(exe_path: &Path) -> Result<()> {
    let task_command = format!("\"{}\"", exe_path.display());
    let args = [
        "/Create",
        "/TN",
        TASK_NAME,
        "/SC",
        "ONLOGON",
        "/TR",
        &task_command,
        "/RL",
        "HIGHEST",
        "/F",
    ];
    let output = schtasks()
        .args(args)
        .output()
        .map_err(AutoStartError::Command)?;

    if output.status.success() {
        Ok(())
    } else {
        run_schtasks_elevated(&args)
    }
}

fn delete_logon_task() -> Result<()> {
    if !is_auto_start()? {
        return Ok(());
    }

    let output = schtasks()
        .args(["/Delete", "/TN", TASK_NAME, "/F"])
        .output()
        .map_err(AutoStartError::Command)?;
    let success = output.status.success();

    if success {
        Ok(())
    } else {
        run_schtasks_elevated(&["/Delete", "/TN", TASK_NAME, "/F"])
    }
}

fn schtasks() -> Command {
    let mut command = Command::new("schtasks.exe");
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn delete_legacy_run_entry() {
    let _ = Command::new("reg.exe")
        .args([
            "delete",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            TASK_NAME,
            "/f",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn run_schtasks_elevated(args: &[&str]) -> Result<()> {
    let argument_list = args
        .iter()
        .map(|arg| format!("'{}'", arg.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",");
    let command = format!(
        "$p = Start-Process -FilePath schtasks.exe -ArgumentList @({argument_list}) -Verb RunAs -Wait -PassThru; exit $p.ExitCode"
    );
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &command,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(AutoStartError::Command)?;

    if output.status.success() {
        Ok(())
    } else {
        Err(AutoStartError::TaskScheduler(output_text(output)))
    }
}

fn output_text(output: std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = format!("{stdout}{stderr}").trim().to_string();

    if text.is_empty() {
        "операция отменена или не подтверждена в UAC".to_string()
    } else if text.contains('\u{FFFD}') {
        "Task Scheduler вернул нечитаемую локализованную ошибку; обычно это отказ в доступе или отмененный UAC-запрос".to_string()
    } else {
        text
    }
}
