use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use llamapilot_lib::build::profile::{BuildBackend, BuildConfiguration};
use llamapilot_lib::llama::{artifacts, discovery};
use llamapilot_lib::models::ModelCatalogService;
use llamapilot_lib::process::CommandSpec;
use llamapilot_lib::profiles::{
    build_command_preview, ProfileInput, ProfileOptionSetting, ProfileRepository,
    ResolvedProfileTarget,
};
use llamapilot_lib::runtime::RuntimeRecord;
use llamapilot_lib::server::{ServerLaunch, ServerLifecycleState, ServerSupervisor};

#[tokio::test]
async fn serves_a_discovered_model_through_a_persisted_profile() {
    let temp = tempfile::tempdir().expect("temporary workflow directory");
    let model_directory = temp.path().join("models");
    let runtime_directory = temp.path().join("runtime");
    let profiles_directory = temp.path().join("profiles");
    let logs_directory = temp.path().join("logs");
    for directory in [
        &model_directory,
        &runtime_directory,
        &profiles_directory,
        &logs_directory,
    ] {
        fs::create_dir_all(directory).expect("workflow directory");
    }

    let model_path = model_directory.join("tiny.gguf");
    write_model(&model_path);
    let model_service = ModelCatalogService::load(
        temp.path().join("model-cache.json"),
        temp.path().join("model-overrides.json"),
    )
    .expect("model service");
    let catalog = model_service
        .scan(std::slice::from_ref(&model_directory))
        .expect("model catalog");
    let model = catalog.models.first().expect("discovered model");
    assert!(model.complete);

    let executable = compile_fixture_server(&runtime_directory);
    let inspection = discovery::inspect(&executable, None)
        .await
        .expect("runtime inspection");
    let capability_summary =
        artifacts::persist(&runtime_directory, &inspection).expect("capability sidecar");
    let executable_metadata = fs::metadata(&executable).expect("runtime metadata");
    let runtime = RuntimeRecord {
        id: "runtime-workflow".into(),
        source_id: "source-workflow".into(),
        source_name: "fixture llama.cpp".into(),
        repository: "https://example.invalid/llama.cpp".into(),
        commit: "cafebabe".into(),
        short_commit: "cafebab".into(),
        branch: "fixture".into(),
        backend: BuildBackend::Cpu,
        configuration: BuildConfiguration::Release,
        generator: "rustc fixture".into(),
        build_date: "2026-08-26T12:00:00Z".into(),
        directory: runtime_directory,
        executable,
        size_bytes: executable_metadata.len(),
        file_count: 1,
        capabilities: Some(capability_summary),
    };

    let port = available_port();
    let target = ResolvedProfileTarget {
        runtime_label: runtime.label(),
        model_name: model.display_name.clone(),
        model_path: model_path.clone(),
        projector_path: None,
    };
    let profile_input = ProfileInput {
        name: "Managed workflow".into(),
        description: Some("Integration fixture".into()),
        runtime_id: runtime.id.clone(),
        model_id: model.id.clone(),
        host: "127.0.0.1".into(),
        port,
        auto_select_port: false,
        options: BTreeMap::from([(
            "modelAlias".into(),
            ProfileOptionSetting::Custom {
                value: "coding-agent-model".into(),
            },
        )]),
        environment: BTreeMap::new(),
        additional_arguments: Vec::new(),
    };
    let profiles = ProfileRepository::new(profiles_directory);
    let created = profiles
        .create(profile_input, target.clone())
        .expect("persisted profile");
    let profile = profiles.find(&created.id).expect("reloaded profile");

    let preview =
        build_command_preview(profile.input(), &runtime, &inspection.capabilities, &target)
            .expect("command preview");
    assert!(preview
        .arguments
        .windows(2)
        .any(|pair| pair[0] == "--model" && pair[1] == model_path.to_string_lossy()));
    assert!(preview
        .arguments
        .windows(2)
        .any(|pair| pair == ["--alias", "coding-agent-model"]));

    let command = preview.environment.iter().fold(
        CommandSpec::new(&preview.program).args(&preview.arguments),
        |command, (key, value)| command.env(key, value),
    );
    let supervisor = ServerSupervisor::new(logs_directory).expect("server supervisor");
    supervisor
        .start(ServerLaunch {
            profile_id: profile.id.clone(),
            profile_name: profile.name.clone(),
            runtime_id: runtime.id.clone(),
            runtime_label: runtime.label(),
            model_name: model.display_name.clone(),
            host: profile.host.clone(),
            port: profile.port,
            command,
            warnings: preview.warnings,
        })
        .await
        .expect("server start");

    let ready = wait_for_state(&supervisor, ServerLifecycleState::Ready).await;
    assert_eq!(ready.health_status, Some(200));
    assert!(ready.pid.is_some());

    let telemetry = wait_for_telemetry(&supervisor).await;
    assert_eq!(telemetry.telemetry.props_available, Some(true));
    assert_eq!(telemetry.telemetry.total_slots, Some(1));
    assert_eq!(telemetry.telemetry.predicted_tokens_per_second, Some(12.5));
    assert!(supervisor
        .logs_snapshot()
        .entries
        .iter()
        .any(|entry| entry.text.contains("server is listening")));

    supervisor.stop().await.expect("server stop request");
    supervisor
        .wait_until_stopped(Duration::from_secs(5))
        .await
        .expect("server stopped");
    assert_eq!(
        supervisor.snapshot().await.state,
        ServerLifecycleState::Stopped
    );

    profiles.delete(&profile.id).expect("profile deletion");
    assert!(profiles.list().expect("empty profile list").is_empty());
}

fn compile_fixture_server(runtime_directory: &Path) -> std::path::PathBuf {
    let executable = runtime_directory.join(if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    });
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/llama/fake-server.rs");
    let rustc = which::which("rustc").expect("rustc is available while Cargo tests run");
    let status = Command::new(rustc)
        .arg(source)
        .arg("-o")
        .arg(&executable)
        .status()
        .expect("compile fixture server");
    assert!(status.success(), "fixture server compiles");
    executable
}

fn available_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .expect("available local port")
        .port()
}

async fn wait_for_state(
    supervisor: &std::sync::Arc<ServerSupervisor>,
    expected: ServerLifecycleState,
) -> llamapilot_lib::server::ServerSnapshot {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        let snapshot = supervisor.snapshot().await;
        if snapshot.state == expected {
            return snapshot;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "server did not reach {expected:?}; last snapshot: {snapshot:?}"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn wait_for_telemetry(
    supervisor: &std::sync::Arc<ServerSupervisor>,
) -> llamapilot_lib::server::ServerSnapshot {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        let snapshot = supervisor.snapshot().await;
        if snapshot.telemetry.metrics_available == Some(true) {
            return snapshot;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "server telemetry did not arrive; last snapshot: {snapshot:?}"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn write_string(buffer: &mut Vec<u8>, value: &str) {
    buffer.extend_from_slice(&(value.len() as u64).to_le_bytes());
    buffer.extend_from_slice(value.as_bytes());
}

fn write_model(path: &Path) {
    let entries = [
        ("general.type", "model"),
        ("general.name", "Managed Tiny Model"),
        ("general.architecture", "llama"),
    ];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    bytes.extend_from_slice(&3_u32.to_le_bytes());
    bytes.extend_from_slice(&0_i64.to_le_bytes());
    bytes.extend_from_slice(&(entries.len() as i64).to_le_bytes());
    for (key, value) in entries {
        write_string(&mut bytes, key);
        bytes.extend_from_slice(&8_u32.to_le_bytes());
        write_string(&mut bytes, value);
    }
    let mut file = fs::File::create(path).expect("model fixture");
    file.write_all(&bytes).expect("write model fixture");
}
