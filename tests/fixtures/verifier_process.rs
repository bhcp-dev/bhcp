use std::env;
use std::io::{self, Read, Write};
use std::process::{Command, ExitCode};
use std::thread;
use std::time::Duration;
use std::{fs, net::TcpStream};

use bhcp::cbor::encode_deterministic;
use bhcp::value::Value;

fn main() -> ExitCode {
    let mode = env::args().nth(1).unwrap_or_default();
    if mode == "hold" {
        thread::sleep(Duration::from_secs(30));
        return ExitCode::SUCCESS;
    }
    let mut input = Vec::new();
    if io::stdin().read_to_end(&mut input).is_err() || input.is_empty() {
        return ExitCode::from(90);
    }
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
                    ("code", Value::Text(format!("example/{mode}.fixture@0"))),
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
            thread::sleep(Duration::from_secs(30));
            ExitCode::SUCCESS
        }
        "descendant" => {
            let executable = env::current_exe().unwrap();
            if Command::new(executable).arg("hold").spawn().is_err() {
                thread::sleep(Duration::from_secs(30));
            }
            ExitCode::SUCCESS
        }
        "network-denied" => isolation_result(matches!(
            TcpStream::connect("127.0.0.1:9"),
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied
        )),
        "exec-denied" => isolation_result(matches!(
            Command::new("/usr/bin/true").status(),
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied
        )),
        #[cfg(unix)]
        "fd-denied" => {
            let descriptor = env::args().nth(2).unwrap().parse::<i32>().unwrap();
            #[cfg(target_os = "linux")]
            let descriptor_path = format!("/proc/self/fd/{descriptor}");
            #[cfg(target_os = "macos")]
            let descriptor_path = format!("/dev/fd/{descriptor}");
            isolation_result(fs::read_link(descriptor_path).is_err())
        }
        "read-denied" | "read-allowed" => {
            let path = env::args().nth(2).unwrap_or_default();
            let succeeded = fs::read(path).is_ok();
            isolation_result(succeeded == (mode == "read-allowed"))
        }
        "write-denied" | "write-allowed" => {
            let path = env::args().nth(2).unwrap_or_default();
            let succeeded = fs::write(path, b"adapter write").is_ok();
            isolation_result(succeeded == (mode == "write-allowed"))
        }
        _ => ExitCode::from(91),
    }
}

fn isolation_result(enforced: bool) -> ExitCode {
    if enforced {
        write_value(Value::map([
            ("version", Value::Text("bhcp/adapter-result@0".to_owned())),
            ("state", Value::Text("accepted".to_owned())),
            ("media_type", Value::Text("text/plain".to_owned())),
            ("payload", Value::Bytes(b"isolation enforced".to_vec())),
            ("trust", Value::Array(vec![])),
        ]))
    } else {
        write_value(Value::map([
            ("version", Value::Text("bhcp/adapter-result@0".to_owned())),
            ("state", Value::Text("faulted".to_owned())),
            (
                "reason",
                Value::map([
                    ("code", Value::Text("example/isolation.leaked@0".to_owned())),
                    (
                        "message",
                        Value::Text("sandbox capability leaked".to_owned()),
                    ),
                ]),
            ),
        ]))
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
