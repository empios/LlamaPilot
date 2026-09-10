//! Run a real model through the application's supervisor, readiness and HTTP paths.
use llamapilot_lib::{
    process::CommandSpec,
    server::{ServerLaunch, ServerSupervisor},
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: verify_server <llama-server> <model.gguf> <log-directory>".into());
    }
    let port = std::net::TcpListener::bind("127.0.0.1:0")?
        .local_addr()?
        .port();
    let supervisor = ServerSupervisor::new(PathBuf::from(&args[2]))?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;
    let launch = ServerLaunch {
        profile_id: "qualification".into(),
        profile_name: "Qualification".into(),
        runtime_id: "qualification".into(),
        runtime_label: "Native runtime".into(),
        model_name: "Qualification model".into(),
        host: "127.0.0.1".into(),
        port,
        command: CommandSpec::new(&args[0])
            .arg("--model")
            .arg(&args[1])
            .args([
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--ctx-size",
                "512",
                "--n-gpu-layers",
                "99",
            ]),
        warnings: Vec::new(),
    };
    for attempt in 1..=2 {
        supervisor.start(launch.clone()).await?;
        let result: Result<(), Box<dyn std::error::Error>> = async {
            let deadline = Instant::now() + Duration::from_secs(120);
            loop {
                if let Ok(response) = client.get(format!("http://127.0.0.1:{port}/health")).send().await {
                    if response.status().is_success() { break; }
                }
                if Instant::now() > deadline { return Err("server readiness timeout".into()); }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            let response = client.post(format!("http://127.0.0.1:{port}/completion"))
                .json(&serde_json::json!({ "prompt": "Hello, my name is", "n_predict": 8, "temperature": 0, "seed": 1 }))
                .send().await?.error_for_status()?.json::<serde_json::Value>().await?;
            if response["content"].as_str().unwrap_or("").is_empty() { return Err("empty completion".into()); }
            println!("Attempt {attempt}: {}", serde_json::to_string(&response["timings"])?);
            Ok(())
        }.await;
        supervisor.stop().await?;
        supervisor
            .wait_until_stopped(Duration::from_secs(10))
            .await?;
        result?;
        println!("Attempt {attempt}: stopped successfully");
    }
    Ok(())
}
