use bhcp::hash::{HashAlgorithm, SHA3_512};
use std::path::PathBuf;

use bhcp::manifest::{ProjectManifest, WorkingScope};

#[test]
fn defaults_to_sha3_and_rejects_unregistered_algorithms() {
    assert_eq!(
        ProjectManifest::parse("", "manifest")
            .unwrap()
            .identity_algorithm,
        HashAlgorithm::Sha3_512
    );
    assert_eq!(
        ProjectManifest::parse(&format!("identity_algorithm = \"{SHA3_512}\""), "manifest")
            .unwrap()
            .identity_algorithm,
        HashAlgorithm::Sha3_512
    );
    let error =
        ProjectManifest::parse("identity_algorithm = \"example/hash@0\"", "manifest").unwrap_err();
    assert_eq!(error.code, "BHCP6001");
}

fn adapter(symbol: &str, executable: &str, effects: &str) -> String {
    format!(
        r#"[[verifier_adapter]]
symbol = "{symbol}"
executable = "{executable}"
argv = ["verify", "--input", "-"]
working_scope = "project"
input_media_type = "application/vnd.bhcp.verification-request+cbor"
output_media_type = "application/vnd.bhcp.verifier-result+cbor"
timeout_ms = 30000
allowed_effects = [{effects}]
evidence_kind = "static"
"#
    )
}

#[test]
fn verifier_adapters_are_complete_and_normalized_deterministically() {
    let source = format!(
        "{}\n{}",
        adapter(
            "example/verifier.zeta@0",
            "target/verifiers/zeta",
            r#""bhcp-effect/process@0", "bhcp-effect/fs.read@0""#,
        ),
        adapter(
            "example/verifier.alpha@0",
            "tools/alpha-verifier",
            r#""bhcp-effect/fs.read@0""#,
        )
    );
    let manifest = ProjectManifest::parse(&source, "manifest").unwrap();
    assert_eq!(manifest.verifier_adapters.len(), 2);
    assert_eq!(
        manifest
            .verifier_adapters
            .iter()
            .map(|adapter| adapter.symbol.as_str())
            .collect::<Vec<_>>(),
        ["example/verifier.alpha@0", "example/verifier.zeta@0"]
    );

    let zeta = manifest.adapter("example/verifier.zeta@0").unwrap();
    assert_eq!(zeta.executable, PathBuf::from("target/verifiers/zeta"));
    assert_eq!(zeta.argv, ["verify", "--input", "-"]);
    assert_eq!(zeta.working_scope, WorkingScope::Project);
    assert_eq!(zeta.timeout_ms, 30_000);
    assert_eq!(
        zeta.allowed_effects,
        ["bhcp-effect/fs.read@0", "bhcp-effect/process@0"]
    );
    assert_eq!(zeta.evidence_kind, "static");
}

#[test]
fn verifier_adapters_reject_unknown_duplicate_or_incomplete_declarations() {
    for (source, expected) in [
        (
            adapter("example/verifier.test@0", "tools/verifier", "")
                .replace("timeout_ms = 30000\n", "mystery = \"value\"\n"),
            "unknown verifier-adapter key \"mystery\"",
        ),
        (
            adapter("example/verifier.test@0", "tools/verifier", "")
                .replace("timeout_ms = 30000", "timeout_ms = 1\ntimeout_ms = 2"),
            "duplicate timeout_ms",
        ),
        (
            format!(
                "{}\n{}",
                adapter("example/verifier.test@0", "tools/one", ""),
                adapter("example/verifier.test@0", "tools/two", "")
            ),
            "duplicate verifier symbol \"example/verifier.test@0\"",
        ),
        (
            adapter("example/verifier.test@0", "tools/verifier", "")
                .replace("argv = [\"verify\", \"--input\", \"-\"]\n", ""),
            "verifier adapter requires argv",
        ),
    ] {
        let error = ProjectManifest::parse(&source, "manifest").unwrap_err();
        assert_eq!(error.code, "BHCP6002");
        assert_eq!(error.message, expected);
    }
}

#[test]
fn verifier_adapters_reject_shells_path_escapes_and_ambient_network() {
    for (source, expected) in [
        (
            adapter("example/verifier.test@0", "/usr/bin/verifier", ""),
            "executable must stay within the project root",
        ),
        (
            adapter("example/verifier.test@0", "../verifier", ""),
            "executable must stay within the project root",
        ),
        (
            adapter("example/verifier.test@0", "tools/../verifier", ""),
            "executable must stay within the project root",
        ),
        (
            adapter("example/verifier.test@0", "sh", ""),
            "verifier adapter must not invoke a shell",
        ),
        (
            adapter("example/verifier.test@0", "tools/PwSh.ExE", ""),
            "verifier adapter must not invoke a shell",
        ),
        (
            adapter("example/verifier.test@0", "tools/verifier -c", ""),
            "executable must be one project-relative path, not a shell string",
        ),
        (
            adapter(
                "example/verifier.test@0",
                "tools/verifier",
                r#""bhcp-effect/network@0""#,
            ),
            "adapter effect \"bhcp-effect/network@0\" is not locally permitted",
        ),
    ] {
        let error = ProjectManifest::parse(&source, "manifest").unwrap_err();
        assert_eq!(error.code, "BHCP6002");
        assert_eq!(error.message, expected);
    }
}

#[test]
fn verifier_adapter_scalar_boundaries_fail_closed() {
    let baseline = adapter("example/verifier.test@0", "tools/verifier", "");
    for (source, expected) in [
        (
            baseline.replace("timeout_ms = 30000", "timeout_ms = 0"),
            "timeout_ms must be between 1 and 86400000",
        ),
        (
            baseline.replace("timeout_ms = 30000", "timeout_ms = \"30000\""),
            "timeout_ms must be an unquoted integer",
        ),
        (
            baseline.replace(
                "input_media_type = \"application/vnd.bhcp.verification-request+cbor\"",
                "input_media_type = \"not a media type\"",
            ),
            "invalid adapter media type",
        ),
        (
            baseline.replace(
                "input_media_type = \"application/vnd.bhcp.verification-request+cbor\"",
                "input_media_type = \"application/@invalid\"",
            ),
            "invalid adapter media type",
        ),
        (
            baseline.replace("evidence_kind = \"static\"", "evidence_kind = \"\""),
            "evidence_kind must be a registered class or symbol-id",
        ),
        (
            baseline.replace(
                "allowed_effects = []",
                "allowed_effects = [\"bhcp-effect/fs.read@0\", \"bhcp-effect/fs.read@0\"]",
            ),
            "duplicate adapter effect",
        ),
    ] {
        let error = ProjectManifest::parse(&source, "manifest").unwrap_err();
        assert_eq!(error.code, "BHCP6002");
        assert_eq!(error.message, expected);
    }
}

#[test]
fn canonical_and_local_verifier_fields_are_documented_separately() {
    let semantics = std::fs::read_to_string("SEMANTICS.md").unwrap();
    let readme = std::fs::read_to_string("README.md").unwrap();
    for required in [
        "Canonical verifier bindings own",
        "Project-local adapter declarations own",
        "MUST NOT use a shell",
        "MUST NOT grant ambient network",
    ] {
        assert!(semantics.contains(required), "SEMANTICS omitted {required}");
    }
    assert!(readme.contains("[[verifier_adapter]]"));
    assert!(readme.contains("target/verifiers/example"));
}

#[test]
fn discovers_root_manifest_from_a_nested_relative_source_path() {
    let manifest = ProjectManifest::discover(&PathBuf::from(
        "conformance/v0/fixtures/canonical-simple.bhcp",
    ))
    .unwrap();

    assert_eq!(manifest.identity_algorithm, HashAlgorithm::Sha3_512);
}
