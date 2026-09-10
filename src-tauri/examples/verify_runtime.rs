//! Build qualification helper: exercise the same toolchain, snapshot and inspection code as the app.
use llamapilot_lib::{
    build::detect, config::Settings, hardware, llama::discovery, runtime::snapshot,
};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let input = std::path::PathBuf::from(
        args.next()
            .ok_or("usage: verify_runtime <build-bin> <new-snapshot-directory>")?,
    );
    let output = std::path::PathBuf::from(args.next().ok_or("missing snapshot directory")?);
    let tools = detect::detect(&Settings::default()).await;
    println!("{}", serde_json::to_string_pretty(&tools)?);
    println!(
        "{}",
        serde_json::to_string_pretty(&hardware::snapshot().await)?
    );
    let stage = snapshot::stage_runtime(&input, &output)?;
    let inspection = discovery::inspect(&stage.executable(), None).await?;
    println!("{}", serde_json::to_string_pretty(&inspection)?);
    stage.commit(&serde_json::json!({ "qualification": true }))?;
    println!("Snapshot ready: {}", output.display());
    Ok(())
}
