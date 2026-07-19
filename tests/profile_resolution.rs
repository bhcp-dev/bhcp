use bhcp::hash::HashAlgorithm;
use bhcp::pipeline::{
    compile_source_bytes_with_profile_registry, compile_source_with_policy, parse_policy_source,
};
use bhcp::policy::{SourcePolicyDocument, TypeMode};
use bhcp::profile::{
    FormattingRules, PresentationHeader, ProfileDocument, ProfileRegistry, SyntaxDocument,
    SyntaxMapping, SyntaxMappingCategory,
};

fn header() -> PresentationHeader {
    PresentationHeader {
        features: vec![],
        semantic_id: None,
        artifact_id: None,
        provenance: None,
        authorization: None,
    }
}

fn mapping(
    category: SyntaxMappingCategory,
    canonical: &str,
    surface: &str,
) -> SyntaxMapping {
    SyntaxMapping {
        category,
        canonical: canonical.to_owned(),
        surface: surface.to_owned(),
    }
}

fn syntax(
    symbol: &str,
    extends: Option<&str>,
    mappings: Vec<SyntaxMapping>,
) -> SyntaxDocument {
    SyntaxDocument {
        header: header(),
        symbol: symbol.to_owned(),
        extends: extends.map(str::to_owned),
        mappings,
        formatting: FormattingRules {
            indent_width: 4,
            line_width: 100,
            final_newline: true,
        },
    }
}

fn profile(
    symbol: &str,
    extends: Option<&str>,
    syntax: &str,
    overlays: &[&str],
    type_mode: TypeMode,
) -> ProfileDocument {
    ProfileDocument {
        header: header(),
        symbol: symbol.to_owned(),
        extends: extends.map(str::to_owned),
        syntax: syntax.to_owned(),
        policy_overlays: overlays.iter().map(|overlay| (*overlay).to_owned()).collect(),
        type_mode,
    }
}

fn policy(source: &str) -> SourcePolicyDocument {
    parse_policy_source(source, "overlay.bhcp")
        .unwrap()
        .documents
        .into_iter()
        .next()
        .unwrap()
}

const ORG: &str = r#"
§policy example/policy.org@0 {
  layer organization;
  rule a-mode: type-mode strengthen gradual nonwaivable;
}
"#;

const REPO: &str = r#"
§policy example/policy.repo@0 {
  layer repository;
  rule b-mode: type-mode strengthen infer-strict nonwaivable;
}
"#;

fn documents() -> (
    Vec<SyntaxDocument>,
    Vec<ProfileDocument>,
    Vec<SourcePolicyDocument>,
) {
    let syntaxes = vec![
        syntax(
            "example/syntax.base@0",
            None,
            vec![
                mapping(SyntaxMappingCategory::Keyword, "goal", "outcome"),
                mapping(SyntaxMappingCategory::Sigil, "§", "$"),
            ],
        ),
        syntax(
            "example/syntax.child@0",
            Some("example/syntax.base@0"),
            vec![
                mapping(SyntaxMappingCategory::OpenDelimiter, "{", "^"),
                mapping(SyntaxMappingCategory::CloseDelimiter, "}", "~"),
                mapping(SyntaxMappingCategory::Terminator, ";", "?"),
            ],
        ),
    ];
    let profiles = vec![
        profile(
            "example/profile.base@0",
            None,
            "example/syntax.base@0",
            &["example/policy.org@0"],
            TypeMode::Gradual,
        ),
        profile(
            "example/profile.child@0",
            Some("example/profile.base@0"),
            "example/syntax.child@0",
            &["example/policy.repo@0"],
            TypeMode::InferStrict,
        ),
    ];
    (syntaxes, profiles, vec![policy(ORG), policy(REPO)])
}

fn registry(reverse: bool) -> ProfileRegistry {
    let (mut syntaxes, mut profiles, mut policies) = documents();
    if reverse {
        syntaxes.reverse();
        profiles.reverse();
        policies.reverse();
    }
    let mut registry = ProfileRegistry::new();
    for document in syntaxes {
        registry.register_syntax(document).unwrap();
    }
    for document in profiles {
        registry.register_profile(document).unwrap();
    }
    for document in policies {
        registry.register_policy(document).unwrap();
    }
    registry
}

#[test]
fn syntax_profile_and_overlay_chains_resolve_root_to_leaf_deterministically() {
    let forward = registry(false)
        .resolve("example/profile.child@0", HashAlgorithm::default())
        .unwrap();
    let reverse = registry(true)
        .resolve("example/profile.child@0", HashAlgorithm::default())
        .unwrap();

    assert_eq!(forward.to_value(), reverse.to_value());
    assert_eq!(
        forward.syntax_chain,
        ["example/syntax.base@0", "example/syntax.child@0"]
    );
    assert_eq!(
        forward.profile_chain,
        ["example/profile.base@0", "example/profile.child@0"]
    );
    assert_eq!(
        forward.policy_overlays,
        ["example/policy.org@0", "example/policy.repo@0"]
    );
    assert_eq!(forward.type_mode, TypeMode::InferStrict);
    assert_eq!(forward.syntax.extends, None);
    assert_eq!(
        forward
            .syntax
            .mappings
            .iter()
            .map(|mapping| (mapping.category, mapping.canonical.as_str(), mapping.surface.as_str()))
            .collect::<Vec<_>>(),
        [
            (SyntaxMappingCategory::Keyword, "goal", "outcome"),
            (SyntaxMappingCategory::Sigil, "§", "$"),
            (SyntaxMappingCategory::OpenDelimiter, "{", "^"),
            (SyntaxMappingCategory::CloseDelimiter, "}", "~"),
            (SyntaxMappingCategory::Terminator, ";", "?"),
        ]
    );
    assert_eq!(
        forward.effective_policy.effective.type_mode.value,
        TypeMode::InferStrict
    );
}

const CANONICAL: &str = r#"§goal example/G@0 {
  §input value: Text;
  §output result: Text;
}
"#;

const CUSTOM: &str = r#"#!bhcp-profile example/profile.child@0
$outcome example/G@0 ^
  $input value: Text?
  $output result: Text?
~
"#;

#[test]
fn resolved_profile_compilation_preserves_meaning_and_applies_overlays_before_elaboration() {
    let registry = registry(false);
    let resolved = registry
        .resolve("example/profile.child@0", HashAlgorithm::default())
        .unwrap();
    let canonical =
        compile_source_with_policy(CANONICAL, "canonical.bhcp", &resolved.effective_policy)
            .unwrap();
    let custom = compile_source_bytes_with_profile_registry(
        CUSTOM.as_bytes(),
        "custom.bhcp",
        &registry,
    )
    .unwrap();

    assert_eq!(custom.semantic_hash, canonical.semantic_hash);
    assert_ne!(custom.ast_hash, canonical.ast_hash);
    assert_eq!(custom.ast.profile, "example/profile.child@0");
    assert_eq!(
        custom.ir.goals[0]
            .policy_decision
            .as_ref()
            .unwrap()
            .type_mode,
        "infer-strict"
    );
    assert_eq!(
        custom.effective_policy.as_ref().unwrap(),
        &resolved.effective_policy
    );
}

#[test]
fn missing_cycles_unrelated_syntax_weaker_modes_and_duplicate_overlays_fail_stably() {
    let root_syntax = syntax("example/syntax.root@0", None, vec![]);
    let other_syntax = syntax("example/syntax.other@0", None, vec![]);

    let cases: Vec<(&str, Vec<SyntaxDocument>, Vec<ProfileDocument>)> = vec![
        (
            "missing-profile-parent",
            vec![root_syntax.clone()],
            vec![profile(
                "example/profile.child@0",
                Some("example/profile.absent@0"),
                "example/syntax.root@0",
                &[],
                TypeMode::Dynamic,
            )],
        ),
        (
            "profile-inheritance-cycle",
            vec![root_syntax.clone()],
            vec![
                profile(
                    "example/profile.a@0",
                    Some("example/profile.b@0"),
                    "example/syntax.root@0",
                    &[],
                    TypeMode::Dynamic,
                ),
                profile(
                    "example/profile.b@0",
                    Some("example/profile.a@0"),
                    "example/syntax.root@0",
                    &[],
                    TypeMode::Dynamic,
                ),
            ],
        ),
        (
            "syntax-inheritance-cycle",
            vec![
                syntax(
                    "example/syntax.a@0",
                    Some("example/syntax.b@0"),
                    vec![],
                ),
                syntax(
                    "example/syntax.b@0",
                    Some("example/syntax.a@0"),
                    vec![],
                ),
            ],
            vec![profile(
                "example/profile.child@0",
                None,
                "example/syntax.a@0",
                &[],
                TypeMode::Dynamic,
            )],
        ),
        (
            "unrelated-syntax",
            vec![root_syntax.clone(), other_syntax],
            vec![
                profile(
                    "example/profile.base@0",
                    None,
                    "example/syntax.root@0",
                    &[],
                    TypeMode::Gradual,
                ),
                profile(
                    "example/profile.child@0",
                    Some("example/profile.base@0"),
                    "example/syntax.other@0",
                    &[],
                    TypeMode::InferStrict,
                ),
            ],
        ),
        (
            "weaker-type-mode",
            vec![root_syntax.clone()],
            vec![
                profile(
                    "example/profile.base@0",
                    None,
                    "example/syntax.root@0",
                    &[],
                    TypeMode::Strict,
                ),
                profile(
                    "example/profile.child@0",
                    Some("example/profile.base@0"),
                    "example/syntax.root@0",
                    &[],
                    TypeMode::InferStrict,
                ),
            ],
        ),
        (
            "duplicate-overlay",
            vec![root_syntax],
            vec![
                profile(
                    "example/profile.base@0",
                    None,
                    "example/syntax.root@0",
                    &["example/policy.org@0"],
                    TypeMode::Gradual,
                ),
                profile(
                    "example/profile.child@0",
                    Some("example/profile.base@0"),
                    "example/syntax.root@0",
                    &["example/policy.org@0"],
                    TypeMode::Gradual,
                ),
            ],
        ),
    ];

    for (expected, syntaxes, profiles) in cases {
        let leaf = profiles.last().unwrap().symbol.clone();
        let mut registry = ProfileRegistry::new();
        for document in syntaxes {
            registry.register_syntax(document).unwrap();
        }
        for document in profiles {
            registry.register_profile(document).unwrap();
        }
        registry.register_policy(policy(ORG)).unwrap();
        let diagnostic = registry
            .resolve(&leaf, HashAlgorithm::default())
            .expect_err(expected);
        assert_eq!(diagnostic.code, "BHCP9003", "{expected}");
        assert!(diagnostic.message.contains(expected), "{diagnostic:?}");
    }
}

#[test]
fn inherited_mapping_conflicts_missing_overlays_and_policy_weakening_fail_closed() {
    let mut conflict = ProfileRegistry::new();
    conflict
        .register_syntax(syntax(
            "example/syntax.base@0",
            None,
            vec![mapping(
                SyntaxMappingCategory::Keyword,
                "goal",
                "outcome",
            )],
        ))
        .unwrap();
    conflict
        .register_syntax(syntax(
            "example/syntax.child@0",
            Some("example/syntax.base@0"),
            vec![mapping(
                SyntaxMappingCategory::Keyword,
                "input",
                "outcome",
            )],
        ))
        .unwrap();
    conflict
        .register_profile(profile(
            "example/profile.child@0",
            None,
            "example/syntax.child@0",
            &[],
            TypeMode::InferStrict,
        ))
        .unwrap();
    let diagnostic = conflict
        .resolve("example/profile.child@0", HashAlgorithm::default())
        .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP9002");
    assert!(diagnostic.message.contains("ambiguous-surface"));

    let mut missing_overlay = ProfileRegistry::new();
    missing_overlay
        .register_syntax(syntax("example/syntax.root@0", None, vec![]))
        .unwrap();
    missing_overlay
        .register_profile(profile(
            "example/profile.child@0",
            None,
            "example/syntax.root@0",
            &["example/policy.absent@0"],
            TypeMode::InferStrict,
        ))
        .unwrap();
    let diagnostic = missing_overlay
        .resolve("example/profile.child@0", HashAlgorithm::default())
        .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP9003");
    assert!(diagnostic.message.contains("missing-policy-overlay"));

    let strict = r#"
§policy example/policy.strict@0 {
  layer organization;
  rule a-mode: type-mode strengthen strict nonwaivable;
}
"#;
    let weaker = r#"
§policy example/policy.weaker@0 {
  layer repository;
  rule b-mode: type-mode strengthen gradual nonwaivable;
}
"#;
    let mut weakening = ProfileRegistry::new();
    weakening
        .register_syntax(syntax("example/syntax.root@0", None, vec![]))
        .unwrap();
    weakening
        .register_profile(profile(
            "example/profile.child@0",
            None,
            "example/syntax.root@0",
            &["example/policy.strict@0", "example/policy.weaker@0"],
            TypeMode::Strict,
        ))
        .unwrap();
    weakening.register_policy(policy(strict)).unwrap();
    weakening.register_policy(policy(weaker)).unwrap();
    let diagnostic = weakening
        .resolve("example/profile.child@0", HashAlgorithm::default())
        .unwrap_err();
    assert_eq!(diagnostic.code, "BHCP8103");
}
