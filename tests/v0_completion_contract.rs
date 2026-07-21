use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use bhcp::hash::{HashAlgorithm, format_hash};
use bhcp::model::ContentReference;
use bhcp::pipeline::{
    compile_source, compile_source_bytes_with_profile_registry_and_waivers,
    compile_source_with_policy, parse_policy_source, parse_profile_source,
};
use bhcp::policy::{ExactNumber, WaiverDocument, WaiverWeakening, apply_waiver, compose_policies};
use bhcp::profile::PresentationDocument;
use bhcp::schema::{parse_diagnostic, validate_root, validate_schema_inventory};
use bhcp::value::Value;

const ISSUE_KEYS: &[(u64, &str)] = &[
    (99, "v0-completion-contract"),
    (100, "definition-source-parser"),
    (101, "goal-source-parser"),
    (102, "governance-source-parser"),
    (103, "complete-type-checker"),
    (104, "expression-calculus"),
    (105, "function-predicate-elaboration"),
    (106, "ownership-analysis"),
    (107, "effect-authority-budget-analysis"),
    (108, "policy-waiver-lowering"),
    (109, "profile-source-lowering"),
    (110, "extension-lowering"),
    (111, "recursion-retention-lowering"),
    (112, "frontend-completion-audit"),
    (113, "graph-core-model"),
    (114, "obligation-graph-builder"),
    (115, "capability-graph-builder"),
    (116, "state-graph-builder"),
    (117, "obligation-proof-checker"),
    (118, "graph-completion-audit"),
    (119, "planner-boundary"),
    (120, "conflict-scheduler"),
    (121, "budget-retry-planner"),
    (122, "execution-graph-builder"),
    (123, "capability-executor-runtime"),
    (124, "state-cas-runtime"),
    (125, "execution-lifecycle-runtime"),
    (126, "evidence-graph-assembly"),
    (127, "runtime-policy-enforcement"),
    (128, "coding-agent-backend"),
    (129, "rust-sdk"),
    (130, "complete-cli"),
    (131, "complete-v0-conformance"),
    (132, "reference-program-e2e"),
    (133, "v0-security-audit"),
    (134, "v0-completion-audit"),
];

const STAGES: &[&str] = &[
    "source",
    "checking",
    "graphs",
    "planning",
    "execution",
    "evidence",
    "sdk",
    "cli",
    "conformance",
    "certification",
];

const ARTIFACTS: &[&str] = &[
    "program",
    "program-profiled",
    "program-contract",
    "registry",
    "policy",
    "waiver",
    "waiver-projection",
    "syntax",
    "syntax-projection",
    "profile",
    "profile-projection",
    "extension",
    "extension-projection",
    "extension-type-rule",
    "extension-effect-rule",
    "extension-policy-rule",
    "extension-normalization-rule",
    "extension-evidence-rule",
    "policy-evidence-registry",
    "planner-input",
    "execution-input",
    "expected-obligations",
    "outcome-matrix",
    "frontend-completion-report",
    "graph-identities",
    "graph-completion-report",
];

const FEATURES: &[&str] = &[
    "typed-functions-and-predicates",
    "nested-goals-and-recursion",
    "effects-and-ownership",
    "policy-boundary",
    "waiver-boundary",
    "custom-profile",
    "derived-extension",
    "planning",
    "execution",
    "per-obligation-evidence",
];

const NON_GOALS: &[&str] = &[
    "full-theorem-proving",
    "unrestricted-macros-or-grammar-plugins",
    "comprehensive-temporal-or-reactive-logic",
    "universal-workflow-synthesis",
];

const FRONTEND_LEDGER: &[(&str, &str, &str, &str)] = &[
    (
        "NUM-01",
        "S4",
        "tests/type_checker.rs::num_01_preserves_bits_rejects_overflow_and_requires_canonical_rationals",
        "schemas/v0/bhcp-v0.cddl",
    ),
    (
        "OWN-01",
        "S4",
        "tests/ownership_analysis.rs::own_01_overlapping_read_borrows_are_accepted",
        "conformance/v0/fixtures/own-01-read-overlap.bhcp",
    ),
    (
        "OWN-02",
        "S4",
        "tests/ownership_analysis.rs::own_02_write_borrow_conflicts_with_an_overlapping_read",
        "conformance/v0/fixtures/own-02-write-conflict.bhcp",
    ),
    (
        "OWN-03",
        "S4",
        "tests/ownership_analysis.rs::own_03_move_then_reuse_is_rejected_after_a_nested_branch_join",
        "conformance/v0/fixtures/own-03-use-after-move.bhcp",
    ),
    (
        "OWN-04",
        "S4",
        "tests/ownership_analysis.rs::own_04_persistent_state_rejects_an_expiring_borrow",
        "conformance/v0/fixtures/own-04-expired-retention.bhcp",
    ),
    (
        "PLN-03",
        "S4",
        "tests/type_checker.rs::every_v0_wire_type_has_a_closed_deterministic_checked_model",
        "schemas/v0/bhcp-v0.cddl",
    ),
    (
        "TYP-01",
        "S4",
        "tests/type_checker.rs::typ_01_infer_strict_materializes_types_without_implicit_dynamic",
        "schemas/v0/bhcp-v0.cddl",
    ),
    (
        "TYP-02",
        "S4",
        "tests/type_checker.rs::typ_02_03_dynamic_boundaries_require_and_materialize_runtime_checks",
        "schemas/v0/bhcp-v0.cddl",
    ),
    (
        "TYP-03",
        "S4",
        "tests/type_checker.rs::typ_02_03_dynamic_boundaries_require_and_materialize_runtime_checks",
        "schemas/v0/bhcp-v0.cddl",
    ),
    (
        "TYP-04",
        "S4",
        "tests/type_checker.rs::typ_04_05_nominal_identity_and_structural_width_are_distinct",
        "schemas/v0/bhcp-v0.cddl",
    ),
    (
        "TYP-05",
        "S4",
        "tests/type_checker.rs::typ_04_05_nominal_identity_and_structural_width_are_distinct",
        "schemas/v0/bhcp-v0.cddl",
    ),
    (
        "TYP-06",
        "S4",
        "tests/type_checker.rs::typ_06_refinement_introduction_requires_predicate_evidence",
        "schemas/v0/bhcp-v0.cddl",
    ),
    (
        "TYP-07",
        "S4",
        "tests/type_checker.rs::typ_07_08_option_and_result_preserve_explicit_tags_and_payloads",
        "schemas/v0/bhcp-v0.cddl",
    ),
    (
        "TYP-08",
        "S4",
        "tests/type_checker.rs::typ_07_08_option_and_result_preserve_explicit_tags_and_payloads",
        "schemas/v0/bhcp-v0.cddl",
    ),
    (
        "EXP-01",
        "S5",
        "tests/expression_calculus.rs::exp_01_static_finite_quantification_is_checked_and_deterministic",
        "schemas/v0/bhcp-v0.cddl",
    ),
    (
        "EXP-02",
        "S5",
        "tests/expression_calculus.rs::exp_02_unknown_calls_and_partial_arithmetic_fail_before_evaluation",
        "schemas/v0/bhcp-v0.cddl",
    ),
    (
        "EFF-01",
        "S6",
        "tests/effect_authority_analysis.rs::eff_01_child_effects_propagate_without_becoming_kernel_metadata",
        "conformance/v0/fixtures/canonical-all.ir.cbor",
    ),
    (
        "EFF-02",
        "S6",
        "tests/effect_authority_analysis.rs::eff_02_unsafe_effects_remain_visible_as_unresolved_evidence",
        "conformance/v0/fixtures/canonical-simple.ir.cbor",
    ),
    (
        "EFF-03",
        "S6",
        "tests/effect_authority_analysis.rs::eff_03_parent_prohibitions_and_explicit_ceilings_deny_child_excess",
        "conformance/v0/reference-program/program.bhcp",
    ),
    (
        "SYN-01",
        "S7",
        "tests/profile_source_lowering.rs::source_definitions_materialize_canonical_documents_and_a_validated_registry",
        "conformance/v0/reference-program/program.words.bhcp",
    ),
    (
        "SYN-02",
        "S7",
        "tests/goal_parser.rs::complete_goal_forms_build_a_closed_ordered_schema_valid_ast",
        "conformance/v0/reference-program/program.bhcp",
    ),
    (
        "SYN-03",
        "S7",
        "tests/pipeline.rs::complete_definition_forms_build_a_closed_schema_valid_ast",
        "conformance/v0/reference-program/program.bhcp",
    ),
    (
        "SYN-04",
        "S7",
        "tests/governance_parser.rs::complete_governance_forms_build_closed_ordered_schema_valid_ast",
        "conformance/v0/reference-program/policy.bhcp",
    ),
    (
        "SYN-05",
        "S7",
        "tests/profile_source_lowering.rs::source_definitions_materialize_canonical_documents_and_a_validated_registry",
        "conformance/v0/reference-program/profile.bhcp",
    ),
    (
        "SYN-06",
        "S7",
        "tests/profile_source_lowering.rs::invalid_source_registries_fail_atomically_before_custom_program_parsing",
        "conformance/v0/reference-program/syntax.bhcp",
    ),
    (
        "SYN-07",
        "S7",
        "tests/profile_source_lowering.rs::source_definitions_materialize_canonical_documents_and_a_validated_registry",
        "conformance/v0/reference-program/program.words.bhcp",
    ),
    (
        "KRN-11",
        "S8",
        "tests/recursion_retention_lowering.rs::recursive_children_retain_static_bounds_and_decreasing_measure_evidence",
        "conformance/v0/reference-program/program.bhcp",
    ),
    (
        "KRN-12",
        "S8",
        "tests/extension_lowering.rs::checked_in_reference_extension_is_executable_and_fully_lowered",
        "conformance/v0/reference-program/extension.bhcp",
    ),
    (
        "REC-01",
        "S8",
        "tests/recursion_retention_lowering.rs::recursive_children_retain_static_bounds_and_decreasing_measure_evidence",
        "conformance/v0/reference-program/program.bhcp",
    ),
    (
        "REC-02",
        "S8",
        "tests/recursion_retention_lowering.rs::unbounded_recursion_is_rejected_before_semantic_ir",
        "conformance/v0/reference-program/program.bhcp",
    ),
    (
        "REC-03",
        "S8",
        "tests/recursion_retention_lowering.rs::retained_ir_revalidates_recursion_metadata_against_the_recursive_edge",
        "conformance/v0/fixtures/canonical-gate.ir.cbor",
    ),
    (
        "EXT-01",
        "S9",
        "tests/extension_lowering.rs::derived_extension_executes_to_core_and_removes_its_meta_definition",
        "conformance/v0/reference-program/extension.bhcp",
    ),
    (
        "EXT-02",
        "S9",
        "tests/extension_lowering.rs::derived_lowerer_identity_is_unobservable_after_equivalent_execution",
        "conformance/v0/reference-program/extension.bhcp",
    ),
    (
        "EXT-03",
        "S9",
        "tests/extension_lowering.rs::missing_or_core_overriding_derived_extensions_fail_before_ir",
        "conformance/v0/reference-program/extension.bhcp",
    ),
    (
        "EXT-04",
        "S9",
        "tests/extension_lowering.rs::supported_native_extension_retains_exact_schema_payload_and_identity",
        "conformance/v0/reference-program/extension.diag",
    ),
    (
        "POL-01",
        "S9",
        "tests/policy_waiver_lowering.rs::source_policy_and_waiver_match_the_canonical_document_path",
        "conformance/v0/reference-program/policy.bhcp",
    ),
    (
        "POL-02",
        "S9",
        "tests/policy_waiver_lowering.rs::unwaived_inline_policy_enforces_goal_expressions_before_ir",
        "conformance/v0/reference-program/policy.bhcp",
    ),
    (
        "POL-03",
        "S9",
        "tests/policy_waiver_lowering.rs::governance_order_normalizes_and_unresolved_or_partial_waivers_fail_closed",
        "conformance/v0/reference-program/policy.bhcp",
    ),
    (
        "POL-04",
        "S9",
        "tests/policy_waiver_lowering.rs::all_six_source_waiver_categories_match_canonical_application",
        "conformance/v0/reference-program/waiver.bhcp",
    ),
    (
        "POL-05",
        "S9",
        "tests/policy_waiver_lowering.rs::inline_policy_composes_with_derived_extension_lowering",
        "conformance/v0/reference-program/extension.bhcp",
    ),
    (
        "POL-06",
        "S9",
        "tests/policy_waiver_lowering.rs::policy_and_waiver_presentation_is_artifact_only",
        "conformance/v0/reference-program/waiver.diag",
    ),
    (
        "POL-07",
        "S9",
        "tests/policy_waiver_lowering.rs::native_extension_only_ir_cannot_bypass_waiver_validation",
        "conformance/v0/reference-program/extension.diag",
    ),
    (
        "WAV-01",
        "S9",
        "tests/policy_waiver_lowering.rs::waiver_time_is_required_injected_and_fail_closed",
        "conformance/v0/reference-program/waiver.bhcp",
    ),
    (
        "WAV-02",
        "S9",
        "tests/policy_waiver_lowering.rs::delegated_source_authority_matches_canonical_application",
        "conformance/v0/reference-program/waiver.diag",
    ),
];

const GRAPH_APPLICABLE_SCENARIOS: &[&str] = &[
    "ALG-ALL-F",
    "ALG-ALL-R",
    "ALG-ALL-S",
    "ALG-ALL-U",
    "ALG-ANY-F",
    "ALG-ANY-R",
    "ALG-ANY-S",
    "ALG-ANY-U",
    "ALG-CHAIN-F",
    "ALG-CHAIN-R",
    "ALG-CHAIN-S",
    "ALG-CHAIN-U",
    "ALG-GATE-F",
    "ALG-GATE-R",
    "ALG-GATE-S",
    "ALG-GATE-U",
    "ALG-NONE-F",
    "ALG-NONE-R",
    "ALG-NONE-S",
    "ALG-NONE-U",
    "CBOR-02",
    "CBOR-03",
    "EFF-01",
    "EFF-02",
    "EFF-03",
    "ID-01",
    "ID-02",
    "ID-03",
    "ID-04",
    "KRN-01",
    "KRN-02",
    "KRN-03",
    "KRN-04",
    "KRN-05",
    "KRN-06",
    "KRN-07",
    "KRN-08",
    "KRN-09",
    "KRN-10",
    "KRN-11",
    "KRN-12",
    "KRN-13",
    "OWN-01",
    "OWN-02",
    "OWN-03",
    "OWN-04",
    "PLN-05",
    "POL-07",
    "POL-08",
    "REC-01",
    "REC-02",
    "REC-03",
    "RET-01",
    "STA-01",
    "STA-02",
    "WAV-01",
    "WAV-02",
];

const GRAPH_DIAGNOSTICS: &[&str] = &[
    "BHCP7001", "BHCP7002", "BHCP7003", "BHCP7004", "BHCP7005", "BHCP7101", "BHCP7102", "BHCP7103",
    "BHCP7201", "BHCP7202", "BHCP7203", "BHCP7301", "BHCP7302", "BHCP7401", "BHCP7402", "BHCP7403",
    "BHCP7501", "BHCP7502", "BHCP7503", "BHCP7504", "BHCP7505", "BHCP7506", "BHCP7507",
];

const REGISTRY: &[(&str, &str)] = &[
    ("canonical-program", "program.bhcp"),
    ("alternate-program", "program.words.bhcp"),
    ("program-contract", "program-contract.txt"),
    ("syntax-source", "syntax.bhcp"),
    ("syntax-document", "syntax.diag"),
    ("syntax-symbol", "bhcp.reference/words@0"),
    ("profile-source", "profile.bhcp"),
    ("profile-document", "profile.diag"),
    ("profile-symbol", "bhcp.reference/review-profile@0"),
    ("policy-source", "policy.bhcp"),
    (
        "organization-policy",
        "bhcp.reference/organization-policy@0",
    ),
    ("repository-policy", "bhcp.reference/repository-policy@0"),
    ("waiver-source", "waiver.bhcp"),
    ("waiver-document", "waiver.diag"),
    ("waiver-symbol", "bhcp.reference/offline-emergency-waiver@0"),
    ("waiver-decision-at", "2026-01-01T00:30:00Z"),
    ("extension-source", "extension.bhcp"),
    ("extension-document", "extension.diag"),
    ("extension-symbol", "bhcp.reference/review@0"),
    ("extension-lowering", "bhcp.reference/lowerReview@0"),
    ("extension-type-rule", "extension-type.rule"),
    ("extension-effect-rule", "extension-effect.rule"),
    ("extension-policy-rule", "extension-policy.rule"),
    (
        "extension-normalization-rule",
        "extension-normalization.rule",
    ),
    ("extension-evidence-rule", "extension-evidence.rule"),
    ("policy-evidence-registry", "policy-evidence-registry.txt"),
    ("planner-input", "planner-input.txt"),
    ("execution-input", "execution-input.txt"),
    ("expected-obligations", "expected-obligations.txt"),
    ("outcome-matrix", "outcome-matrix.txt"),
    ("graph-identities", "graph-identities.txt"),
];

const OBLIGATIONS: &[&str] = &[
    "source|bhcp.reference/DeliverChange@0:attempts|limit|formal|open",
    "source|bhcp.reference/DeliverChange@0:non-empty-digest|contract|formal|open",
    "source|bhcp.reference/DeliverChange@0:tree-depth-matches|contract|formal|open",
    "source|bhcp.reference/Approve@0:high-risk|contract|human-approved|open",
    "source|bhcp.reference/Persist@0:safe-result|contract|static|open",
    "source|bhcp.reference/Persist@0:stored|contract|formal|open",
    "source|bhcp.reference/WalkTree@0:depth|limit|formal|open",
    "source|bhcp.reference/WalkTree@0:leaf-at-zero|contract|formal|open",
    "source|bhcp.reference/WalkTree@0:non-negative-depth|contract|formal|open",
    "policy|bhcp.reference/limit.attempts@0|limit|formal|open",
    "policy|bhcp.reference/obligation.static-analysis@0|evidence-demand|static|open",
    "policy|bhcp.reference/obligation.human-approval@0|evidence-demand|human-approved|open",
];

const OUTCOMES: &[&str] = &[
    "satisfied|execution|completed|satisfied",
    "refuted|execution|completed|refuted",
    "unresolved|execution|completed|unresolved:missing-evidence",
    "policy-denied|planning|refused|policy-denied",
    "budget-refused|planning|refused|budget-exhausted",
    "stale|execution|completed|unresolved:stale-evidence",
    "cancelled|execution|completed|unresolved:cancelled",
    "faulted|execution|faulted|operational-fault",
];

const POLICY_EVIDENCE_BINDINGS: &[&str] = &[
    "bhcp.reference/obligation.human-approval@0|bhcp.verifier/human-approval@0|bhcp.reference/Approve@0:high-risk",
    "bhcp.reference/obligation.static-analysis@0|bhcp.verifier/static-analysis@0|bhcp.reference/Persist@0:safe-result",
];

const EXTENSION_RULES: &[(&str, &str, &str)] = &[
    (
        "type_rule",
        "extension-type-rule",
        "extension: bhcp.reference/review@0\ninput: bhcp.reference/Risk@0\noutput: Unit\nchildren: []\n",
    ),
    (
        "effect_rule",
        "extension-effect-rule",
        "extension: bhcp.reference/review@0\neffects: []\n",
    ),
    (
        "policy_rule",
        "extension-policy-rule",
        "extension: bhcp.reference/review@0\npolicy: inherit-enclosing-without-override\n",
    ),
    (
        "normalization_rule",
        "extension-normalization-rule",
        "extension: bhcp.reference/review@0\nnormalization: lower-completely-to-kernel-network\nreducer: bhcp.reference/reviewReducer@0\n",
    ),
    (
        "evidence_rule",
        "extension-evidence-rule",
        "extension: bhcp.reference/review@0\nevidence: checked-kernel-derivation\n",
    ),
];

const PROGRAM_SEMANTICS: &[&str] = &[
    "type|bhcp.reference/StartDelivery@0|output|token|Text",
    "type|bhcp.reference/ConfirmDelivery@0|input|started|{token:Text}",
    "type|bhcp.reference/ConfirmDelivery@0|output|confirmation|Text",
    "type|bhcp.reference/Approve@0|input|risk|bhcp.reference/Risk@0",
    "type|bhcp.reference/Approve@0|output|approval|Text",
    "type|bhcp.reference/Persist@0|input|patch|owned affine bhcp.reference/Patch@0",
    "type|bhcp.reference/Persist@0|resource|repository|owned linear bhcp.reference/Repository@0",
    "type|bhcp.reference/Persist@0|output|receipt|Result<bhcp.reference/Receipt@0,bhcp.reference/DeliveryError@0>",
    "type|bhcp.reference/WalkTree@0|input|node|bhcp.reference/Node@0",
    "type|bhcp.reference/WalkTree@0|input|remaining|Integer",
    "type|bhcp.reference/WalkTree@0|output|result|Unit",
    "type|bhcp.reference/ReviewApproval@0|input|risk|bhcp.reference/Risk@0",
    "type|bhcp.reference/DeliverySequence@0|output|confirmation|Text",
    "type|bhcp.reference/DeliverChange@0|input|patch|owned affine bhcp.reference/Patch@0",
    "type|bhcp.reference/DeliverChange@0|input|risk|bhcp.reference/Risk@0",
    "type|bhcp.reference/DeliverChange@0|input|tree|bhcp.reference/Node@0",
    "type|bhcp.reference/DeliverChange@0|input|tree_depth|Integer",
    "type|bhcp.reference/DeliverChange@0|resource|repository|owned linear bhcp.reference/Repository@0",
    "type|bhcp.reference/DeliverChange@0|state|attempts|Integer",
    "type|bhcp.reference/DeliverChange@0|output|tree_checked|Unit",
    "type|bhcp.reference/DeliverChange@0|output|review_checkpoint|Unit",
    "type|bhcp.reference/DeliverChange@0|output|approval|variant{Excluded,Included({approval:Text})}",
    "type|bhcp.reference/DeliverChange@0|output|sequence|{confirmation:Text}",
    "type|bhcp.reference/DeliverChange@0|output|delivery|{receipt:Result<bhcp.reference/Receipt@0,bhcp.reference/DeliveryError@0>}",
    "type|bhcp.reference/review@0|input|risk|bhcp.reference/Risk@0",
    "type|bhcp.reference/review@0|output|result|Unit",
    "clause|bhcp.reference/Approve@0|high-risk|requires|Bool|bhcp.reference/isHighRisk@0(risk)",
    "clause|bhcp.reference/WalkTree@0|non-negative-depth|requires|Bool|0 <= remaining",
    "clause|bhcp.reference/WalkTree@0|leaf-at-zero|requires|Bool|bhcp.reference/leafAtZero@0(node,remaining)",
    "clause|bhcp.reference/DeliverChange@0|non-empty-digest|requires|Bool|bhcp.reference/patchDigestIsNonEmpty@0(patch)",
    "clause|bhcp.reference/DeliverChange@0|tree-depth-matches|requires|Bool|bhcp.reference/treeDepthMatches@0(tree,tree_depth)",
    "clause|bhcp.reference/Persist@0|safe-result|ensures|Bool|bhcp.reference/isSuccessful@0(receipt)",
    "clause|bhcp.reference/Persist@0|stored|ensures|Bool|bhcp.reference/hasStoredDigest@0(receipt)",
    "limit|bhcp.reference/WalkTree@0|depth|bhcp.reference/limit.depth@0|remaining|64|Bool",
    "limit|bhcp.reference/DeliverChange@0|attempts|bhcp.reference/limit.attempts@0|attempts|3|Bool",
    "effect|bhcp.reference/Persist@0|allow|bhcp-effect/fs.read@0|resource.repository",
    "effect|bhcp.reference/Persist@0|allow|bhcp-effect/fs.write@0|resource.repository",
    "effect|bhcp.reference/Persist@0|forbid|bhcp-effect/network@0|-",
    "effect|bhcp.reference/DeliverChange@0|allow|bhcp-effect/fs.read@0|resource.repository",
    "effect|bhcp.reference/DeliverChange@0|allow|bhcp-effect/fs.write@0|resource.repository",
    "effect|bhcp.reference/DeliverChange@0|allow|bhcp-effect/process@0|literal.cargo",
    "effect|bhcp.reference/DeliverChange@0|forbid|bhcp-effect/network@0|-",
    "recursion|bhcp.reference/WalkTree@0|walked|well-founded|remaining|remaining - 1|0 < remaining|bhcp.reference/leafAtZero@0(node,remaining)",
    "chain|bhcp.reference/DeliverySequence@0|body|started|bhcp.reference/StartDelivery@0|input-free|-",
    "chain|bhcp.reference/DeliverySequence@0|body|confirmed|bhcp.reference/ConfirmDelivery@0|predecessor-whole|step.started",
    "reducer|bhcp.reference/reviewReducer@0|bhcp.reference/Risk@0|{}|Reduction<Unit>|pending-or-concluded",
    "lowerer|bhcp.reference/lowerReview@0|Meta<DerivedForm,bhcp.reference/Risk@0,Unit>|Meta<NetworkShape,bhcp.reference/Risk@0,Unit>|bhcp/meta.network-shape@0",
    "extension-shape|bhcp.reference/review@0|bhcp.reference/Risk@0|Unit|no-children",
];

const PROGRAM_DEFINITIONS: &[(&str, &str)] = &[
    ("type", "bhcp.reference/NonEmptyText@0"),
    ("type", "bhcp.reference/Risk@0"),
    ("type", "bhcp.reference/Patch@0"),
    ("type", "bhcp.reference/Repository@0"),
    ("type", "bhcp.reference/Receipt@0"),
    ("type", "bhcp.reference/DeliveryError@0"),
    ("type", "bhcp.reference/Node@0"),
    ("type", "bhcp.reference/WalkInput@0"),
    ("type", "bhcp.reference/Delivery@0"),
    ("function", "bhcp.reference/isHighRisk@0"),
    ("function", "bhcp.reference/patchDigestIsNonEmpty@0"),
    ("function", "bhcp.reference/treeDepthMatches@0"),
    ("function", "bhcp.reference/leafAtZero@0"),
    ("function", "bhcp.reference/isSuccessful@0"),
    ("function", "bhcp.reference/hasStoredDigest@0"),
    ("predicate", "bhcp.reference/nonEmpty@0"),
    ("goal", "bhcp.reference/StartDelivery@0"),
    ("goal", "bhcp.reference/ConfirmDelivery@0"),
    ("goal", "bhcp.reference/Approve@0"),
    ("goal", "bhcp.reference/Persist@0"),
    ("goal", "bhcp.reference/WalkTree@0"),
    ("goal", "bhcp.reference/ReviewApproval@0"),
    ("goal", "bhcp.reference/DeliverySequence@0"),
    ("goal", "bhcp.reference/DeliverChange@0"),
    ("function", "bhcp.reference/reviewReducer@0"),
    ("function", "bhcp.reference/lowerReview@0"),
    ("extension", "bhcp.reference/review@0"),
];

const PROGRAM_SOURCE_HASHES: &[&str] = &[
    "source-hash|type|bhcp.reference/NonEmptyText@0|bhcp.hash/sha3-512@0:01d8536b07636ad0c97e56159a87c4a4541d017f0974b7e3736838e69dc75092c42b2983eaaff870a1cccd77cb1aa49f3d0bbc1c11d4066171e2ed105f7a6a26",
    "source-hash|type|bhcp.reference/Risk@0|bhcp.hash/sha3-512@0:ab3fa7f2a5bf92f339edbcf5fe1cb2d6f855836c26c399c6962ed0474bf348452cc027233ac4d8e4a542c0d1f9f136315e8238d03e8b4287b3d4963fbd779429",
    "source-hash|type|bhcp.reference/Patch@0|bhcp.hash/sha3-512@0:b14aabcc24ad133e82c47587acfb5a87d80808a2da86f73a373a15fde39189a644c14226ea086bb41eff9e52c4b0d7b862ab9cf197781bcf40db63df44340448",
    "source-hash|type|bhcp.reference/Repository@0|bhcp.hash/sha3-512@0:f0e001c58a73a9a990652428c3c12a0b7fb29de43ede62ea2e0939d4161c4004286266b6eb848293189c2a95d0e8e04c2ffab7d014323cac182c6e4ba646eb9d",
    "source-hash|type|bhcp.reference/Receipt@0|bhcp.hash/sha3-512@0:906714efd9e258cb5ac6090a274ea6597815ad67e7b44cd0ad08a0b8ffcdf5864a2fdd61e8b72743e604881e5624d52f43b68e68d6aaa692c043d9ebfe02ba6c",
    "source-hash|type|bhcp.reference/DeliveryError@0|bhcp.hash/sha3-512@0:8f1c479e5181dda73cf2f3688a85a962abe4872dc28894030b8110e073f853ec13d32dd1fc87d037977e67545bb07ed86bb656951047c06565ee748992c304d1",
    "source-hash|type|bhcp.reference/Node@0|bhcp.hash/sha3-512@0:da7de285a7ad42c2bd9a5c77be2a6b27e4f9563e9eba71311db8a248c0c11c038b83a4ec45b2d28a561d0fcff0da24f14eef49eb4036f4a2d4d33ce9eb7b8a0d",
    "source-hash|type|bhcp.reference/WalkInput@0|bhcp.hash/sha3-512@0:af77aa62cb078671b8476325108e1f6ff9aeb22f33ec5df639f42869af1af55d70a45018066df62d83c2503d09b7cfe6f9c76c79d0d13ff436efb0baa9d6b693",
    "source-hash|type|bhcp.reference/Delivery@0|bhcp.hash/sha3-512@0:8c85168b01c3379cce760bc96da647a4ba48b927e1d271cf34235f755f3b8ac3eb1b283cf3b37fbeea083a1911aea62ceed63d902b46c54a91d9bf262e4dde9a",
    "source-hash|function|bhcp.reference/isHighRisk@0|bhcp.hash/sha3-512@0:d2eec7944f101557e4e9645798f5d1e30e4a3b07bd629cef6504d9c3ce0e4bf3bc94ba77b7419e2d4c7b7be077f45c50c547c864bba4a0ff100bb71da60fb0f7",
    "source-hash|function|bhcp.reference/patchDigestIsNonEmpty@0|bhcp.hash/sha3-512@0:3d3beb3903a2cf0ca4e5dbf7b6cedbe230e5bd1c8d9238a6a0ae0325f83441eff34451fac3da44b85248c5bf360aaa0a95831706294b29751a00f1d79b8f3ed8",
    "source-hash|function|bhcp.reference/treeDepthMatches@0|bhcp.hash/sha3-512@0:e6f8a9a7312aa2efb47817d1ea98006cfe451a781d795a67505c8392324fa4047d6de714adcf580e32943d04ff77768338a700620d7cecb2810fb2f884d8317e",
    "source-hash|function|bhcp.reference/leafAtZero@0|bhcp.hash/sha3-512@0:bbce64af0ff307716266ec2c57ad9de2b20212b7186f69382ec7ed9132f0073a6f860d5cd3fafe884641c7527eb35cc26aa15eabee63b8ff0820191a95c9b921",
    "source-hash|function|bhcp.reference/isSuccessful@0|bhcp.hash/sha3-512@0:a8aa239cba1ec77eafb51bc821f6331c1c16c0186ed29a4ae1306ffedb3225fab20970dbeafdb55a78570688804a11d74cf642978b720f20e6b668e6e34f2300",
    "source-hash|function|bhcp.reference/hasStoredDigest@0|bhcp.hash/sha3-512@0:e3963668cf16f657c3391fd1f0214256678a3301c5a11c48ee9e151f29bb19accd0538f7105eb9f1f9863901484467d8a62a5a5be18755b5f29201039e1974ba",
    "source-hash|predicate|bhcp.reference/nonEmpty@0|bhcp.hash/sha3-512@0:a76335b2b3d4bf66b4133f4ac135a899448823ac285fcdcc2dce3eb7a66162d0224429b702767cec40c1671f48331bdd3e36b5287a32c42ec9dbe31353d72902",
    "source-hash|goal|bhcp.reference/StartDelivery@0|bhcp.hash/sha3-512@0:f5c2041eba4034ad467bcd00e472002d4147112de9198bd125164a3353630f393d31e4708756060d8f4b29b6d58925fc74ffd8eea6f740f9fe85331b4cbf6739",
    "source-hash|goal|bhcp.reference/ConfirmDelivery@0|bhcp.hash/sha3-512@0:bf5a7ff6f4efcf2e449a6a7721be4fbe80d4899f1e04caf54536e820fa111512af5ae79dcf617ef5b4aedd6e67af0b99858fd841051397bc12dbc96529c12239",
    "source-hash|goal|bhcp.reference/Approve@0|bhcp.hash/sha3-512@0:c2afc08120e2c85932b4e2169f04f82b4f16aaecab46a065712a82dd63bb04de49da990deb9264b5c64ead20616628b63da367d2e6f63a7ab75f45d4ab6c71d7",
    "source-hash|goal|bhcp.reference/Persist@0|bhcp.hash/sha3-512@0:5773e726376ff260d1f131373ff65da0f1c300230b6d5062cd08a4931a95063589a37a19121a4000e7e71e6cce1ec65d5cefea6315691d07cef8aa9506751d21",
    "source-hash|goal|bhcp.reference/WalkTree@0|bhcp.hash/sha3-512@0:7abb7c286c8188a07112e7fe757e7dda91a553fecf5a12b02d71963b2dc4fe45523ecf73212dc9e3a1e43f2c5202cf21924f5bca31f2d6988907b1682bf9e84c",
    "source-hash|goal|bhcp.reference/ReviewApproval@0|bhcp.hash/sha3-512@0:e8188a64f264a0c223bf2cd31a01a290c2a3c2f4b9b7d08da5109b5a242acde749bef4d1c05321047e513f82bd9553fbb8de4de6120a7feeae449a674d644ebe",
    "source-hash|goal|bhcp.reference/DeliverySequence@0|bhcp.hash/sha3-512@0:d6b8ee30e0d350850a6da0011b850c2ebf1fe834399ea9983a7ed974fc76491a64335ae06697cb29b5f24fbf0a8a2b3d76031d15d56a2dee8b5b9ed752692d89",
    "source-hash|goal|bhcp.reference/DeliverChange@0|bhcp.hash/sha3-512@0:5eeff8a17a050b42eda0421bd0db97a57934ed59af4bc3f40d4a42be1b5757874520097a91fa5825999ebe6daa9b3027563bcdccc41ee6793ce9ce21045b5e38",
    "source-hash|function|bhcp.reference/reviewReducer@0|bhcp.hash/sha3-512@0:a371bd995c73a30e6ee2ee3ed7e89e4682d1e41843a1a12cf9b6df3bda4cc91eceb6038a7f149ad982c8944c81e8461c08219db1b00c5d7b6f0f610a8609bb38",
    "source-hash|function|bhcp.reference/lowerReview@0|bhcp.hash/sha3-512@0:03d6b8313992e4b03eaf910d333d25bd04496095d96bafd2c49d3a5530a8bbe7ef6f986e23d365012ea5df07d2f9396d0af56b73722d1d1b89e2330da06562bf",
    "source-hash|extension|bhcp.reference/review@0|bhcp.hash/sha3-512@0:a4efad8f3a560339836d309788349afc204d30a42dc675d00fe7ff43968114c99c485bb13dd48bfeff11713de6a3e549df8c101fd49db45d336137b47729ee0b",
];

const PROGRAM_FILE_HASHES: &[(&str, &str)] = &[
    (
        "program.bhcp",
        "bhcp.hash/sha3-512@0:28c25c87b4709b83b2eeb20a1e45c599cfb50d90ae76507f5884ab217a92389bec2fbb7d407079371e7ec83096df3a971dbf6728f9713b09950affd18afb840f",
    ),
    (
        "extension.bhcp",
        "bhcp.hash/sha3-512@0:07b3f74e8ef4d3d70a570a243f78dfac67f6f148f8be90a6920d3c006bb6b08c18f2499cfd7ea78eb0982e8e39c246b986fb555870c69be912cb81f561c06991",
    ),
];

#[derive(Debug)]
struct Stage {
    owners: Vec<String>,
    outcome: String,
}

#[derive(Debug)]
struct Feature {
    artifact: String,
    owners: Vec<String>,
    needles: Vec<String>,
}

#[derive(Debug, Eq, PartialEq)]
struct FrontendScenario {
    section: String,
    test: String,
    artifact: PathBuf,
}

#[derive(Debug, Eq, PartialEq)]
struct GraphScenario {
    test: String,
    roots: BTreeSet<String>,
    diagnostics: BTreeSet<String>,
    identity: PathBuf,
}

#[derive(Debug, Default)]
struct CompletionContract {
    version: Option<String>,
    issues: BTreeMap<u64, String>,
    scenarios: BTreeMap<String, String>,
    frontend: BTreeMap<String, FrontendScenario>,
    graphs: BTreeMap<String, GraphScenario>,
    roots: BTreeMap<String, String>,
    stages: BTreeMap<String, Stage>,
    artifacts: BTreeMap<String, PathBuf>,
    features: BTreeMap<String, Feature>,
    non_goals: BTreeSet<String>,
}

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn insert_unique<K: Ord, V>(
    map: &mut BTreeMap<K, V>,
    key: K,
    value: V,
    kind: &str,
) -> Result<(), String> {
    if map.insert(key, value).is_some() {
        return Err(format!("duplicate {kind}"));
    }
    Ok(())
}

fn comma_list(field: &str) -> Vec<String> {
    field.split(',').map(str::to_owned).collect()
}

fn parse_contract(text: &str) -> Result<CompletionContract, String> {
    let mut contract = CompletionContract::default();
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split('|').collect::<Vec<_>>();
        match fields.as_slice() {
            ["version", version] => {
                if contract.version.replace((*version).to_owned()).is_some() {
                    return Err("duplicate version".to_owned());
                }
            }
            ["issue", number, key] => {
                let number = number
                    .parse::<u64>()
                    .map_err(|_| format!("line {line_number}: invalid issue number"))?;
                insert_unique(&mut contract.issues, number, (*key).to_owned(), "issue")?;
            }
            ["scenario", id, owner] => insert_unique(
                &mut contract.scenarios,
                (*id).to_owned(),
                (*owner).to_owned(),
                "scenario",
            )?,
            ["frontend", id, section, test, artifact] => insert_unique(
                &mut contract.frontend,
                (*id).to_owned(),
                FrontendScenario {
                    section: (*section).to_owned(),
                    test: (*test).to_owned(),
                    artifact: PathBuf::from(artifact),
                },
                "front-end scenario",
            )?,
            ["graph", id, test, roots, diagnostics, identity] => insert_unique(
                &mut contract.graphs,
                (*id).to_owned(),
                GraphScenario {
                    test: (*test).to_owned(),
                    roots: comma_list(roots).into_iter().collect(),
                    diagnostics: comma_list(diagnostics).into_iter().collect(),
                    identity: PathBuf::from(identity),
                },
                "graph scenario",
            )?,
            ["root", kind, owner] => insert_unique(
                &mut contract.roots,
                (*kind).to_owned(),
                (*owner).to_owned(),
                "root",
            )?,
            ["stage", id, owners, outcome] => insert_unique(
                &mut contract.stages,
                (*id).to_owned(),
                Stage {
                    owners: comma_list(owners),
                    outcome: (*outcome).to_owned(),
                },
                "stage",
            )?,
            ["artifact", id, path] => insert_unique(
                &mut contract.artifacts,
                (*id).to_owned(),
                PathBuf::from(path),
                "artifact",
            )?,
            ["feature", id, artifact, owners, needles] => insert_unique(
                &mut contract.features,
                (*id).to_owned(),
                Feature {
                    artifact: (*artifact).to_owned(),
                    owners: comma_list(owners),
                    needles: comma_list(needles),
                },
                "feature",
            )?,
            ["non-goal", id] => {
                if !contract.non_goals.insert((*id).to_owned()) {
                    return Err("duplicate non-goal".to_owned());
                }
            }
            _ => return Err(format!("line {line_number}: unknown or malformed record")),
        }
    }
    Ok(contract)
}

fn markdown_scenarios(readme: &str) -> BTreeSet<String> {
    let mut scenarios = BTreeSet::new();
    for line in readme.lines().filter(|line| line.starts_with("| ")) {
        let Some(id) = line.split('|').nth(1).map(str::trim) else {
            continue;
        };
        if id == "ID" || !id.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
            continue;
        }
        if let Some(behavior) = id.strip_prefix("ALG-") {
            for verdict in ["S", "R", "U", "F"] {
                scenarios.insert(format!("ALG-{behavior}-{verdict}"));
            }
        } else {
            scenarios.insert(id.to_owned());
        }
    }
    scenarios
}

fn schema_roots(example_manifest: &str) -> BTreeSet<String> {
    example_manifest
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .map(str::to_owned)
        .collect()
}

fn expected_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn reference_directory(root: &Path) -> PathBuf {
    root.join("conformance/v0/reference-program")
}

fn read_reference(root: &Path, name: &str) -> Result<String, String> {
    fs::read_to_string(reference_directory(root).join(name))
        .map_err(|error| format!("cannot read reference artifact {name}: {error}"))
}

fn parse_registry(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut registry = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let fields = line.split('|').collect::<Vec<_>>();
        let [key, value] = fields.as_slice() else {
            return Err(format!("registry line {} is malformed", index + 1));
        };
        insert_unique(
            &mut registry,
            (*key).to_owned(),
            (*value).to_owned(),
            "registry key",
        )?;
    }
    Ok(registry)
}

fn value_text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    match value.get(key) {
        Some(Value::Text(text)) => Ok(text),
        _ => Err(format!("projection field {key} is not text")),
    }
}

#[derive(Clone, Debug)]
struct DataCall {
    parent: String,
    step: String,
    callee: String,
    argument: String,
    mode: String,
    source: String,
}

#[derive(Debug, Default)]
struct ProgramContract {
    definitions: BTreeSet<(String, String)>,
    source_hashes: BTreeMap<(String, String), String>,
    file_hashes: BTreeMap<String, String>,
    facts: BTreeMap<(String, String, String), String>,
    consumes: BTreeSet<(String, String, String)>,
    calls: Vec<DataCall>,
    calls0: Vec<(String, String, String)>,
    semantics: BTreeSet<String>,
}

fn parse_program_contract(text: &str) -> Result<ProgramContract, String> {
    let mut contract = ProgramContract::default();
    for (index, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split('|').collect::<Vec<_>>();
        match fields.as_slice() {
            ["definition", kind, symbol] => {
                if !contract
                    .definitions
                    .insert(((*kind).to_owned(), (*symbol).to_owned()))
                {
                    return Err("duplicate program definition".to_owned());
                }
            }
            ["source-hash", kind, symbol, digest] => insert_unique(
                &mut contract.source_hashes,
                ((*kind).to_owned(), (*symbol).to_owned()),
                (*digest).to_owned(),
                "program source hash",
            )?,
            ["file-hash", name, digest] => insert_unique(
                &mut contract.file_hashes,
                (*name).to_owned(),
                (*digest).to_owned(),
                "program file hash",
            )?,
            ["fact", owner, kind, name, mode] => insert_unique(
                &mut contract.facts,
                ((*owner).to_owned(), (*kind).to_owned(), (*name).to_owned()),
                (*mode).to_owned(),
                "program fact",
            )?,
            ["consume", owner, source, destination] => {
                if !contract.consumes.insert((
                    (*owner).to_owned(),
                    (*source).to_owned(),
                    (*destination).to_owned(),
                )) {
                    return Err("duplicate program consumption".to_owned());
                }
            }
            ["call", parent, step, callee, argument, mode, source] => {
                contract.calls.push(DataCall {
                    parent: (*parent).to_owned(),
                    step: (*step).to_owned(),
                    callee: (*callee).to_owned(),
                    argument: (*argument).to_owned(),
                    mode: (*mode).to_owned(),
                    source: (*source).to_owned(),
                });
            }
            ["call0", parent, step, callee] => contract.calls0.push((
                (*parent).to_owned(),
                (*step).to_owned(),
                (*callee).to_owned(),
            )),
            [kind, ..]
                if matches!(
                    *kind,
                    "type"
                        | "clause"
                        | "limit"
                        | "effect"
                        | "recursion"
                        | "chain"
                        | "reducer"
                        | "lowerer"
                        | "extension-shape"
                ) =>
            {
                if !contract.semantics.insert(line.to_owned()) {
                    return Err("duplicate semantic projection row".to_owned());
                }
            }
            _ => {
                return Err(format!("program contract line {} is malformed", index + 1));
            }
        }
    }
    Ok(contract)
}

fn fact_mode<'a>(
    contract: &'a ProgramContract,
    owner: &str,
    kind: &str,
    name: &str,
) -> Result<&'a str, String> {
    contract
        .facts
        .get(&(owner.to_owned(), kind.to_owned(), name.to_owned()))
        .map(String::as_str)
        .ok_or_else(|| format!("unknown {kind} {owner}:{name}"))
}

fn definition_block<'a>(source: &'a str, kind: &str, symbol: &str) -> Result<&'a str, String> {
    let marker = format!("§{kind} {symbol}");
    let start = source
        .find(&marker)
        .ok_or_else(|| format!("source omits {marker}"))?;
    let tail = &source[start..];
    let end = tail[marker.len()..]
        .find("\n§")
        .map(|offset| marker.len() + offset)
        .unwrap_or(tail.len());
    Ok(&tail[..end])
}

fn compact(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn top_level_definition_inventory(source: &str) -> Result<BTreeSet<(String, String)>, String> {
    let mut definitions = BTreeSet::new();
    for line in source.lines().filter(|line| line.starts_with('§')) {
        let Some((kind, remainder)) = line['§'.len_utf8()..].split_once(' ') else {
            return Err(format!("unprojected top-level construct {line:?}"));
        };
        if !matches!(
            kind,
            "type" | "function" | "predicate" | "goal" | "extension"
        ) {
            return Err(format!("unprojected top-level construct §{kind}"));
        }
        let symbol = remainder
            .split(|character: char| character.is_whitespace() || character == '(')
            .next()
            .filter(|symbol| !symbol.is_empty())
            .ok_or_else(|| format!("top-level {kind} definition omits a symbol"))?;
        if !definitions.insert((kind.to_owned(), symbol.to_owned())) {
            return Err(format!("duplicate top-level definition {kind} {symbol}"));
        }
    }
    Ok(definitions)
}

fn source_goal_types(
    canonical: &str,
    definitions: &BTreeSet<(String, String)>,
) -> Result<BTreeMap<(String, String, String), String>, String> {
    let mut types = BTreeMap::new();
    for (_, owner) in definitions.iter().filter(|(kind, _)| kind == "goal") {
        let block = definition_block(canonical, "goal", owner)?;
        for line in block.lines().map(str::trim) {
            let Some(kind) = ["input", "resource", "state", "output"]
                .into_iter()
                .find(|kind| line.starts_with(&format!("§{kind} ")))
            else {
                continue;
            };
            let declaration = line
                .strip_prefix(&format!("§{kind} "))
                .expect("matched declaration prefix");
            let Some((name, value_type)) = declaration.split_once(':') else {
                return Err(format!("malformed source fact in {owner}"));
            };
            let Some(value_type) = value_type.trim().strip_suffix(';') else {
                return Err(format!("unterminated source fact in {owner}"));
            };
            insert_unique(
                &mut types,
                (owner.clone(), kind.to_owned(), name.trim().to_owned()),
                compact(value_type),
                "source typed fact",
            )?;
        }
    }
    Ok(types)
}

fn source_clause_inventory(
    canonical: &str,
    definitions: &BTreeSet<(String, String)>,
) -> Result<BTreeSet<(String, String, String)>, String> {
    let mut clauses = BTreeSet::new();
    for (_, owner) in definitions.iter().filter(|(kind, _)| kind == "goal") {
        let block = definition_block(canonical, "goal", owner)?;
        for line in block.lines().map(str::trim) {
            let Some(contract) = ["requires", "ensures"]
                .into_iter()
                .find(|contract| line.starts_with(&format!("§{contract} \"")))
            else {
                continue;
            };
            let Some(label) = line
                .strip_prefix(&format!("§{contract} \""))
                .and_then(|remainder| remainder.split_once("\":"))
                .map(|(label, _)| label)
            else {
                return Err(format!("malformed source clause in {owner}"));
            };
            if !clauses.insert((owner.clone(), contract.to_owned(), label.to_owned())) {
                return Err(format!("duplicate source clause {owner}:{label}"));
            }
        }
    }
    Ok(clauses)
}

fn validate_program_projection_sources(
    text: &str,
    canonical: &str,
    extension_source: &str,
) -> Result<(), String> {
    let projection = parse_program_contract(text)?;
    if projection.semantics != expected_set(PROGRAM_SEMANTICS) {
        return Err("program semantic projection mismatch".to_owned());
    }
    let expected_definitions = PROGRAM_DEFINITIONS
        .iter()
        .map(|(kind, symbol)| ((*kind).to_owned(), (*symbol).to_owned()))
        .collect::<BTreeSet<_>>();
    if projection.definitions != expected_definitions {
        return Err("program definition projection mismatch".to_owned());
    }
    let definition_symbols = projection
        .definitions
        .iter()
        .map(|(_, symbol)| symbol.as_str())
        .collect::<BTreeSet<_>>();

    for (kind, symbol) in &projection.definitions {
        if !matches!(
            kind.as_str(),
            "type" | "function" | "predicate" | "goal" | "extension"
        ) {
            return Err(format!("unknown definition kind {kind}"));
        }
        let marker = format!("§{kind} {symbol}");
        let occurrences =
            canonical.matches(&marker).count() + extension_source.matches(&marker).count();
        if occurrences != 1 {
            return Err(format!(
                "definition marker {marker:?} occurs {occurrences} times"
            ));
        }
    }

    let mut source_definitions = top_level_definition_inventory(canonical)?;
    source_definitions.extend(top_level_definition_inventory(extension_source)?);
    if source_definitions != projection.definitions {
        return Err("source definition projection mismatch".to_owned());
    }

    let projected_goal_types = projection
        .semantics
        .iter()
        .filter(|row| row.starts_with("type|"))
        .filter_map(|row| {
            let fields = row.splitn(6, '|').collect::<Vec<_>>();
            let [_, owner, kind, name, value_type] = fields.as_slice() else {
                return None;
            };
            (*owner != "bhcp.reference/review@0"
                && !(*owner == "bhcp.reference/WalkTree@0"
                    && *kind == "output"
                    && *name == "result"))
                .then(|| {
                    (
                        ((*owner).to_owned(), (*kind).to_owned(), (*name).to_owned()),
                        compact(value_type),
                    )
                })
        })
        .collect::<BTreeMap<_, _>>();
    let actual_goal_types = source_goal_types(canonical, &projection.definitions)?;
    if actual_goal_types != projected_goal_types {
        return Err("source typed fact projection mismatch".to_owned());
    }
    let projected_goal_modes = projection
        .facts
        .iter()
        .filter(|((owner, kind, name), _)| {
            owner != "bhcp.reference/review@0"
                && !(owner == "bhcp.reference/WalkTree@0" && kind == "output" && name == "result")
                && !(owner == "bhcp.reference/ReviewApproval@0"
                    && kind == "output"
                    && name == "result")
        })
        .map(|(key, mode)| (key.clone(), mode.clone()))
        .collect::<BTreeMap<_, _>>();
    let actual_goal_modes = actual_goal_types
        .iter()
        .map(|(key, value_type)| {
            let mode = if value_type.starts_with("ownedaffine") {
                "owned-affine"
            } else if value_type.starts_with("ownedlinear") {
                "owned-linear"
            } else {
                "unrestricted"
            };
            (key.clone(), mode.to_owned())
        })
        .collect::<BTreeMap<_, _>>();
    if actual_goal_modes != projected_goal_modes {
        return Err("source fact ownership projection mismatch".to_owned());
    }

    let projected_clauses = projection
        .semantics
        .iter()
        .filter(|row| row.starts_with("clause|"))
        .filter_map(|row| {
            let fields = row.splitn(6, '|').collect::<Vec<_>>();
            let [_, owner, label, contract, _, _] = fields.as_slice() else {
                return None;
            };
            Some((
                (*owner).to_owned(),
                (*contract).to_owned(),
                (*label).to_owned(),
            ))
        })
        .collect::<BTreeSet<_>>();
    if source_clause_inventory(canonical, &projection.definitions)? != projected_clauses {
        return Err("source clause projection mismatch".to_owned());
    }

    for row in projection
        .semantics
        .iter()
        .filter(|row| row.starts_with("type|"))
    {
        let fields = row.splitn(6, '|').collect::<Vec<_>>();
        let [_, owner, kind, name, value_type] = fields.as_slice() else {
            return Err("typed projection row is malformed".to_owned());
        };
        if *owner == "bhcp.reference/review@0"
            || (*owner == "bhcp.reference/WalkTree@0" && *kind == "output")
        {
            continue;
        }
        let block = definition_block(canonical, "goal", owner)?;
        let marker = format!("§{kind} {name}: {value_type};");
        if !compact(block).contains(&compact(&marker)) {
            return Err(format!("{owner} omits owner-scoped typed fact {marker}"));
        }
    }

    for row in projection
        .semantics
        .iter()
        .filter(|row| row.starts_with("clause|"))
    {
        let fields = row.splitn(6, '|').collect::<Vec<_>>();
        let [_, owner, label, contract, _, expression] = fields.as_slice() else {
            return Err("clause projection row is malformed".to_owned());
        };
        let block = definition_block(canonical, "goal", owner)?;
        let marker = format!("§{contract} \"{label}\": {expression}");
        if !compact(block).contains(&compact(&marker)) {
            return Err(format!("{owner} omits owner-scoped clause {marker}"));
        }
    }

    let expected_source_hashes = PROGRAM_SOURCE_HASHES
        .iter()
        .map(|row| {
            let fields = row.splitn(4, '|').collect::<Vec<_>>();
            let ["source-hash", kind, symbol, digest] = fields.as_slice() else {
                panic!("malformed expected source hash row");
            };
            (
                ((*kind).to_owned(), (*symbol).to_owned()),
                (*digest).to_owned(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if projection.source_hashes != expected_source_hashes {
        return Err("program source hash projection mismatch".to_owned());
    }
    for ((kind, symbol), expected_hash) in &projection.source_hashes {
        let block = if canonical.contains(&format!("§{kind} {symbol}")) {
            definition_block(canonical, kind, symbol)?
        } else {
            definition_block(extension_source, kind, symbol)?
        };
        let actual_hash = format_hash(&HashAlgorithm::default().hash(block.as_bytes()));
        if &actual_hash != expected_hash {
            return Err(format!(
                "type definition projection mismatch for {kind} {symbol}: source hash differs"
            ));
        }
    }
    let expected_file_hashes = PROGRAM_FILE_HASHES
        .iter()
        .map(|(name, digest)| ((*name).to_owned(), (*digest).to_owned()))
        .collect::<BTreeMap<_, _>>();
    if projection.file_hashes != expected_file_hashes {
        return Err("program whole source hash projection mismatch".to_owned());
    }
    for (name, source) in [
        ("program.bhcp", canonical),
        ("extension.bhcp", extension_source),
    ] {
        let actual_hash = format_hash(&HashAlgorithm::default().hash(source.as_bytes()));
        if projection.file_hashes.get(name) != Some(&actual_hash) {
            return Err(format!("whole source hash mismatch for {name}"));
        }
    }

    let deliver = definition_block(canonical, "goal", "bhcp.reference/DeliverChange@0")?;
    let persist = definition_block(canonical, "goal", "bhcp.reference/Persist@0")?;
    let walk = definition_block(canonical, "goal", "bhcp.reference/WalkTree@0")?;
    let sequence = definition_block(canonical, "goal", "bhcp.reference/DeliverySequence@0")?;
    for (block, marker) in [
        (
            walk,
            "§limit \"depth\": bhcp.reference/limit.depth@0: remaining <= 64;",
        ),
        (walk, "remaining = remaining - 1"),
        (
            persist,
            "§allows bhcp-effect/fs.read@0(repository), bhcp-effect/fs.write@0(repository);",
        ),
        (persist, "§forbids bhcp-effect/network@0;"),
        (
            deliver,
            "§limit \"attempts\": bhcp.reference/limit.attempts@0: attempts <= 3;",
        ),
        (
            deliver,
            "§allows bhcp-effect/fs.read@0(repository), bhcp-effect/fs.write@0(repository), bhcp-effect/process@0(\"cargo\");",
        ),
        (deliver, "§forbids bhcp-effect/network@0;"),
        (sequence, "started = bhcp.reference/StartDelivery@0();"),
        (
            sequence,
            "confirmed = bhcp.reference/ConfirmDelivery@0(started = started);",
        ),
    ] {
        if !compact(block).contains(&compact(marker)) {
            return Err(format!("owner-scoped source omits {marker}"));
        }
    }

    let mut steps = BTreeMap::<(String, String), String>::new();
    let mut uses = BTreeMap::<(String, String), usize>::new();
    for (parent, step, callee) in &projection.calls0 {
        if !definition_symbols.contains(parent.as_str())
            || !definition_symbols.contains(callee.as_str())
        {
            return Err("input-free call has an unknown definition".to_owned());
        }
        if projection.facts.keys().any(|(owner, kind, _)| {
            owner == callee && matches!(kind.as_str(), "input" | "resource")
        }) {
            return Err(format!("first chain child {callee} is not input-free"));
        }
        if steps
            .insert((parent.clone(), step.clone()), callee.clone())
            .is_some()
        {
            return Err(format!("duplicate input-free step {step}"));
        }
        let block = definition_block(canonical, "goal", parent)?;
        if !compact(block).contains(&compact(&format!("{callee}();"))) {
            return Err(format!("source omits input-free call {callee}"));
        }
    }
    for call in &projection.calls {
        if !definition_symbols.contains(call.parent.as_str()) {
            return Err(format!("call has unknown parent {}", call.parent));
        }
        if !definition_symbols.contains(call.callee.as_str()) {
            return Err(format!("call has unknown callee {}", call.callee));
        }
        let target_kind = if projection.facts.contains_key(&(
            call.callee.clone(),
            "input".to_owned(),
            call.argument.clone(),
        )) {
            "input"
        } else {
            "resource"
        };
        fact_mode(&projection, &call.callee, target_kind, &call.argument)?;
        if !matches!(call.mode.as_str(), "copy" | "move") {
            return Err(format!("call {} has unknown transfer mode", call.step));
        }

        let source_mode = if let Some(name) = call.source.strip_prefix("input.") {
            fact_mode(&projection, &call.parent, "input", name)?
        } else if let Some(name) = call.source.strip_prefix("resource.") {
            fact_mode(&projection, &call.parent, "resource", name)?
        } else if let Some(step) = call.source.strip_prefix("step.") {
            let callee = steps
                .get(&(call.parent.clone(), step.to_owned()))
                .ok_or_else(|| format!("call {} reads unknown or later step {step}", call.step))?;
            projection
                .facts
                .iter()
                .find_map(|((owner, kind, _), mode)| {
                    (owner == callee && kind == "output").then_some(mode.as_str())
                })
                .ok_or_else(|| format!("step {step} has no declared output"))?
        } else if call.source == "quantifier.child" || call.source.starts_with("expression.") {
            "unrestricted"
        } else {
            return Err(format!(
                "call {} has unknown source {}",
                call.step, call.source
            ));
        };
        if source_mode.starts_with("owned-") != (call.mode == "move") {
            return Err(format!(
                "call {} must move owned values and copy unrestricted values",
                call.step
            ));
        }
        *uses
            .entry((call.parent.clone(), call.source.clone()))
            .or_default() += 1;

        let step_key = (call.parent.clone(), call.step.clone());
        if let Some(existing) = steps.get(&step_key) {
            if existing != &call.callee {
                return Err(format!("step {} names multiple callees", call.step));
            }
        } else {
            steps.insert(step_key, call.callee.clone());
        }

        let call_marker = format!("{}(", call.callee);
        if !canonical.contains(&call_marker) {
            return Err(format!("source omits projected call {call_marker}"));
        }
        let source_name = call
            .source
            .strip_prefix("expression.")
            .unwrap_or_else(|| call.source.rsplit('.').next().unwrap_or(&call.source));
        let argument_marker = if call.mode == "move" {
            format!("{} = move {source_name}", call.argument)
        } else {
            format!("{} = {source_name}", call.argument)
        };
        if !canonical.contains(&argument_marker) {
            return Err(format!("source omits projected argument {argument_marker}"));
        }
    }

    for marker in [
        "§type bhcp.reference/Risk@0 = variant { Low, High };",
        "§type bhcp.reference/Patch@0 = { bytes: Bytes, digest: Text };",
        "§type bhcp.reference/Repository@0 = { root: Text };",
        "§type bhcp.reference/WalkInput@0 = { node: bhcp.reference/Node@0, remaining: Integer };",
        "§input patch: owned affine bhcp.reference/Patch@0;",
        "§output token: Text;",
        "§input started: { token: Text };",
        "§resource repository: owned linear bhcp.reference/Repository@0;",
        "§input remaining: Integer;",
        "§input tree_depth: Integer;",
        "§state attempts: Integer;",
        "§output delivery: { receipt: Result<bhcp.reference/Receipt@0, bhcp.reference/DeliveryError@0> };",
        "§requires \"non-negative-depth\": 0 <= remaining;",
        "§requires \"leaf-at-zero\": bhcp.reference/leafAtZero@0(node, remaining);",
        "§requires \"tree-depth-matches\": bhcp.reference/treeDepthMatches@0(tree, tree_depth);",
        "§limit \"depth\": bhcp.reference/limit.depth@0: remaining <= 64;",
        "§limit \"attempts\": bhcp.reference/limit.attempts@0: attempts <= 3;",
        "remaining = remaining - 1",
        "§allows bhcp-effect/fs.read@0(repository), bhcp-effect/fs.write@0(repository);",
        "§allows bhcp-effect/fs.read@0(repository), bhcp-effect/fs.write@0(repository), bhcp-effect/process@0(\"cargo\");",
    ] {
        if !canonical.contains(marker) {
            return Err(format!("source omits semantic projection marker {marker}"));
        }
    }
    for marker in [
        "parent: bhcp.reference/Risk@0",
        "observations: {}",
        "): Reduction<Unit> =",
        "bhcp/kernel.pending@0",
        "bhcp/kernel.conclude@0",
        "bhcp/meta.network-shape@0(",
        "Meta<DerivedForm, bhcp.reference/Risk@0, Unit>",
        "Meta<NetworkShape, bhcp.reference/Risk@0, Unit>",
    ] {
        if !extension_source.contains(marker) {
            return Err(format!(
                "extension omits semantic projection marker {marker}"
            ));
        }
    }

    for (owner, kind, name) in projection
        .facts
        .keys()
        .filter(|(_, kind, _)| matches!(kind.as_str(), "input" | "resource"))
    {
        let mode = fact_mode(&projection, owner, kind, name)?;
        if !mode.starts_with("owned-") {
            continue;
        }
        let source = format!("{kind}.{name}");
        let call_uses = uses
            .get(&(owner.clone(), source.clone()))
            .copied()
            .unwrap_or_default();
        let local_uses = projection
            .consumes
            .iter()
            .filter(|(candidate_owner, candidate_source, _)| {
                candidate_owner == owner && candidate_source == &source
            })
            .count();
        if call_uses + local_uses != 1 {
            return Err(format!(
                "owned value {owner}:{source} must be consumed exactly once"
            ));
        }
    }

    for ((parent, step), callee) in &steps {
        let Some(output_mode) = projection
            .facts
            .iter()
            .find_map(|((owner, kind, _), mode)| {
                (owner == callee && kind == "output").then_some(mode)
            })
        else {
            return Err(format!("step {step} has no output fact"));
        };
        if output_mode.starts_with("owned-") {
            let source = format!("step.{step}");
            if uses.get(&(parent.clone(), source)).copied() != Some(1) {
                return Err(format!(
                    "owned step output {parent}:{step} is not moved exactly once"
                ));
            }
        }
    }
    Ok(())
}

fn validate_program_projection(root: &Path, text: &str) -> Result<(), String> {
    validate_program_projection_sources(
        text,
        &read_reference(root, "program.bhcp")?,
        &read_reference(root, "extension.bhcp")?,
    )
}

fn validate_program_contract(root: &Path) -> Result<(), String> {
    validate_program_projection(root, &read_reference(root, "program-contract.txt")?)
}

fn validate_reference_policy(text: &str) -> Result<(), String> {
    let parsed_policy =
        parse_policy_source(text, "policy.bhcp").map_err(|error| error.to_string())?;
    let policy_symbols = parsed_policy
        .documents
        .iter()
        .map(|document| document.symbol.as_str())
        .collect::<BTreeSet<_>>();
    if policy_symbols
        != BTreeSet::from([
            "bhcp.reference/organization-policy@0",
            "bhcp.reference/repository-policy@0",
        ])
    {
        return Err("reference policy symbol inventory mismatch".to_owned());
    }
    let effective_policy = compose_policies(&parsed_policy.documents, HashAlgorithm::default())
        .map_err(|error| error.to_string())?;
    let capability_effects = effective_policy
        .effective
        .capabilities
        .iter()
        .map(|rule| rule.value.effect.as_str())
        .collect::<BTreeSet<_>>();
    let capability_goals = effective_policy
        .effective
        .capabilities
        .iter()
        .map(|rule| {
            (
                rule.value.effect.as_str(),
                rule.value
                    .scope
                    .as_ref()
                    .and_then(|scope| scope.goals.as_ref())
                    .cloned(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if capability_effects
        != BTreeSet::from([
            "bhcp-effect/fs.read@0",
            "bhcp-effect/fs.write@0",
            "bhcp-effect/process@0",
        ])
        || capability_goals.get("bhcp-effect/fs.read@0")
            != Some(&Some(vec![
                "bhcp.reference/DeliverChange@0".to_owned(),
                "bhcp.reference/Persist@0".to_owned(),
            ]))
        || capability_goals.get("bhcp-effect/fs.write@0")
            != Some(&Some(vec![
                "bhcp.reference/DeliverChange@0".to_owned(),
                "bhcp.reference/Persist@0".to_owned(),
            ]))
        || capability_goals.get("bhcp-effect/process@0")
            != Some(&Some(vec!["bhcp.reference/DeliverChange@0".to_owned()]))
        || !effective_policy
            .effective
            .prohibitions
            .iter()
            .any(|rule| rule.value.effect == "bhcp-effect/network@0")
        || !effective_policy.effective.limits.iter().any(|rule| {
            rule.value.dimension == "bhcp.reference/limit.attempts@0"
                && rule.value.maximum == ExactNumber::Integer(2)
        })
    {
        return Err(
            "reference policy does not authorize the declared effects and base limit".to_owned(),
        );
    }
    Ok(())
}

fn validate_policy_evidence_bindings(canonical: &str, text: &str) -> Result<(), String> {
    let bindings = text.lines().map(str::to_owned).collect::<BTreeSet<_>>();
    if bindings != expected_set(POLICY_EVIDENCE_BINDINGS) {
        return Err("policy evidence producer registry mismatch".to_owned());
    }
    for binding in &bindings {
        let fields = binding.split('|').collect::<Vec<_>>();
        let [obligation, producer, target] = fields.as_slice() else {
            return Err("policy evidence producer binding is malformed".to_owned());
        };
        let (goal, label) = target
            .split_once(':')
            .ok_or_else(|| "policy evidence target is malformed".to_owned())?;
        if !OBLIGATIONS
            .iter()
            .any(|row| row.starts_with(&format!("policy|{obligation}|evidence-demand|")))
            || !canonical.contains(&format!("with {producer}"))
            || !canonical.contains(&format!("§goal {goal}"))
            || !canonical.contains(&format!("\"{label}\""))
        {
            return Err("policy evidence producer binding is disconnected".to_owned());
        }
    }
    Ok(())
}

fn validate_reference_semantics(root: &Path) -> Result<(), String> {
    let directory = reference_directory(root);
    let registry = parse_registry(&read_reference(root, "registry.txt")?)?;
    let expected_registry = REGISTRY
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect::<BTreeMap<_, _>>();
    if registry != expected_registry {
        return Err("reference registry mismatch".to_owned());
    }
    for (key, value) in &registry {
        if (key.ends_with("program")
            || key.ends_with("source")
            || key.ends_with("document")
            || matches!(
                key.as_str(),
                "program-contract"
                    | "planner-input"
                    | "execution-input"
                    | "expected-obligations"
                    | "outcome-matrix"
                    | "policy-evidence-registry"
            )
            || key.ends_with("-rule"))
            && !directory.join(value).is_file()
        {
            return Err(format!("registry path {key} does not exist"));
        }
    }

    let canonical = read_reference(root, "program.bhcp")?;
    let alternate = read_reference(root, "program.words.bhcp")?;
    let (preamble, alternate_body) = alternate
        .split_once('\n')
        .ok_or_else(|| "alternate program has no preamble".to_owned())?;
    if preamble != "#!bhcp-profile bhcp.reference/review-profile@0" {
        return Err("alternate program selects the wrong profile".to_owned());
    }
    if alternate_body.replace("§intent", "§goal") != canonical {
        return Err("canonical and alternate source structures differ".to_owned());
    }

    validate_reference_policy(&read_reference(root, "policy.bhcp")?)?;

    let syntax_value = parse_diagnostic(&read_reference(root, "syntax.diag")?)
        .map_err(|error| error.to_string())?;
    validate_root(&syntax_value, "syntax").map_err(|error| error.to_string())?;
    let PresentationDocument::Syntax(syntax) =
        PresentationDocument::from_value(&syntax_value).map_err(|error| error.to_string())?
    else {
        return Err("syntax projection has the wrong root".to_owned());
    };
    if syntax.symbol != "bhcp.reference/words@0"
        || syntax.mappings.len() != 1
        || syntax.mappings[0].canonical != "goal"
        || syntax.mappings[0].surface != "intent"
    {
        return Err("syntax projection does not define the alternate source mapping".to_owned());
    }
    let syntax_source = read_reference(root, "syntax.bhcp")?;
    for marker in [
        &syntax.symbol,
        &syntax.mappings[0].canonical,
        &syntax.mappings[0].surface,
    ] {
        if !syntax_source.contains(marker) {
            return Err(format!("syntax source omits projected value {marker}"));
        }
    }

    let profile_value = parse_diagnostic(&read_reference(root, "profile.diag")?)
        .map_err(|error| error.to_string())?;
    validate_root(&profile_value, "profile").map_err(|error| error.to_string())?;
    let PresentationDocument::Profile(profile) =
        PresentationDocument::from_value(&profile_value).map_err(|error| error.to_string())?
    else {
        return Err("profile projection has the wrong root".to_owned());
    };
    if profile.symbol != "bhcp.reference/review-profile@0"
        || profile.syntax != syntax.symbol
        || profile.policy_overlays
            != [
                "bhcp.reference/organization-policy@0",
                "bhcp.reference/repository-policy@0",
            ]
    {
        return Err("profile projection is disconnected from syntax or policy".to_owned());
    }
    let profile_source = read_reference(root, "profile.bhcp")?;
    for marker in std::iter::once(profile.symbol.as_str())
        .chain(std::iter::once(profile.syntax.as_str()))
        .chain(profile.policy_overlays.iter().map(String::as_str))
    {
        if !profile_source.contains(marker) {
            return Err(format!("profile source omits projected value {marker}"));
        }
    }

    let profile_registry_source = format!(
        "{}\n{}\n{}",
        read_reference(root, "policy.bhcp")?,
        syntax_source,
        profile_source
    );
    let lowered_profiles = parse_profile_source(&profile_registry_source, "profiles.bhcp")
        .map_err(|error| error.to_string())?;
    if lowered_profiles.syntaxes.len() != 1 || lowered_profiles.profiles.len() != 1 {
        return Err("reference profile source did not lower to exactly two roots".to_owned());
    }
    if lowered_profiles.syntaxes[0].to_value(false) != syntax_value
        || lowered_profiles.profiles[0].to_value(false) != profile_value
    {
        return Err("source-lowered profile roots differ from canonical projections".to_owned());
    }
    if lowered_profiles.syntaxes[0].header.artifact_id.is_none()
        || lowered_profiles.profiles[0].header.artifact_id.is_none()
    {
        return Err("source-lowered profile roots omit artifact identities".to_owned());
    }
    let resolved_profile = lowered_profiles
        .registry
        .resolve("bhcp.reference/review-profile@0", HashAlgorithm::default())
        .map_err(|error| error.to_string())?;
    let waiver_value = parse_diagnostic(&read_reference(root, "waiver.diag")?)
        .map_err(|error| error.to_string())?;
    validate_root(&waiver_value, "waiver").map_err(|error| error.to_string())?;
    let waiver = WaiverDocument::from_value(&waiver_value).map_err(|error| error.to_string())?;
    let waived_policy = apply_waiver(
        &resolved_profile.effective_policy,
        &waiver,
        &registry["waiver-decision-at"],
        HashAlgorithm::default(),
    )
    .map_err(|error| error.to_string())?;
    let extension_source = read_reference(root, "extension.bhcp")?;
    let canonical_compilation = compile_source_with_policy(
        &format!("{extension_source}\n{canonical}"),
        "program.bhcp",
        &waived_policy,
    )
    .map_err(|error| error.to_string())?;
    let alternate_source = format!("{preamble}\n{extension_source}\n{alternate_body}");
    let unwaived_diagnostic = compile_source_bytes_with_profile_registry_and_waivers(
        alternate_source.as_bytes(),
        "program.words.bhcp",
        &lowered_profiles.registry,
        &[],
        &registry["waiver-decision-at"],
    )
    .expect_err("the resolved base policy must reject the unwaived attempt limit");
    if unwaived_diagnostic.code != "BHCP8204"
        || !unwaived_diagnostic
            .message
            .contains("exceeds the effective policy maximum")
    {
        return Err("unwaived profile did not reject the retained attempt limit".to_owned());
    }
    let alternate_compilation = compile_source_bytes_with_profile_registry_and_waivers(
        alternate_source.as_bytes(),
        "program.words.bhcp",
        &lowered_profiles.registry,
        std::slice::from_ref(&waiver),
        &registry["waiver-decision-at"],
    )
    .map_err(|error| error.to_string())?;
    if canonical_compilation.effective_policy.as_ref() != Some(&waived_policy)
        || alternate_compilation.effective_policy.as_ref() != Some(&waived_policy)
        || canonical_compilation.ir.type_mode != resolved_profile.type_mode
        || alternate_compilation.ir.type_mode != resolved_profile.type_mode
        || !canonical_compilation
            .ir
            .goals
            .iter()
            .any(|goal| goal.policy_decision.is_some())
        || !alternate_compilation
            .ir
            .goals
            .iter()
            .any(|goal| goal.policy_decision.is_some())
    {
        return Err("canonical/profile parity omitted resolved governance".to_owned());
    }
    if canonical_compilation.semantic_hash != alternate_compilation.semantic_hash
        || canonical_compilation.ast_hash == alternate_compilation.ast_hash
        || canonical_compilation.ir_hash != alternate_compilation.ir_hash
    {
        return Err(format!(
            "canonical and remapped sources do not preserve semantic identity and artifact distinction: semantic_equal={}, ast_distinct={}, ir_equal={}",
            canonical_compilation.semantic_hash == alternate_compilation.semantic_hash,
            canonical_compilation.ast_hash != alternate_compilation.ast_hash,
            canonical_compilation.ir_hash == alternate_compilation.ir_hash,
        ));
    }

    if waiver.symbol != "bhcp.reference/offline-emergency-waiver@0"
        || waiver.targets.len() != 1
        || waiver.targets[0].rule.policy != "bhcp.reference/repository-policy@0"
        || waiver.targets[0].rule.rule != "d-attempts"
        || registry["waiver-decision-at"] <= waiver.not_before
        || registry["waiver-decision-at"] >= waiver.expires_at
    {
        return Err("waiver projection is disconnected or inactive".to_owned());
    }
    let WaiverWeakening::LoosenLimit { from, to } = &waiver.targets[0].weakening else {
        return Err("reference waiver does not loosen the attempt limit".to_owned());
    };
    if from.dimension != "bhcp.reference/limit.attempts@0"
        || from.maximum != ExactNumber::Integer(2)
        || to.dimension != from.dimension
        || to.unit != from.unit
        || to.maximum != ExactNumber::Integer(3)
        || !canonical
            .contains("§limit \"attempts\": bhcp.reference/limit.attempts@0: attempts <= 3;")
        || !read_reference(root, "planner-input.txt")?
            .contains("budget = { attempts: 3, wall-time: duration \"PT10M\", processes: 4 }")
    {
        return Err(
            "reference waiver does not materially authorize the frozen attempt budget".to_owned(),
        );
    }
    let waiver_policy = parse_policy_source(&read_reference(root, "policy.bhcp")?, "policy.bhcp")
        .map_err(|error| error.to_string())?;
    let base_policy = compose_policies(&waiver_policy.documents, HashAlgorithm::default())
        .map_err(|error| error.to_string())?;
    if base_policy != resolved_profile.effective_policy {
        return Err("resolved profile policy differs from the reviewed policy source".to_owned());
    }
    if !waived_policy.effective.limits.iter().any(|rule| {
        rule.value.dimension == "bhcp.reference/limit.attempts@0"
            && rule.value.maximum == ExactNumber::Integer(3)
    }) {
        return Err("active reference waiver did not produce the attempt ceiling".to_owned());
    }
    let waiver_source = read_reference(root, "waiver.bhcp")?;
    for marker in [
        waiver.symbol.as_str(),
        waiver.targets[0].rule.policy.as_str(),
        waiver.targets[0].rule.rule.as_str(),
        waiver.not_before.as_str(),
        waiver.expires_at.as_str(),
    ] {
        if !waiver_source.contains(marker) {
            return Err(format!("waiver source omits projected value {marker}"));
        }
    }

    let extension_value = parse_diagnostic(&read_reference(root, "extension.diag")?)
        .map_err(|error| error.to_string())?;
    validate_root(&extension_value, "extension-descriptor").map_err(|error| error.to_string())?;
    if value_text(&extension_value, "symbol")? != "bhcp.reference/review@0"
        || value_text(&extension_value, "lowering")? != "bhcp.reference/lowerReview@0"
        || !canonical.contains("bhcp.reference/review@0(risk = risk)")
    {
        return Err("derived extension is disconnected from the reference program".to_owned());
    }
    let extension_source = read_reference(root, "extension.bhcp")?;
    for marker in [
        value_text(&extension_value, "symbol")?,
        value_text(&extension_value, "lowering")?,
        "bhcp.reference/reviewReducer@0",
    ] {
        if !extension_source.contains(marker) {
            return Err(format!("extension source omits projected value {marker}"));
        }
    }
    for (field, registry_key, expected_rule) in EXTENSION_RULES {
        let bytes = fs::read(directory.join(&registry[*registry_key]))
            .map_err(|error| format!("cannot read {registry_key}: {error}"))?;
        if bytes != expected_rule.as_bytes() {
            return Err(format!("extension {field} reviewed rule mismatch"));
        }
        let expected =
            ContentReference::from_bytes("text/plain", &bytes, HashAlgorithm::default()).to_value();
        if extension_value.get(field) != Some(&expected) {
            return Err(format!(
                "extension {field} does not bind its reviewed rule bytes"
            ));
        }
    }

    validate_policy_evidence_bindings(
        &canonical,
        &read_reference(root, "policy-evidence-registry.txt")?,
    )?;

    let obligations = read_reference(root, "expected-obligations.txt")?
        .lines()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if obligations != expected_set(OBLIGATIONS) {
        return Err("reference obligation inventory mismatch".to_owned());
    }
    for line in &obligations {
        let fields = line.split('|').collect::<Vec<_>>();
        if fields.len() != 5
            || !matches!(fields[4], "open" | "discharged" | "refuted" | "unresolved")
        {
            return Err("invalid obligation state".to_owned());
        }
    }

    let outcomes = read_reference(root, "outcome-matrix.txt")?
        .lines()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if outcomes != expected_set(OUTCOMES) {
        return Err("reference outcome matrix mismatch".to_owned());
    }
    let execution_input = read_reference(root, "execution-input.txt")?;
    for marker in [
        "patch = { bytes: h'00', digest: \"reference-patch\" }",
        "repository = { root: \"reference-workspace\" }",
        "risk = High",
        "tree = { name: \"repository\", depth: 0, children: [] }",
        "tree_depth = 0",
        "attempts = 0",
        "expected-output = bhcp.reference/Delivery@0",
    ] {
        if !execution_input.contains(marker) {
            return Err(format!("execution input omits {marker}"));
        }
    }
    let cases = execution_input
        .lines()
        .filter_map(|line| line.strip_prefix("case = "))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let expected_cases = OUTCOMES
        .iter()
        .filter_map(|line| line.split('|').next())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if cases != expected_cases {
        return Err("execution inputs do not cover the outcome matrix".to_owned());
    }

    let planner = read_reference(root, "planner-input.txt")?;
    for (key, value) in &registry {
        if matches!(
            key.as_str(),
            "canonical-program"
                | "alternate-program"
                | "program-contract"
                | "syntax-document"
                | "profile-document"
                | "policy-source"
                | "waiver-document"
                | "waiver-decision-at"
                | "extension-document"
                | "policy-evidence-registry"
                | "execution-input"
                | "expected-obligations"
                | "outcome-matrix"
        ) && !planner.contains(value)
        {
            return Err(format!("planner input omits registry link {key}"));
        }
    }

    validate_program_contract(root)
}

fn validate_contract(root: &Path, text: &str) -> Result<(), String> {
    let contract = parse_contract(text)?;
    if contract.version.as_deref() != Some("bhcp-v0-completion@0") {
        return Err("missing or unknown completion contract version".to_owned());
    }

    let expected_issues = ISSUE_KEYS
        .iter()
        .map(|(number, key)| (*number, (*key).to_owned()))
        .collect::<BTreeMap<_, _>>();
    if contract.issues != expected_issues {
        return Err("issue inventory mismatch".to_owned());
    }
    let issue_keys = contract.issues.values().cloned().collect::<BTreeSet<_>>();

    let readme = fs::read_to_string(root.join("conformance/v0/README.md"))
        .map_err(|error| format!("cannot read conformance README: {error}"))?;
    let expected_scenarios = markdown_scenarios(&readme);
    if contract.scenarios.keys().cloned().collect::<BTreeSet<_>>() != expected_scenarios {
        return Err("scenario inventory mismatch".to_owned());
    }
    let expected_frontend = FRONTEND_LEDGER
        .iter()
        .map(|(id, section, test, artifact)| {
            (
                (*id).to_owned(),
                FrontendScenario {
                    section: (*section).to_owned(),
                    test: (*test).to_owned(),
                    artifact: PathBuf::from(artifact),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    if contract.frontend != expected_frontend {
        return Err("front-end completion ledger mismatch".to_owned());
    }
    for (id, entry) in &contract.frontend {
        if !contract.scenarios.contains_key(id)
            || !matches!(
                entry.section.as_str(),
                "S4" | "S5" | "S6" | "S7" | "S8" | "S9"
            )
        {
            return Err(format!(
                "front-end scenario {id} is outside the S4-S9 inventory"
            ));
        }
        let (test_file, test_name) = entry
            .test
            .split_once("::")
            .ok_or_else(|| format!("front-end scenario {id} has no exact test target"))?;
        let test_source = fs::read_to_string(root.join(test_file))
            .map_err(|error| format!("cannot read front-end test {test_file}: {error}"))?;
        if !test_source.contains(&format!("fn {test_name}(")) {
            return Err(format!(
                "front-end scenario {id} names unknown test {}",
                entry.test
            ));
        }
        if entry.artifact.is_absolute()
            || entry
                .artifact
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
            || !root.join(&entry.artifact).is_file()
        {
            return Err(format!(
                "front-end scenario {id} names an invalid exact artifact"
            ));
        }
    }

    let graph_scenarios = GRAPH_APPLICABLE_SCENARIOS
        .iter()
        .map(|id| (*id).to_owned())
        .collect::<BTreeSet<_>>();
    if contract.graphs.keys().cloned().collect::<BTreeSet<_>>() != graph_scenarios {
        return Err("graph completion ledger mismatch".to_owned());
    }
    let graph_roots = BTreeSet::from([
        "obligation-graph".to_owned(),
        "capability-graph".to_owned(),
        "state-graph".to_owned(),
    ]);
    let allowed_diagnostics = expected_set(GRAPH_DIAGNOSTICS);
    let identity_path = PathBuf::from("conformance/v0/reference-program/graph-identities.txt");
    let mut covered_diagnostics = BTreeSet::new();
    for (id, entry) in &contract.graphs {
        if !contract.scenarios.contains_key(id)
            || entry.roots.is_empty()
            || !entry.roots.is_subset(&graph_roots)
            || entry.diagnostics.is_empty()
            || !entry.diagnostics.is_subset(&allowed_diagnostics)
            || entry.identity != identity_path
        {
            return Err(format!("graph scenario {id} has incomplete exact evidence"));
        }
        let (test_file, test_name) = entry
            .test
            .split_once("::")
            .ok_or_else(|| format!("graph scenario {id} has no exact test target"))?;
        let test_source = fs::read_to_string(root.join(test_file))
            .map_err(|error| format!("cannot read graph test {test_file}: {error}"))?;
        if !test_source.contains(&format!("fn {test_name}(")) {
            return Err(format!(
                "graph scenario {id} names unknown test {}",
                entry.test
            ));
        }
        covered_diagnostics.extend(entry.diagnostics.iter().cloned());
    }
    for required in [
        "BHCP7501", "BHCP7502", "BHCP7503", "BHCP7504", "BHCP7505", "BHCP7506", "BHCP7507",
    ] {
        if !covered_diagnostics.contains(required) {
            return Err(format!(
                "graph ledger omits cross-graph diagnostic {required}"
            ));
        }
    }

    let example_manifest = fs::read_to_string(root.join("schemas/v0/examples/manifest.txt"))
        .map_err(|error| format!("cannot read schema fixture manifest: {error}"))?;
    let expected_roots = schema_roots(&example_manifest);
    if expected_roots.len() != 17 {
        return Err("wire root inventory no longer contains 17 kinds".to_owned());
    }
    if contract.roots.keys().cloned().collect::<BTreeSet<_>>() != expected_roots {
        return Err("wire root inventory mismatch".to_owned());
    }
    let schema = fs::read_to_string(root.join("schemas/v0/bhcp-v0.cddl"))
        .map_err(|error| format!("cannot read v0 schema: {error}"))?;
    let root_refs = expected_roots
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    validate_schema_inventory(&schema, &root_refs).map_err(|error| error.to_string())?;

    if contract.stages.keys().cloned().collect::<BTreeSet<_>>() != expected_set(STAGES) {
        return Err("pipeline stage inventory mismatch".to_owned());
    }
    if contract.artifacts.keys().cloned().collect::<BTreeSet<_>>() != expected_set(ARTIFACTS) {
        return Err("reference artifact inventory mismatch".to_owned());
    }
    if contract.features.keys().cloned().collect::<BTreeSet<_>>() != expected_set(FEATURES) {
        return Err("reference feature inventory mismatch".to_owned());
    }
    if contract.non_goals != expected_set(NON_GOALS) {
        return Err("v0 non-goal inventory mismatch".to_owned());
    }

    for owner in contract
        .scenarios
        .values()
        .chain(contract.roots.values())
        .chain(
            contract
                .stages
                .values()
                .flat_map(|stage| stage.owners.iter()),
        )
        .chain(
            contract
                .features
                .values()
                .flat_map(|feature| feature.owners.iter()),
        )
    {
        if !issue_keys.contains(owner) {
            return Err(format!("unknown issue key {owner}"));
        }
    }
    if contract
        .stages
        .values()
        .any(|stage| stage.outcome.is_empty())
    {
        return Err("pipeline stage has no observable outcome".to_owned());
    }

    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("cannot resolve repository root: {error}"))?;
    for (id, relative) in &contract.artifacts {
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(format!("reference artifact {id} has an unsafe path"));
        }
        let path = root.join(relative);
        if !path.is_file() {
            return Err(format!(
                "reference artifact {id} does not exist: {relative:?}"
            ));
        }
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot inspect reference artifact {id}: {error}"))?;
        let resolved = fs::canonicalize(&path)
            .map_err(|error| format!("cannot resolve reference artifact {id}: {error}"))?;
        if metadata.file_type().is_symlink() || !resolved.starts_with(&canonical_root) {
            return Err(format!("reference artifact {id} escapes the repository"));
        }
    }
    for (id, feature) in &contract.features {
        let relative = contract
            .artifacts
            .get(&feature.artifact)
            .ok_or_else(|| format!("feature {id} names unknown artifact {}", feature.artifact))?;
        let body = fs::read_to_string(root.join(relative))
            .map_err(|error| format!("cannot read feature artifact {id}: {error}"))?;
        for needle in &feature.needles {
            if needle.is_empty() || !body.contains(needle) {
                return Err(format!("feature {id} is missing marker {needle:?}"));
            }
        }
    }

    validate_reference_semantics(root)?;

    Ok(())
}

fn manifest_text() -> String {
    fs::read_to_string(repository().join("conformance/v0/completion-manifest.txt"))
        .expect("the v0 completion manifest must be checked in")
}

#[test]
fn completion_manifest_closes_the_normative_inventory() {
    let text = manifest_text();
    validate_contract(&repository(), &text).unwrap();
}

#[test]
fn completion_manifest_rejects_omitted_duplicate_and_unknown_records() {
    let text = manifest_text();
    let omitted = text.replacen("scenario|SYN-01|profile-source-lowering\n", "", 1);
    assert_eq!(
        validate_contract(&repository(), &omitted).unwrap_err(),
        "scenario inventory mismatch"
    );

    let duplicate = format!("{text}scenario|SYN-01|profile-source-lowering\n");
    assert_eq!(
        validate_contract(&repository(), &duplicate).unwrap_err(),
        "duplicate scenario"
    );

    let unknown = text.replacen(
        "scenario|SYN-01|profile-source-lowering",
        "scenario|SYN-99|profile-source-lowering",
        1,
    );
    assert_eq!(
        validate_contract(&repository(), &unknown).unwrap_err(),
        "scenario inventory mismatch"
    );

    let unknown_owner = text.replacen(
        "scenario|SYN-01|profile-source-lowering",
        "scenario|SYN-01|not-a-roadmap-issue",
        1,
    );
    assert_eq!(
        validate_contract(&repository(), &unknown_owner).unwrap_err(),
        "unknown issue key not-a-roadmap-issue"
    );
}

#[test]
fn reference_program_reaches_governed_semantic_ir() {
    let root = repository().join("conformance/v0/reference-program");
    let source = format!(
        "{}\n{}",
        fs::read_to_string(root.join("extension.bhcp")).unwrap(),
        fs::read_to_string(root.join("program.bhcp")).unwrap()
    );
    let compilation = compile_source(&source, "reference-program/program.bhcp")
        .expect("the complete v0 front end must lower the frozen reference program");
    compilation.ir.validate().unwrap();
}

#[test]
fn reference_validators_reject_invalid_policy_shapes_and_ownership() {
    let root = repository();
    let policy = read_reference(&root, "policy.bhcp").unwrap().replacen(
        "dimension: bhcp.reference/limit.attempts@0",
        "dimension: attempts",
        1,
    );
    let diagnostic = parse_policy_source(&policy, "invalid-policy.bhcp").unwrap_err();
    assert_eq!(diagnostic.code, "BHCP8001");
    assert!(diagnostic.message.contains("dimension must be a symbol-id"));

    let projection = read_reference(&root, "program-contract.txt")
        .unwrap()
        .replacen("|patch|move|input.patch", "|patch|copy|input.patch", 1);
    assert!(
        validate_program_projection(&root, &projection)
            .unwrap_err()
            .contains("must move owned values")
    );

    let policy_scope = read_reference(&root, "policy.bhcp").unwrap().replacen(
        "[bhcp.reference/DeliverChange@0, bhcp.reference/Persist@0]",
        "[bhcp.reference/DeliverChange@0]",
        1,
    );
    let scope_error = validate_reference_policy(&policy_scope).unwrap_err();
    assert!(
        scope_error.contains("BHCP8101")
            || scope_error
                == "reference policy does not authorize the declared effects and base limit"
    );

    let typed_projection = read_reference(&root, "program-contract.txt").unwrap();
    for (from, to) in [
        ("|attempts|3|Bool", "|attempts|3|Integer"),
        ("|remaining|remaining - 1|", "|remaining|remaining + 1|"),
        ("|bhcp-effect/fs.read@0|", "|fs.read|"),
        ("|{}|Reduction<Unit>|", "|{}|Unit|"),
        ("|input-free|-", "|parent-input|input.patch"),
    ] {
        let mutation = typed_projection.replacen(from, to, 1);
        assert_eq!(
            validate_program_projection(&root, &mutation).unwrap_err(),
            "program semantic projection mismatch"
        );
    }

    let canonical = read_reference(&root, "program.bhcp").unwrap();
    let owner_type_drift = canonical.replacen(
        "§input risk: bhcp.reference/Risk@0;\n    §input tree: bhcp.reference/Node@0;",
        "§input risk: Text;\n    §input tree: bhcp.reference/Node@0;",
        1,
    );
    let owner_error = validate_program_projection_sources(
        &typed_projection,
        &owner_type_drift,
        &read_reference(&root, "extension.bhcp").unwrap(),
    )
    .unwrap_err();
    assert!(owner_error.contains("typed fact projection"));

    let undefined_repository = canonical.replacen(
        "§type bhcp.reference/Repository@0 = { root: Text };\n",
        "",
        1,
    );
    assert!(
        validate_program_projection_sources(
            &typed_projection,
            &undefined_repository,
            &read_reference(&root, "extension.bhcp").unwrap(),
        )
        .unwrap_err()
        .contains("occurs 0 times")
    );

    let omitted_obligation = canonical.replacen(
        "    §ensures \"stored\": bhcp.reference/hasStoredDigest@0(receipt);\n",
        "",
        1,
    );
    assert!(
        validate_program_projection_sources(
            &typed_projection,
            &omitted_obligation,
            &read_reference(&root, "extension.bhcp").unwrap(),
        )
        .unwrap_err()
        .contains("clause projection")
    );

    let type_shape_drift =
        canonical.replacen("sequence: { confirmation: Text }", "sequence: Bool", 1);
    assert!(
        validate_program_projection_sources(
            &typed_projection,
            &type_shape_drift,
            &read_reference(&root, "extension.bhcp").unwrap(),
        )
        .unwrap_err()
        .contains("type definition projection")
    );

    let input_drift = canonical.replacen(
        "§goal bhcp.reference/StartDelivery@0 {\n    §output token: Text;",
        "§goal bhcp.reference/StartDelivery@0 {\n    §input seed: Text;\n    §output token: Text;",
        1,
    );
    assert!(
        validate_program_projection_sources(
            &typed_projection,
            &input_drift,
            &read_reference(&root, "extension.bhcp").unwrap(),
        )
        .unwrap_err()
        .contains("typed fact projection")
    );

    let unprojected_top_level = canonical.replacen(
        "§type bhcp.reference/Risk@0 = variant { Low, High };",
        "§type bhcp.reference/Risk@0 = variant { Low, High };\n§waiver bhcp.reference/hidden@0;",
        1,
    );
    assert!(
        validate_program_projection_sources(
            &typed_projection,
            &unprojected_top_level,
            &read_reference(&root, "extension.bhcp").unwrap(),
        )
        .unwrap_err()
        .contains("unprojected top-level construct")
    );

    let indented_top_level = format!("  §waiver bhcp.reference/hidden@0;\n{canonical}");
    assert!(
        validate_program_projection_sources(
            &typed_projection,
            &indented_top_level,
            &read_reference(&root, "extension.bhcp").unwrap(),
        )
        .unwrap_err()
        .contains("whole source hash")
    );

    let bindings = read_reference(&root, "policy-evidence-registry.txt")
        .unwrap()
        .replacen(
            "bhcp.verifier/static-analysis@0",
            "bhcp.verifier/missing@0",
            1,
        );
    assert_eq!(
        validate_policy_evidence_bindings(
            &read_reference(&root, "program.bhcp").unwrap(),
            &bindings,
        )
        .unwrap_err(),
        "policy evidence producer registry mismatch"
    );
}
