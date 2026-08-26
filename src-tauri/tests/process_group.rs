//! Verifies that the Windows job object really tears down a whole process tree.
//!
//! This is the behaviour build cancellation depends on: terminating CMake alone would leave
//! MSBuild and its compilers running. The tests deliberately use a *detached* grandchild, which
//! is precisely the case a plain `child.kill()` fails to handle.

#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use llamapilot_lib::platform::ProcessGroup;
use tokio::process::Child;
use tokio::process::Command;

/// How long the grandchild waits before writing its marker, and how long tests wait for it.
const GRANDCHILD_DELAY: &str = "4";
const OBSERVATION_WINDOW: Duration = Duration::from_secs(7);

/// Writes a launcher that detaches a grandchild, which waits and then writes a marker file.
///
/// The work is split across two batch files so no quoting has to survive Rust's Windows argument
/// escaping, which `cmd.exe` does not interpret the same way.
fn write_scripts(directory: &Path) -> PathBuf {
    let launcher = directory.join("launch.cmd");
    let worker = directory.join("worker.cmd");

    std::fs::write(
        &worker,
        format!(
            "@echo off\r\nping -n {GRANDCHILD_DELAY} 127.0.0.1 >nul\r\necho ok > \"%~dp0survived.txt\"\r\n"
        ),
    )
    .expect("write worker script");

    std::fs::write(
        &launcher,
        "@echo off\r\nstart \"\" /B \"%~dp0worker.cmd\"\r\n",
    )
    .expect("write launcher script");

    launcher
}

fn spawn_launcher(launcher: &Path) -> Child {
    Command::new("cmd.exe")
        .arg("/C")
        .arg(launcher)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("cmd.exe should be spawnable")
}

#[tokio::test]
async fn the_marker_appears_when_the_tree_is_left_alone() {
    // Control case. Without it, the kill tests would pass even if the marker never appeared for
    // an unrelated reason, such as a malformed command line.
    let temp = tempfile::tempdir().expect("temp dir");
    let launcher = write_scripts(temp.path());
    let marker = temp.path().join("survived.txt");

    let mut child = spawn_launcher(&launcher);
    let _ = child.wait().await;

    tokio::time::sleep(OBSERVATION_WINDOW).await;

    assert!(
        marker.exists(),
        "the detached grandchild should have written its marker when nothing killed it"
    );
}

#[tokio::test]
async fn terminating_a_job_kills_detached_grandchildren() {
    let temp = tempfile::tempdir().expect("temp dir");
    let launcher = write_scripts(temp.path());
    let marker = temp.path().join("survived.txt");

    let group = ProcessGroup::new().expect("job object");
    let mut child = spawn_launcher(&launcher);
    group
        .adopt(child.id().expect("child pid"))
        .expect("child joins the job");

    // `start /B` returns immediately, so the launcher exits while the grandchild still waits.
    let _ = child.wait().await;

    group.terminate();
    tokio::time::sleep(OBSERVATION_WINDOW).await;

    assert!(
        !marker.exists(),
        "the detached grandchild survived job termination"
    );
}

#[tokio::test]
async fn a_process_adopted_after_cancellation_is_terminated_immediately() {
    // Covers the narrow race where Cancel lands after the runner checked the flag but before the
    // freshly spawned CMake process was assigned to the job object.
    let temp = tempfile::tempdir().expect("temp dir");
    let launcher = write_scripts(temp.path());
    let marker = temp.path().join("survived.txt");

    let group = ProcessGroup::new().expect("job object");
    group.terminate();

    let mut child = spawn_launcher(&launcher);
    group
        .adopt(child.id().expect("child pid"))
        .expect("cancelled job accepts and terminates the child");
    let _ = child.wait().await;

    tokio::time::sleep(OBSERVATION_WINDOW).await;
    assert!(
        !marker.exists(),
        "a child adopted after cancellation was allowed to keep running"
    );
}

#[tokio::test]
async fn dropping_a_job_also_kills_its_processes() {
    // KILL_ON_JOB_CLOSE is what stops a crash of the application leaving orphaned compilers.
    let temp = tempfile::tempdir().expect("temp dir");
    let launcher = write_scripts(temp.path());
    let marker = temp.path().join("survived.txt");

    {
        let group = ProcessGroup::new().expect("job object");
        let mut child = spawn_launcher(&launcher);
        group
            .adopt(child.id().expect("child pid"))
            .expect("child joins the job");
        let _ = child.wait().await;
    }

    tokio::time::sleep(OBSERVATION_WINDOW).await;

    assert!(
        !marker.exists(),
        "closing the job handle should have killed the tree"
    );
}
