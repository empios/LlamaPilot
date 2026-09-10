#![cfg(unix)]
use llamapilot_lib::{
    process::{self, CommandSpec},
    runtime::snapshot,
};
use std::os::unix::fs::{symlink, PermissionsExt};

#[tokio::test]
async fn snapshot_survives_source_removal_with_modes_and_versioned_libraries() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    std::fs::create_dir(&source).unwrap();
    let binary = source.join("llama-server");
    std::fs::write(&binary, "#!/bin/sh\necho relocated\n").unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(source.join("libggml.so.1.2"), "library fixture").unwrap();
    symlink("libggml.so.1.2", source.join("libggml.so.1")).unwrap();
    std::fs::write(source.join("ggml-metal.metallib"), "resource").unwrap();
    let target = dir.path().join("runtime");
    let stage = snapshot::stage_runtime(&source, &target).unwrap();
    stage.commit(&serde_json::json!({})).unwrap();
    std::fs::remove_dir_all(source).unwrap();
    assert_eq!(
        std::fs::read_to_string(target.join("libggml.so.1")).unwrap(),
        "library fixture"
    );
    assert!(target.join("ggml-metal.metallib").is_file());
    let result = process::capture(&CommandSpec::new(target.join("llama-server")))
        .await
        .unwrap();
    assert!(result.succeeded());
    assert_eq!(result.stdout.trim(), "relocated");
}
