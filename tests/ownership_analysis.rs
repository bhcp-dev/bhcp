use std::fs;
use std::path::PathBuf;

use bhcp::hash::HashAlgorithm;
use bhcp::kernel::KernelRuntime;
use bhcp::model::ContentReference;
use bhcp::ownership::{
    OwnershipPolicy, PersistentShareApproval, analyze_program, analyze_program_with_policy,
};
use bhcp::parser::parse_canonical;
use bhcp::pipeline::compile_source;
use bhcp::value::Value;

fn analyze(source: &str) -> bhcp::diagnostic::Result<bhcp::ownership::OwnershipReport> {
    let source_name = "ownership.bhcp";
    let source_ref = ContentReference {
        media_type: "text/bhcp;profile=bhcp%2Fcanonical%400".to_owned(),
        size: source.len(),
        digests: vec![HashAlgorithm::default().hash(source.as_bytes())],
    };
    let program = parse_canonical(source, source_name, source_ref)?;
    analyze_program(&program, source_name)
}

fn fixture(name: &str) -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("conformance/v0/fixtures")
            .join(name),
    )
    .unwrap()
}

const RESOURCE: &str = "example/File@0";

fn goals(read_access: &str, second_access: &str, body: &str) -> String {
    format!(
        r#"
§goal example/Read@0 {{
    §input file: borrowed {read_access} 'scope {RESOURCE};
}}

§goal example/Second@0 {{
    §input file: borrowed {second_access} 'scope {RESOURCE};
}}

§goal example/Parent@0 {{
    §input file: owned write affine 'scope {RESOURCE};
    {body}
}}
"#
    )
}

#[test]
fn own_01_overlapping_read_borrows_are_accepted() {
    let source = fixture("own-01-read-overlap.bhcp");
    let report = analyze(&source).unwrap();
    assert_eq!(report.checked_goals, 3);
    assert_eq!(report.checked_bindings, 3);
}

#[test]
fn own_02_write_borrow_conflicts_with_an_overlapping_read() {
    let source = fixture("own-02-write-conflict.bhcp");
    let diagnostic = analyze(&source).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4402");
    assert!(diagnostic.message.contains("file"));
    assert!(diagnostic.message.contains("reader"));
    assert!(diagnostic.message.contains("writer"));
    assert!(diagnostic.message.contains("conformance/File@0"));
    assert!(diagnostic.message.contains("scope"));
    assert!(diagnostic.message.contains("ownership.bhcp:"));
}

#[test]
fn own_03_move_then_reuse_is_rejected_after_a_nested_branch_join() {
    let source = fixture("own-03-use-after-move.bhcp");
    let diagnostic = analyze(&source).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4403");
    assert!(diagnostic.message.contains("file"));
    assert!(diagnostic.message.contains("take"));
    assert!(diagnostic.message.contains("later"));
    assert!(diagnostic.message.contains("scope"));
    assert!(diagnostic.message.contains("conformance/File@0"));
}

#[test]
fn own_04_persistent_state_rejects_an_expiring_borrow() {
    let source = fixture("own-04-expired-retention.bhcp");
    let diagnostic = analyze(&source).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4404");
    assert!(diagnostic.message.contains("retained"));
    assert!(diagnostic.message.contains("file"));
    assert!(diagnostic.message.contains("request"));
}

#[test]
fn linear_values_must_be_consumed_on_every_control_flow_outcome() {
    let source = format!(
        r#"
§goal example/Consume@0 {{
    §input file: owned write linear 'scope {RESOURCE};
}}

§goal example/Noop@0 {{}}

§goal example/Parent@0 {{
    §input file: owned write linear 'scope {RESOURCE};
    §any {{
        take = example/Consume@0(file = move file);
        skip = example/Noop@0();
    }};
}}
"#
    );
    let diagnostic = analyze(&source).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4405");
    assert!(diagnostic.message.contains("file"));
    assert!(diagnostic.message.contains("take"));
    assert!(diagnostic.message.contains("skip"));
}

#[test]
fn invalid_mode_and_lifetime_crossings_fail_closed() {
    let cases = [
        (
            goals(
                "read",
                "read",
                r#"§gate when true {
        child = example/Read@0(file = share file);
    };"#,
            ),
            "BHCP4401",
            "share",
        ),
        (
            format!(
                r#"
§goal example/Read@0 {{
    §input file: borrowed read 'child {RESOURCE};
}}
§goal example/Parent@0 {{
    §input file: borrowed read 'parent {RESOURCE};
    §gate when true {{
        child = example/Read@0(file = borrow file);
    }};
}}
"#
            ),
            "BHCP4404",
            "parent",
        ),
    ];
    for (source, code, message) in cases {
        let diagnostic = analyze(&source).unwrap_err();
        assert_eq!(diagnostic.code, code, "{source}");
        assert!(diagnostic.message.contains(message), "{diagnostic}");
    }
}

#[test]
fn rejected_ownership_never_reaches_semantic_ir_emission() {
    let source = goals(
        "read",
        "write",
        r#"§all {
        reader = example/Read@0(file = borrow file);
        writer = example/Second@0(file = borrow file);
    };"#,
    );
    let diagnostic = compile_source(&source, "ownership.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4402");
}

#[test]
fn executable_handle_qualifiers_are_fully_materialized_in_ir() {
    let source = format!(
        r#"
§goal example/Inspect@0 {{
    §input file: borrowed read affine {RESOURCE};
}}

§goal example/Parent@0 {{
    §input file: owned {RESOURCE};
    §gate when true {{
        child = example/Inspect@0(file = borrow file);
    }};
}}
"#
    );
    let compiled = compile_source(&source, "ownership.bhcp").unwrap();
    let Value::Array(record) = compiled.ir.goals[1].input.to_value() else {
        panic!("goal input must be a record type")
    };
    let Value::Array(fields) = &record[2] else {
        panic!("record fields must be an array")
    };
    let Value::Array(field) = &fields[0] else {
        panic!("record field must be an array")
    };
    let Value::Array(file) = &field[1] else {
        panic!("field type must be an array")
    };
    assert_eq!(file[0], Value::Text("handle".to_owned()));
    assert_eq!(file[1], Value::Text("owned".to_owned()));
    assert_eq!(file[2], Value::Text("write".to_owned()));
    assert_eq!(file[3], Value::Text("unrestricted".to_owned()));
    assert_eq!(file[4], Value::Text("goal".to_owned()));
    assert!(
        compiled
            .ir
            .features
            .contains(&"bhcp/feature.ownership-analysis@0".to_owned())
    );
}

#[test]
fn share_value_move_and_recursive_boundaries_are_checked() {
    let valid_share = format!(
        r#"
§goal example/SharedChild@0 {{
    §input file: shared read 'scope {RESOURCE};
}}
§goal example/SharedParent@0 {{
    §input file: shared read 'scope {RESOURCE};
    child = example/SharedChild@0(file = share file);
}}
"#
    );
    analyze(&valid_share).unwrap();

    let recursive_move = format!(
        r#"
§goal example/Recur@0 {{
    §input file: owned write affine 'scope {RESOURCE};
    next = example/Recur@0(file = move file);
}}
"#
    );
    analyze(&recursive_move).unwrap();

    for (source, expected) in [
        (
            format!(
                r#"
§goal example/Child@0 {{ §input file: shared read 'scope {RESOURCE}; }}
§goal example/Parent@0 {{
    §input file: shared read 'scope {RESOURCE};
    child = example/Child@0(file = move file);
}}
"#
            ),
            "move",
        ),
        (
            format!(
                r#"
§goal example/Child@0 {{ §input file: owned write affine 'scope {RESOURCE}; }}
§goal example/Parent@0 {{
    §input file: owned write affine 'scope {RESOURCE};
    child = example/Child@0(file = file);
}}
"#
            ),
            "explicitly",
        ),
    ] {
        let diagnostic = analyze(&source).unwrap_err();
        assert_eq!(diagnostic.code, "BHCP4401");
        assert!(diagnostic.message.contains(expected), "{diagnostic}");
    }
}

#[test]
fn variant_resources_and_case_fixtures_keep_independent_scopes() {
    let source = format!(
        r#"
§goal example/Cases@0 {{
    §input file: owned write affine 'case variant {{ Present({RESOURCE}), Empty }};
    §case "first": {{ selected = file; expect completed satisfied; }};
    §case "second": {{ selected = file; expect completed satisfied; }};
}}
"#
    );
    let report = analyze(&source).unwrap();
    assert_eq!(report.checked_bindings, 1);
}

#[test]
fn persistent_shares_require_an_exact_policy_approval() {
    let source = format!(
        r#"
§goal example/Retain@0 {{
    §input file: shared read 'release {RESOURCE};
    §state retained: shared read 'release {RESOURCE} = file;
}}
"#
    );
    let source_name = "ownership.bhcp";
    let source_ref = ContentReference {
        media_type: "text/bhcp;profile=bhcp%2Fcanonical%400".to_owned(),
        size: source.len(),
        digests: vec![HashAlgorithm::default().hash(source.as_bytes())],
    };
    let program = parse_canonical(&source, source_name, source_ref).unwrap();
    assert_eq!(
        analyze_program(&program, source_name).unwrap_err().code,
        "BHCP4404"
    );
    let policy = OwnershipPolicy {
        persistent_shares: [PersistentShareApproval {
            goal: "example/Retain@0".to_owned(),
            binding: "file".to_owned(),
            lifetime: "release".to_owned(),
        }]
        .into_iter()
        .collect(),
    };
    analyze_program_with_policy(&program, source_name, &policy).unwrap();
}

#[test]
fn shares_cannot_extend_a_resource_lifetime() {
    let source = format!(
        r#"
§goal example/Child@0 {{
    §input file: shared read 'long {RESOURCE};
}}
§goal example/Parent@0 {{
    §input file: owned write affine 'short {RESOURCE};
    §gate when true {{
        child = example/Child@0(file = share file);
    }};
}}
"#
    );
    let diagnostic = compile_source(&source, "ownership.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4404");
    assert!(diagnostic.message.contains("short"));
    assert!(diagnostic.message.contains("long"));
}

#[test]
fn sequential_accesses_inside_one_concurrent_branch_do_not_overlap() {
    let source = format!(
        r#"
§goal example/Read@0 {{
    §input file: borrowed read 'scope {RESOURCE};
}}
§goal example/Write@0 {{
    §input file: borrowed write 'scope {RESOURCE};
}}
§goal example/Noop@0 {{}}
§goal example/Parent@0 {{
    §input file: owned write affine 'scope {RESOURCE};
    §all {{
        serial = §chain {{
            first = example/Write@0(file = borrow file);
            second = example/Read@0(file = borrow file);
        }};
        other = example/Noop@0();
    }};
}}
"#
    );
    analyze(&source).expect("ordered accesses within one all branch must not conflict");
}

#[test]
fn nested_sequences_still_conflict_with_separate_concurrent_branches() {
    let source = format!(
        r#"
§goal example/Read@0 {{
    §input file: borrowed read 'scope {RESOURCE};
}}
§goal example/Write@0 {{
    §input file: borrowed write 'scope {RESOURCE};
}}
§goal example/Parent@0 {{
    §input file: owned write affine 'scope {RESOURCE};
    §all {{
        serial = §chain {{
            first = example/Write@0(file = borrow file);
            second = example/Read@0(file = borrow file);
        }};
        parallel = example/Read@0(file = borrow file);
    }};
}}
"#
    );
    let diagnostic = analyze(&source).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4402");
    assert!(diagnostic.message.contains("first"));
    assert!(diagnostic.message.contains("parallel"));
}

#[test]
fn persistent_state_checks_handle_references_nested_in_expressions() {
    let source = format!(
        r#"
§goal example/Retain@0 {{
    §input file: borrowed read 'request {RESOURCE};
    §state retained: borrowed read 'request {RESOURCE} = if true then file else file;
}}
"#
    );
    let diagnostic = analyze(&source).unwrap_err();
    assert_eq!(diagnostic.code, "BHCP4404");
    assert!(diagnostic.message.contains("retained"));
    assert!(diagnostic.message.contains("file"));
}

#[test]
fn canonical_handle_references_are_accepted_by_executable_ir() {
    let source = format!(
        r#"
§goal example/Inspect@0 {{
    §input file: borrowed read 'scope {RESOURCE};
}}
§goal example/Parent@0 {{
    §input file: owned write affine 'scope {RESOURCE};
    §gate when true {{
        child = example/Inspect@0(file = borrow file);
    }};
}}
"#
    );
    let compiled = compile_source(&source, "ownership.bhcp").unwrap();
    let runtime = KernelRuntime::new(&compiled.ir);
    runtime
        .reduce(
            "network-1",
            Value::map([(
                "file",
                Value::map([("ref", Value::Text("file-1".to_owned()))]),
            )]),
            &[],
        )
        .expect("canonical handle reference must inhabit the executable handle type");
    for invalid in [
        Value::map([("file", Value::map([("ref", Value::Text(String::new()))]))]),
        Value::map([(
            "file",
            Value::map([
                ("ref", Value::Text("file-1".to_owned())),
                ("extra", Value::Bool(true)),
            ]),
        )]),
    ] {
        assert_eq!(
            runtime.reduce("network-1", invalid, &[]).unwrap_err().code,
            "BHCP4101"
        );
    }
}
