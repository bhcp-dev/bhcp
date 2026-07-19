use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Category {
    Keyword,
    Sigil,
    OpenDelimiter,
    CloseDelimiter,
    Terminator,
    Alias,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Mapping {
    category: Category,
    canonical: &'static str,
    surface: &'static str,
}

#[derive(Clone, Debug)]
struct Syntax {
    parent: Option<&'static str>,
    mappings: Vec<Mapping>,
}

#[derive(Clone, Debug)]
struct Profile {
    parent: Option<&'static str>,
    syntax: &'static str,
    overlays: Vec<&'static str>,
    type_mode: &'static str,
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn mapping(category: Category, canonical: &'static str, surface: &'static str) -> Mapping {
    Mapping {
        category,
        canonical,
        surface,
    }
}

fn canonical_category(canonical: &str) -> Option<Category> {
    match canonical {
        "§goal" | "§input" | "§output" | "§requires" | "§ensures" | "§allows" | "§forbids"
        | "§limit" | "§prefer" | "§verify" => Some(Category::Keyword),
        "§" => Some(Category::Sigil),
        "{" | "(" | "[" => Some(Category::OpenDelimiter),
        "}" | ")" | "]" => Some(Category::CloseDelimiter),
        ";" => Some(Category::Terminator),
        value if value.contains('/') && value.contains('@') => Some(Category::Alias),
        _ => None,
    }
}

fn punctuation(category: Category) -> bool {
    matches!(
        category,
        Category::Sigil | Category::OpenDelimiter | Category::CloseDelimiter | Category::Terminator
    )
}

fn resolve_syntax(
    symbol: &'static str,
    registry: &BTreeMap<&'static str, Syntax>,
) -> Result<BTreeMap<(Category, &'static str), Mapping>, &'static str> {
    fn visit(
        symbol: &'static str,
        registry: &BTreeMap<&'static str, Syntax>,
        active: &mut BTreeSet<&'static str>,
    ) -> Result<BTreeMap<(Category, &'static str), Mapping>, &'static str> {
        let syntax = registry.get(symbol).ok_or("missing-parent")?;
        if !active.insert(symbol) {
            return Err("inheritance-cycle");
        }
        let mut effective = match syntax.parent {
            Some(parent) => visit(parent, registry, active)?,
            None => BTreeMap::new(),
        };
        let mut local = BTreeSet::new();
        for candidate in &syntax.mappings {
            if candidate.surface.is_empty()
                || candidate.surface.chars().any(char::is_whitespace)
                || candidate.surface.chars().any(char::is_control)
            {
                return Err("invalid-surface");
            }
            if canonical_category(candidate.canonical) != Some(candidate.category) {
                return Err("category-mismatch");
            }
            let coordinate = (candidate.category, candidate.canonical);
            if !local.insert(coordinate) {
                return Err("duplicate-coordinate");
            }
            effective.insert(coordinate, candidate.clone());
        }
        active.remove(symbol);

        let mut surfaces = BTreeMap::new();
        for candidate in effective.values() {
            if let Some(existing) = surfaces.insert(candidate.surface, candidate.canonical)
                && existing != candidate.canonical
            {
                return Err("ambiguous-surface");
            }
            if candidate.category == Category::Alias
                && candidate.surface.starts_with("bhcp/")
                && candidate.surface != candidate.canonical
            {
                return Err("core-override");
            }
        }
        let punctuation_surfaces: Vec<_> = effective
            .values()
            .filter(|candidate| punctuation(candidate.category))
            .map(|candidate| candidate.surface)
            .collect();
        for left in &punctuation_surfaces {
            for right in &punctuation_surfaces {
                if left != right && right.starts_with(left) {
                    return Err("punctuation-prefix");
                }
            }
        }
        let aliases: Vec<_> = effective
            .values()
            .filter(|candidate| candidate.category == Category::Alias)
            .collect();
        if aliases
            .iter()
            .any(|left| aliases.iter().any(|right| left.canonical == right.surface))
        {
            return Err("recursive-alias");
        }
        Ok(effective)
    }

    visit(symbol, registry, &mut BTreeSet::new())
}

fn syntax_is_descendant(
    child: &'static str,
    ancestor: &'static str,
    syntaxes: &BTreeMap<&'static str, Syntax>,
) -> bool {
    let mut cursor = Some(child);
    let mut seen = BTreeSet::new();
    while let Some(symbol) = cursor {
        if symbol == ancestor {
            return true;
        }
        if !seen.insert(symbol) {
            return false;
        }
        cursor = syntaxes.get(symbol).and_then(|syntax| syntax.parent);
    }
    false
}

fn mode_rank(mode: &str) -> Option<u8> {
    match mode {
        "dynamic" => Some(0),
        "gradual" => Some(1),
        "infer-strict" => Some(2),
        "strict" => Some(3),
        _ => None,
    }
}

fn resolve_profile(
    symbol: &'static str,
    profiles: &BTreeMap<&'static str, Profile>,
    syntaxes: &BTreeMap<&'static str, Syntax>,
) -> Result<Vec<&'static str>, &'static str> {
    fn visit(
        symbol: &'static str,
        profiles: &BTreeMap<&'static str, Profile>,
        syntaxes: &BTreeMap<&'static str, Syntax>,
        active: &mut BTreeSet<&'static str>,
    ) -> Result<(Vec<&'static str>, &'static str, &'static str), &'static str> {
        let profile = profiles.get(symbol).ok_or("missing-parent")?;
        resolve_syntax(profile.syntax, syntaxes)?;
        if !active.insert(symbol) {
            return Err("inheritance-cycle");
        }
        let (mut overlays, inherited_syntax, inherited_mode) = match profile.parent {
            Some(parent) => visit(parent, profiles, syntaxes, active)?,
            None => (Vec::new(), profile.syntax, profile.type_mode),
        };
        if profile.parent.is_some() {
            if !syntax_is_descendant(profile.syntax, inherited_syntax, syntaxes) {
                return Err("unrelated-syntax");
            }
            if mode_rank(profile.type_mode).ok_or("invalid-type-mode")?
                < mode_rank(inherited_mode).ok_or("invalid-type-mode")?
            {
                return Err("weaker-type-mode");
            }
        }
        for overlay in &profile.overlays {
            if overlays.contains(overlay) {
                return Err("duplicate-overlay");
            }
            overlays.push(overlay);
        }
        active.remove(symbol);
        Ok((overlays, profile.syntax, profile.type_mode))
    }

    visit(symbol, profiles, syntaxes, &mut BTreeSet::new()).map(|resolved| resolved.0)
}

fn base_syntaxes() -> BTreeMap<&'static str, Syntax> {
    BTreeMap::from([
        (
            "canonical",
            Syntax {
                parent: None,
                mappings: vec![],
            },
        ),
        (
            "words",
            Syntax {
                parent: Some("canonical"),
                mappings: vec![mapping(Category::Keyword, "§goal", "outcome")],
            },
        ),
    ])
}

#[test]
fn syntax_resolution_vectors_pin_safe_overrides_and_every_conflict_class() {
    let mut syntaxes = base_syntaxes();
    syntaxes.insert(
        "layout",
        Syntax {
            parent: Some("words"),
            mappings: vec![
                mapping(Category::OpenDelimiter, "{", "<"),
                mapping(Category::CloseDelimiter, "}", ">"),
                mapping(Category::Alias, "example/Check@0", "check"),
            ],
        },
    );
    let resolved = resolve_syntax("layout", &syntaxes).unwrap();
    assert_eq!(resolved[&(Category::Keyword, "§goal")].surface, "outcome");
    assert_eq!(resolved[&(Category::OpenDelimiter, "{")].surface, "<");

    let cases = [
        (
            "duplicate",
            Syntax {
                parent: None,
                mappings: vec![
                    mapping(Category::Keyword, "§goal", "goal"),
                    mapping(Category::Keyword, "§goal", "outcome"),
                ],
            },
            "duplicate-coordinate",
        ),
        (
            "ambiguous",
            Syntax {
                parent: None,
                mappings: vec![
                    mapping(Category::Keyword, "§goal", "same"),
                    mapping(Category::Keyword, "§input", "same"),
                ],
            },
            "ambiguous-surface",
        ),
        (
            "category",
            Syntax {
                parent: None,
                mappings: vec![mapping(Category::Keyword, "{", "begin")],
            },
            "category-mismatch",
        ),
        (
            "prefix",
            Syntax {
                parent: None,
                mappings: vec![
                    mapping(Category::OpenDelimiter, "{", "<"),
                    mapping(Category::CloseDelimiter, "}", "</"),
                ],
            },
            "punctuation-prefix",
        ),
        (
            "alias-chain",
            Syntax {
                parent: None,
                mappings: vec![
                    mapping(Category::Alias, "example/B@0", "example/A@0"),
                    mapping(Category::Alias, "example/C@0", "example/B@0"),
                ],
            },
            "recursive-alias",
        ),
        (
            "core",
            Syntax {
                parent: None,
                mappings: vec![mapping(
                    Category::Alias,
                    "example/reducer@0",
                    "bhcp/prelude.all-reducer@0",
                )],
            },
            "core-override",
        ),
    ];
    for (name, syntax, expected) in cases {
        let registry = BTreeMap::from([(name, syntax)]);
        assert_eq!(resolve_syntax(name, &registry), Err(expected), "{name}");
    }

    let missing = BTreeMap::from([(
        "child",
        Syntax {
            parent: Some("absent"),
            mappings: vec![],
        },
    )]);
    assert_eq!(resolve_syntax("child", &missing), Err("missing-parent"));
    let cycle = BTreeMap::from([
        (
            "a",
            Syntax {
                parent: Some("b"),
                mappings: vec![],
            },
        ),
        (
            "b",
            Syntax {
                parent: Some("a"),
                mappings: vec![],
            },
        ),
    ]);
    assert_eq!(resolve_syntax("a", &cycle), Err("inheritance-cycle"));
}

#[test]
fn profile_resolution_vectors_pin_parent_overlay_and_type_mode_order() {
    let syntaxes = base_syntaxes();
    let profiles = BTreeMap::from([
        (
            "base",
            Profile {
                parent: None,
                syntax: "canonical",
                overlays: vec!["example/policy.org@0"],
                type_mode: "gradual",
            },
        ),
        (
            "child",
            Profile {
                parent: Some("base"),
                syntax: "words",
                overlays: vec!["example/policy.team@0", "example/policy.repo@0"],
                type_mode: "infer-strict",
            },
        ),
    ]);
    assert_eq!(
        resolve_profile("child", &profiles, &syntaxes).unwrap(),
        [
            "example/policy.org@0",
            "example/policy.team@0",
            "example/policy.repo@0"
        ]
    );

    for (name, profile, expected) in [
        (
            "weaker",
            Profile {
                parent: Some("child"),
                syntax: "words",
                overlays: vec![],
                type_mode: "gradual",
            },
            "weaker-type-mode",
        ),
        (
            "duplicate",
            Profile {
                parent: Some("child"),
                syntax: "words",
                overlays: vec!["example/policy.org@0"],
                type_mode: "strict",
            },
            "duplicate-overlay",
        ),
        (
            "unrelated",
            Profile {
                parent: Some("child"),
                syntax: "canonical",
                overlays: vec![],
                type_mode: "strict",
            },
            "unrelated-syntax",
        ),
    ] {
        let mut candidate = profiles.clone();
        candidate.insert(name, profile);
        assert_eq!(
            resolve_profile(name, &candidate, &syntaxes),
            Err(expected),
            "{name}"
        );
    }
}

#[test]
fn semantics_and_wire_contract_name_the_closed_decision_boundaries() {
    let semantics = fs::read_to_string(root().join("SEMANTICS.md")).unwrap();
    for heading in [
        "#### S9.1.1 Mapping vocabulary and lexical safety",
        "#### S9.1.2 Syntax inheritance and conflict resolution",
        "#### S9.1.3 Profile inheritance, overlays, and identity",
    ] {
        assert!(semantics.contains(heading), "missing {heading}");
    }
    for boundary in [
        "duplicate-coordinate",
        "ambiguous-surface",
        "punctuation-prefix",
        "recursive-alias",
        "core-override",
        "unrelated-syntax",
        "weaker-type-mode",
        "duplicate-overlay",
    ] {
        assert!(semantics.contains(boundary), "missing {boundary}");
    }

    let schema = fs::read_to_string(root().join("schemas/v0/bhcp-v0.cddl")).unwrap();
    for shape in [
        "keyword-syntax-mapping = {",
        "punctuation-syntax-mapping = {",
        "alias-syntax-mapping = {",
        "formatting-rules = {",
        "\"indent_width\": 0..16",
        "\"line_width\": 40..512",
        "\"final_newline\": bool",
    ] {
        assert!(schema.contains(shape), "missing {shape}");
    }
}
