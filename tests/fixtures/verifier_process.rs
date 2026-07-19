use std::env;
use std::io::{self, Read, Write};
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use bhcp::cbor::encode_deterministic;
use bhcp::value::Value;

fn main() -> ExitCode {
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() || input.is_empty() {
        return ExitCode::from(90);
    }
    let mode = env::args().nth(1).unwrap_or_default();
    match mode.as_str() {
        "accepted" | "rejected" => write_value(Value::map([
            ("version", Value::Text("bhcp/adapter-result@0".to_owned())),
            ("state", Value::Text(mode)),
            ("media_type", Value::Text("text/plain".to_owned())),
            ("payload", Value::Bytes(b"fixture evidence".to_vec())),
            ("trust", Value::Array(vec![])),
        ])),
        "unresolved" | "faulted" => write_value(Value::map([
            ("version", Value::Text("bhcp/adapter-result@0".to_owned())),
            ("state", Value::Text(mode.clone())),
            (
                "reason",
                Value::map([
                    (
                        "code",
                        Value::Text(format!("example/{mode}.fixture@0")),
                    ),
                    ("message", Value::Text(format!("fixture {mode}"))),
                ]),
            ),
        ])),
        "malformed" => {
            let _ = io::stdout().write_all(b"not deterministic cbor");
            ExitCode::SUCCESS
        }
        "nonzero" => ExitCode::from(17),
        "flood" => {
            let chunk = [b'x'; 8192];
            for _ in 0..512 {
                if io::stdout().write_all(&chunk).is_err() {
                    break;
                }
            }
            ExitCode::SUCCESS
        }
        "stderr-flood" => {
            let chunk = [b'e'; 8192];
            for _ in 0..128 {
                if io::stderr().write_all(&chunk).is_err() {
                    break;
                }
            }
            write_value(Value::map([
                ("version", Value::Text("bhcp/adapter-result@0".to_owned())),
                ("state", Value::Text("accepted".to_owned())),
                ("media_type", Value::Text("text/plain".to_owned())),
                ("payload", Value::Bytes(b"fixture evidence".to_vec())),
                ("trust", Value::Array(vec![])),
            ]))
        }
        "sleep" => {
            thread::sleep(Duration::from_secs(5));
            ExitCode::SUCCESS
        }
        _ => ExitCode::from(91),
    }
}

fn write_value(value: Value) -> ExitCode {
    let bytes = match encode_deterministic(&value) {
        Ok(bytes) => bytes,
        Err(_) => return ExitCode::from(92),
    };
    if io::stdout().write_all(&bytes).is_err() {
        ExitCode::from(93)
    } else {
        ExitCode::SUCCESS
    }
}
