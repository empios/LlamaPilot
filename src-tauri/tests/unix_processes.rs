#![cfg(unix)]
use llamapilot_lib::{
    platform::ProcessGroup,
    process::{self, CommandSpec},
};
use std::{
    path::Path,
    process::Stdio,
    time::{Duration, Instant},
};

async fn wait_file(path: &Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(text) = std::fs::read_to_string(path) {
            if !text.trim().is_empty() {
                return text;
            }
        }
        assert!(Instant::now() < deadline, "fixture did not become ready");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn cancellation_kills_descendants_and_closes_inherited_output() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("child.pid");
    let group = std::sync::Arc::new(ProcessGroup::new().unwrap());
    let running_group = group.clone();
    let spec = CommandSpec::new("/bin/sh")
        .args(["-c", "sleep 60 & echo $! > \"$1\"; wait", "fixture"])
        .arg(&marker);
    let task = tokio::spawn(async move { process::capture_in_group(&spec, &running_group).await });
    wait_file(&marker).await;
    group.terminate();
    let output = tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(!output.succeeded());
}

#[tokio::test]
async fn normal_exit_cleans_up_background_children() {
    let spec = CommandSpec::new("/bin/sh").args(["-c", "sleep 60 & echo done"]);
    let output = tokio::time::timeout(Duration::from_secs(10), process::capture(&spec))
        .await
        .unwrap()
        .unwrap();
    assert!(output.succeeded());
    assert_eq!(output.stdout.trim(), "done");
}

// Invoked as a subprocess by the crash test, never by the ordinary test runner.
#[test]
#[ignore]
fn watchdog_parent_fixture() {
    let marker = std::env::var_os("LLAMAPILOT_WATCHDOG_MARKER").expect("fixture marker");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let spec = CommandSpec::new("/bin/sh")
            .args([
                "-c",
                "echo $$ > \"$1\"; while :; do sleep 1; done",
                "fixture",
            ])
            .arg(marker);
        process::capture(&spec).await.unwrap();
    });
}

#[tokio::test]
async fn watchdog_cleans_up_after_parent_is_killed() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("owned.pid");
    let mut parent = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "watchdog_parent_fixture"])
        .env("LLAMAPILOT_WATCHDOG_MARKER", &marker)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let pid = wait_file(&marker).await.trim().parse::<i32>().unwrap();
    // Adoption follows spawn; wait for the parent's watcher to be installed before simulating a crash.
    tokio::time::sleep(Duration::from_millis(500)).await;
    parent.kill().unwrap();
    parent.wait().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        // A zombie has exited and cannot retain resources; Linux init in CI may reap later.
        let status = std::process::Command::new("ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .unwrap();
        let state = String::from_utf8_lossy(&status.stdout);
        if !status.status.success() || state.trim().is_empty() || state.trim().starts_with('Z') {
            break;
        }
        assert!(Instant::now() < deadline, "orphan remained alive: {pid}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
