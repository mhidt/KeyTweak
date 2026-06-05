use std::{
    path::PathBuf,
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout},
    time::timeout,
};

fn mock_sidecar_path() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    PathBuf::from(manifest_dir).join("tests").join("mock_sidecar.py")
}

fn python_exe() -> String {
    "py".to_string()
}

async fn spawn_mock() -> (Child, ChildStdin, ChildStdout) {
    let mut cmd = tokio::process::Command::new(python_exe());
    cmd.arg("-3.12")
        .arg(mock_sidecar_path())
        .env("MOCK_DELAY", "0.05")
        .env("MOCK_FAIL_NEVER", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    cmd.creation_flags(0x08000000);

    let mut child = cmd.spawn().expect("failed to spawn mock sidecar");
    let stdin = child.stdin.take().expect("no stdin");
    let stdout = child.stdout.take().expect("no stdout");
    (child, stdin, stdout)
}

struct MockSidecar {
    child: Child,
    stdin: ChildStdin,
    lines: tokio::io::Lines<BufReader<ChildStdout>>,
}

impl MockSidecar {
    async fn new() -> Self {
        let (child, stdin, stdout) = spawn_mock().await;
        let reader = BufReader::new(stdout);
        let lines = reader.lines();
        Self { child, stdin, lines }
    }

    async fn send(&mut self, obj: &serde_json::Value) {
        let mut json = serde_json::to_string(obj).unwrap();
        json.push('\n');
        self.stdin.write_all(json.as_bytes()).await.unwrap();
        self.stdin.flush().await.unwrap();
    }

    async fn recv(&mut self) -> serde_json::Value {
        loop {
            let line = timeout(Duration::from_secs(10), self.lines.next_line())
                .await
                .expect("timeout waiting for response")
                .expect("read error")
                .expect("EOF");
            let resp: serde_json::Value =
                serde_json::from_str(&line).unwrap_or_else(|e| {
                    panic!("Invalid JSON from mock: {e}\nLine: {line}");
                });
            if resp.get("type").and_then(|v| v.as_str()) == Some("heartbeat") {
                continue;
            }
            return resp;
        }
    }

    async fn kill(&mut self) {
        let _ = self.child.start_kill();
    }
}

#[tokio::test]
async fn test_sidecar_init_handshake() {
    let mut mock = MockSidecar::new().await;

    mock.send(&serde_json::json!({
        "cmd": "init",
        "protocol_version": "1.0"
    }))
    .await;

    let resp = mock.recv().await;
    assert_eq!(resp["type"], "init");
    assert_eq!(resp["protocol_version"], "1.0");
    assert_eq!(resp["ready"], true);

    mock.send(&serde_json::json!({"cmd": "exit"})).await;
    let resp = mock.recv().await;
    assert_eq!(resp["type"], "shutdown");
    mock.kill().await;
}

#[tokio::test]
async fn test_sidecar_translate() {
    let mut mock = MockSidecar::new().await;

    mock.send(&serde_json::json!({
        "cmd": "init",
        "protocol_version": "1.0"
    }))
    .await;
    let _ = mock.recv().await;

    mock.send(&serde_json::json!({
        "cmd": "translate",
        "id": 1,
        "q": "hello",
        "source": "en",
        "target": "ru"
    }))
    .await;

    let resp = mock.recv().await;
    assert_eq!(resp["id"], 1);
    assert_eq!(resp["translated"], "Привет");

    mock.send(&serde_json::json!({"cmd": "exit"})).await;
    mock.kill().await;
}

#[tokio::test]
async fn test_sidecar_status() {
    let mut mock = MockSidecar::new().await;

    mock.send(&serde_json::json!({
        "cmd": "init",
        "protocol_version": "1.0"
    }))
    .await;
    let _ = mock.recv().await;

    mock.send(&serde_json::json!({
        "cmd": "status",
        "id": 42
    }))
    .await;

    let resp = mock.recv().await;
    assert_eq!(resp["id"], 42);
    assert_eq!(resp["ready"], true);
    assert!(resp["languages"].is_array());

    mock.send(&serde_json::json!({"cmd": "exit"})).await;
    mock.kill().await;
}

#[tokio::test]
async fn test_sidecar_heartbeat() {
    let mut mock = MockSidecar::new().await;

    mock.send(&serde_json::json!({
        "cmd": "init",
        "protocol_version": "1.0"
    }))
    .await;
    let _ = mock.recv().await;

    let got_heartbeat = timeout(Duration::from_secs(35), async {
        loop {
            let line = mock.lines.next_line().await.unwrap().unwrap();
            let resp: serde_json::Value = serde_json::from_str(&line).unwrap();
            if resp.get("type").and_then(|v| v.as_str()) == Some("heartbeat") {
                return true;
            }
        }
    })
    .await;

    assert!(got_heartbeat.is_ok());

    mock.send(&serde_json::json!({"cmd": "exit"})).await;
    mock.kill().await;
}

#[tokio::test]
async fn test_sidecar_concurrent_requests() {
    let mut mock = MockSidecar::new().await;

    mock.send(&serde_json::json!({
        "cmd": "init",
        "protocol_version": "1.0"
    }))
    .await;
    let _ = mock.recv().await;

    let n = 20;
    for i in 0..n {
        mock.send(&serde_json::json!({
            "cmd": "translate",
            "id": i + 1,
            "q": format!("test {}", i),
            "source": "en",
            "target": "ru"
        }))
        .await;
    }

    let mut received_ids = std::collections::HashSet::new();
    for _ in 0..n {
        let resp = timeout(Duration::from_secs(30), mock.recv())
            .await
            .expect("timeout waiting for response");
        if let Some(id) = resp.get("id").and_then(|v| v.as_u64()) {
            received_ids.insert(id);
        }
    }

    assert_eq!(received_ids.len(), n, "all {n} responses should have unique ids");

    mock.send(&serde_json::json!({"cmd": "exit"})).await;
    mock.kill().await;
}

#[tokio::test]
async fn test_sidecar_graceful_shutdown() {
    let mut mock = MockSidecar::new().await;

    mock.send(&serde_json::json!({
        "cmd": "init",
        "protocol_version": "1.0"
    }))
    .await;
    let _ = mock.recv().await;

    mock.send(&serde_json::json!({"cmd": "exit"})).await;

    let resp = mock.recv().await;
    assert_eq!(resp["type"], "shutdown");
    assert_eq!(resp["reason"], "graceful");
}

#[tokio::test]
async fn test_sidecar_protocol_version_mismatch() {
    let mut mock = MockSidecar::new().await;

    mock.send(&serde_json::json!({
        "cmd": "exit",
        "reason": "protocol_version_mismatch"
    }))
    .await;

    let resp = mock.recv().await;
    assert_eq!(resp["type"], "shutdown");
    assert_eq!(resp["reason"], "protocol_version_mismatch");
    mock.kill().await;
}

#[tokio::test]
async fn test_sidecar_unknown_language() {
    let mut mock = MockSidecar::new().await;

    mock.send(&serde_json::json!({
        "cmd": "init",
        "protocol_version": "1.0"
    }))
    .await;
    let _ = mock.recv().await;

    mock.send(&serde_json::json!({
        "cmd": "translate",
        "id": 99,
        "q": "hello",
        "source": "fr",
        "target": "de"
    }))
    .await;

    let resp = mock.recv().await;
    assert_eq!(resp["id"], 99);
    assert!(resp["translated"].is_string());

    mock.send(&serde_json::json!({"cmd": "exit"})).await;
    mock.kill().await;
}
