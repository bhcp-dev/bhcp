use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::hash::{HashAlgorithm, format_hash};
use bhcp::value::Value;

const FINAL_SOURCE: &str = "pub fn public_ready() -> bool {\n    true\n}\n\npub fn oracle_ready() -> bool {\n    true\n}\n\npub fn policy_ready() -> bool {\n    true\n}\n";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::from(90),
    }
}

fn run() -> Result<(), ()> {
    let mode = env::args().nth(1).ok_or(())?;
    if mode == "judge-change-policy" {
        return if read_source()? == FINAL_SOURCE {
            Ok(())
        } else {
            Err(())
        };
    }
    let expected_verifier = match mode.as_str() {
        "public" => "experiment/verifier/public-rust@0",
        "oracle" => "experiment/verifier/in-session-oracle@0",
        "change-policy" => "experiment/verifier/change-policy@0",
        _ => return Err(()),
    };
    let mut request_bytes = Vec::new();
    io::stdin()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut request_bytes)
        .map_err(|_| ())?;
    let request = decode_deterministic(&request_bytes).map_err(|_| ())?;
    if request.get("version") != Some(&Value::Text("bhcp/adapter-request@0".to_owned()))
        || request.get("verifier") != Some(&Value::Text(expected_verifier.to_owned()))
    {
        return Err(());
    }
    let source = read_source()?;
    let compact: String = source.chars().filter(|character| !character.is_whitespace()).collect();
    let public = compact.contains("pubfnpublic_ready()->bool{true}");
    let oracle = compact.contains("pubfnoracle_ready()->bool{true}");
    let policy = compact.contains("pubfnpolicy_ready()->bool{true}");
    let accepted = match mode.as_str() {
        "public" => public,
        "oracle" => public && oracle && policy,
        "change-policy" => source == FINAL_SOURCE,
        _ => unreachable!(),
    };
    let payload = encode_deterministic(&Value::map([
        ("mode", Value::Text(mode)),
        (
            "subject",
            Value::Text(format_hash(&HashAlgorithm::default().hash(source.as_bytes()))),
        ),
    ]))
    .map_err(|_| ())?;
    let response = Value::map([
        (
            "version",
            Value::Text("bhcp/adapter-result@0".to_owned()),
        ),
        (
            "state",
            Value::Text(if accepted { "accepted" } else { "rejected" }.to_owned()),
        ),
        (
            "media_type",
            Value::Text("application/vnd.bhcp.in-session-evidence+cbor".to_owned()),
        ),
        ("payload", Value::Bytes(payload)),
        ("trust", Value::Array(vec![])),
    ]);
    io::stdout()
        .write_all(&encode_deterministic(&response).map_err(|_| ())?)
        .map_err(|_| ())
}

fn read_source() -> Result<String, ()> {
    fs::read_to_string("subject/src/lib.rs").map_err(|_| ())
}
