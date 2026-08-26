use std::process::Command;

use llama_control_lib::llama::discovery;

#[tokio::test]
async fn interrogates_a_real_process_and_preserves_both_streams() {
    let temp = tempfile::tempdir().expect("temp directory");
    let executable = temp.path().join(if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    });
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/llama/fake-server.rs");
    let rustc = which::which("rustc").expect("rustc is available while Cargo tests run");
    let status = Command::new(rustc)
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .status()
        .expect("compile fixture server");
    assert!(status.success(), "fixture server compiles");

    let inspection = discovery::inspect(&executable, None)
        .await
        .expect("discovery succeeds");

    assert_eq!(inspection.capabilities.version, "b9000-cafebabe");
    assert_eq!(inspection.capabilities.commit.as_deref(), Some("cafebabe"));
    assert_eq!(inspection.capabilities.devices.len(), 1);
    assert_eq!(inspection.capabilities.devices[0].id, "CUDA0");
    assert!(inspection.capabilities.options.contains_key("--brand-new"));
    assert_eq!(
        inspection.capabilities.speculative_types,
        ["none", "draft-simple", "ngram-cache"]
    );
    assert!(inspection.raw.version.stderr.contains("fixture compiler"));
    assert!(inspection.raw.help.stdout.contains("--ctx-size"));
    assert!(inspection.raw.devices.stderr.contains("fixture diagnostic"));
}
