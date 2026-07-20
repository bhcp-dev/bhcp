use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use bhcp::cbor::{decode_deterministic, encode_deterministic};
use bhcp::hash::{HashAlgorithm, format_hash};
use bhcp::model::ContentReference;
use bhcp::value::Value;

const MAX_REQUEST_BYTES: u64 = 1024 * 1024;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::from(90),
    }
}

fn run() -> Result<(), ()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.as_slice() == ["judge-change-policy"] {
        return judge_change_policy();
    }
    let [mode, task, verifier] = arguments.as_slice() else {
        return Err(());
    };
    if mode != "verify" {
        return Err(());
    }
    let expected_verifier = verifier_symbol(task, verifier).ok_or(())?;
    let expected_source = expected_source_hash(task).ok_or(())?;

    let mut request_bytes = Vec::new();
    io::stdin()
        .take(MAX_REQUEST_BYTES + 1)
        .read_to_end(&mut request_bytes)
        .map_err(|_| ())?;
    if request_bytes.len() as u64 > MAX_REQUEST_BYTES {
        return Err(());
    }
    let request = decode_deterministic(&request_bytes).map_err(|_| ())?;
    if request.get("version") != Some(&Value::Text("bhcp/adapter-request@0".to_owned()))
        || request.get("verifier") != Some(&Value::Text(expected_verifier.to_owned()))
    {
        return Err(());
    }
    let Value::Bytes(subject_bytes) = request.get("subject_content").ok_or(())? else {
        return Err(());
    };
    let subject = ContentReference::from_bytes(
        "application/vnd.bhcp.subject-source",
        subject_bytes,
        HashAlgorithm::default(),
    );
    if request.get("subject") != Some(&subject.to_value()) {
        return Err(());
    }
    let subject_hash = format_hash(&HashAlgorithm::default().hash(subject_bytes));
    let accepted = subject_hash == expected_source;
    let payload = encode_deterministic(&Value::map([
        ("task", Value::Text(task.clone())),
        ("verifier", Value::Text(verifier.clone())),
        ("subject", Value::Text(subject_hash)),
    ]))
    .map_err(|_| ())?;
    let response = Value::map([
        ("version", Value::Text("bhcp/adapter-result@0".to_owned())),
        (
            "state",
            Value::Text(if accepted { "accepted" } else { "rejected" }.to_owned()),
        ),
        (
            "media_type",
            Value::Text("application/vnd.bhcp.evidence-generalization+cbor".to_owned()),
        ),
        ("payload", Value::Bytes(payload)),
        ("trust", Value::Array(vec![])),
    ]);
    io::stdout()
        .write_all(&encode_deterministic(&response).map_err(|_| ())?)
        .map_err(|_| ())
}

fn judge_change_policy() -> Result<(), ()> {
    let source = fs::read("subject/src/lib.rs").map_err(|_| ())?;
    if source.is_empty() || source.len() > 1024 * 1024 {
        return Err(());
    }
    Ok(())
}

fn verifier_symbol(task: &str, verifier: &str) -> Option<&'static str> {
    match (task, verifier) {
        ("atomic-batch", "public")
        | ("tenant-policy", "public")
        | ("contextual-policy", "public")
        | ("in-session-evidence", "public") => Some("experiment/verifier/public-rust@0"),
        ("atomic-batch", "oracle") => Some("experiment/verifier/ledger-invariants@0"),
        ("tenant-policy", "oracle") => Some("experiment/verifier/policy-resolution@0"),
        ("contextual-policy", "oracle") => Some("experiment/verifier/contextual-policy@0"),
        ("in-session-evidence", "oracle") => Some("experiment/verifier/in-session-oracle@0"),
        ("atomic-batch", "change")
        | ("tenant-policy", "change")
        | ("contextual-policy", "change")
        | ("in-session-evidence", "change") => Some("experiment/verifier/change-policy@0"),
        _ => None,
    }
}

fn expected_source_hash(task: &str) -> Option<&'static str> {
    match task {
        "atomic-batch" => Some(concat!(
            "bhcp.hash/sha3-512@0:",
            "77f1d1a4659c6bf514fecd5d81b8b9580f3898b2a9a9db28c7450cde014f5c4",
            "5b45ada181a2540fd8d057fed254c9ec3bc0fd5ec02edf19f1179d8b3453317f5"
        )),
        "tenant-policy" => Some(concat!(
            "bhcp.hash/sha3-512@0:",
            "db8746afc1ff776f8bf77138f84b73dc35fc30e88f472309cf05b0ec6bf13ceb",
            "3f33bdd81e0f9d7aaa39a2757aff4c504e32660cca5e28d4069f56ab7f24a85f"
        )),
        "contextual-policy" => Some(concat!(
            "bhcp.hash/sha3-512@0:",
            "d5d10497b79b4b279d63ca53f9f0f7d54d8bcbbb47b1901d4e0ccfe7ed20acc",
            "d2a6802ccb9a1384d2099e4da675bc54c86532e8954a1cb664bec57c7e8c731a1"
        )),
        "in-session-evidence" => Some(concat!(
            "bhcp.hash/sha3-512@0:",
            "1449ecf1e22e4935dff9e2663fbcb9eb9a20e51709f8406a4fdac8290dad59f2",
            "a1f5d8c6f6e92c4094045ed9b31e81ad571583f7d2f461ee65167b0383f4fda7"
        )),
        _ => None,
    }
}
