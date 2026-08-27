use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        Some("--version") => {
            eprintln!("version: b9000-cafebabe (build 9000, commit cafebabe)");
            eprintln!("built with fixture compiler for fixture target");
        }
        Some("--help") => {
            println!("----- common params -----\n");
            println!("-m,    --model FNAME                    model path");
            println!("-a,    --alias STRING                   model names used by API");
            println!("--host HOST                             listen host");
            println!("--port PORT                             listen port");
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
        _ => return serve(&arguments),
    }
    ExitCode::SUCCESS
}

fn serve(arguments: &[String]) -> ExitCode {
    let Some(model) = argument(arguments, "--model") else {
        eprintln!("missing --model");
        return ExitCode::from(2);
    };
    if !Path::new(model).is_file() {
        eprintln!("model does not exist: {model}");
        return ExitCode::from(3);
    }

    let host = argument(arguments, "--host").unwrap_or("127.0.0.1");
    let Some(port) = argument(arguments, "--port").and_then(|value| value.parse::<u16>().ok())
    else {
        eprintln!("missing or invalid --port");
        return ExitCode::from(4);
    };
    let listener = match TcpListener::bind((host, port)) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("could not listen on {host}:{port}: {error}");
            return ExitCode::from(5);
        }
    };

    println!("load: model loaded from {model}");
    println!("main: server is listening on http://{host}:{port}");
    let _ = std::io::stdout().flush();

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            continue;
        };
        let request = read_request(&mut stream);
        let path = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/");
        let (status, content_type, body) = match path {
            "/health" => ("200 OK", "application/json", r#"{"status":"ok"}"#),
            "/props" => (
                "200 OK",
                "application/json",
                r#"{"total_slots":1,"build_info":"fixture-b9000"}"#,
            ),
            "/slots" => ("200 OK", "application/json", r#"[{"is_processing":false}]"#),
            "/metrics" => (
                "200 OK",
                "text/plain",
                "llamacpp:requests_processing 0\nllamacpp:predicted_tokens_seconds 12.5\n",
            ),
            "/completion" => (
                "200 OK",
                "application/json",
                r#"{"content":"review","truncated":false,"stop_type":"limit","timings":{"prompt_n":1024,"prompt_ms":2048.0,"prompt_per_second":500.0,"predicted_n":128,"predicted_ms":3200.0,"predicted_per_second":40.0}}"#,
            ),
            _ => (
                "404 Not Found",
                "application/json",
                r#"{"error":{"code":404,"message":"Not found"}}"#,
            ),
        };
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
    }

    ExitCode::SUCCESS
}

fn read_request(stream: &mut std::net::TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = stream.read(&mut buffer).unwrap_or(0);
        if count == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..count]);
        let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        if request.len() >= header_end + 4 + content_length {
            break;
        }
    }
    String::from_utf8_lossy(&request).into_owned()
}

fn argument<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    arguments
        .windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}
