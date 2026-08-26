use std::process::ExitCode;

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("--version") => {
            eprintln!("version: b9000-cafebabe (build 9000, commit cafebabe)");
            eprintln!("built with fixture compiler for fixture target");
        }
        Some("--help") => {
            println!("----- common params -----\n");
            println!("-c,    --ctx-size N                     context size");
            println!("--brand-new VALUE                      fixture-only unknown flag");
            println!("\n----- speculative params -----\n");
            println!("--spec-type none,draft-simple,ngram-cache");
            println!("                                        speculative decoding types");
        }
        Some("--list-devices") => {
            println!("Available devices:");
            println!("  CUDA0: Fixture GPU (8192 MiB, 6144 MiB free)");
            eprintln!("ggml_cuda_init: fixture diagnostic");
        }
        _ => return ExitCode::from(2),
    }
    ExitCode::SUCCESS
}
