use std::collections::BTreeMap;

use unicode_normalization::is_nfc;

use crate::diagnostic::{Diagnostic, Result};
use crate::model::{AstNode, ContentReference, Point, TokenSpan, is_symbol};
use crate::policy::{
    ArtifactReference, PolicyDocument, PolicyHeader, PolicyLayer, PolicyRule, SourcePolicyDocument,
    WaiverDocument, WaiverTarget, validate_policy_timestamp, validate_values_sorted_unique,
};
use crate::profile::{
    PresentationDocument, ProfileDocument, SyntaxDocument, SyntaxMapping, SyntaxMappingCategory,
};
use crate::value::Value;

#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    text: String,
    value: Option<TokenValue>,
    start: Point,
    end: Point,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TokenKind {
    Keyword,
    Identifier,
    Number,
    String,
    Bytes,
    Operator,
    Punctuation,
    Eof,
}

#[derive(Clone, Debug)]
enum TokenValue {
    Integer(i64),
    Text(String),
    Bytes(Vec<u8>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedToken {
    pub text: String,
    pub start: Point,
    pub end: Point,
}

const REGISTERED_KEYWORDS: &[&str] = &[
    "all",
    "allows",
    "any",
    "case",
    "chain",
    "compose",
    "ensures",
    "extension",
    "extends",
    "forbids",
    "function",
    "gate",
    "goal",
    "input",
    "invariant",
    "limit",
    "none",
    "output",
    "policy",
    "predicate",
    "prefer",
    "profile",
    "refines",
    "requires",
    "resource",
    "state",
    "syntax",
    "type",
    "use",
    "verify",
    "waiver",
];
const OPEN_DELIMITERS: &[&str] = &["(", "[", "{"];
const CLOSE_DELIMITERS: &[&str] = &[")", "]", "}"];
const FIXED_LEXICAL_TOKENS: &[&str] = &[
    "+",
    "-",
    "*",
    "/",
    "%",
    "!",
    "=",
    "<",
    ">",
    "<=",
    ">=",
    "==",
    "!=",
    "&&",
    "||",
    "=>",
    "::",
    "&",
    "|",
    "'",
    ":",
    ",",
    ".",
    "@",
    "\"",
    "//",
    "/*",
    "*/",
    "#!bhcp-profile",
];
const FIXED_BARE_WORDS: &[&str] = &[
    "Bool",
    "Bytes",
    "Decimal",
    "DerivedForm",
    "Duration",
    "Dynamic",
    "Goal",
    "Integer",
    "List",
    "Map",
    "Meta",
    "NetworkShape",
    "Never",
    "Option",
    "Rational",
    "Reduction",
    "Result",
    "Set",
    "Text",
    "Timestamp",
    "Unit",
    "add",
    "affine",
    "as",
    "borrow",
    "borrowed",
    "by",
    "capability",
    "classes",
    "completed",
    "decimal",
    "deny",
    "dimension",
    "dynamic",
    "effect",
    "else",
    "empirical",
    "evidence",
    "exists",
    "expect",
    "false",
    "faulted",
    "for",
    "formal",
    "forall",
    "goals",
    "gradual",
    "human-approved",
    "if",
    "in",
    "infer-strict",
    "integer",
    "layer",
    "linear",
    "maximum",
    "minimum",
    "model-judged",
    "move",
    "narrow",
    "nonwaivable",
    "obligation",
    "operations",
    "organization",
    "owned",
    "parameters",
    "prohibition",
    "rational",
    "read",
    "refuted",
    "repository",
    "requirement",
    "resources",
    "rule",
    "scope",
    "satisfied",
    "share",
    "shared",
    "static",
    "statistical",
    "strengthen",
    "strict",
    "team",
    "then",
    "tighten",
    "true",
    "unit",
    "unrestricted",
    "unresolved",
    "user",
    "using",
    "variant",
    "waivable",
    "when",
    "with",
    "where",
    "write",
];

#[derive(Clone, Debug)]
struct EffectiveSyntax {
    syntax_symbol: String,
    keyword_by_surface: BTreeMap<String, String>,
    keyword_surface_by_canonical: BTreeMap<String, String>,
    sigil_surface: String,
    punctuation_by_surface: Vec<(String, String)>,
    punctuation_surface_by_canonical: BTreeMap<String, String>,
    alias_by_surface: Vec<(String, String)>,
    alias_surface_by_canonical: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
struct MappedSource {
    text: String,
    origins: Vec<Point>,
}

impl EffectiveSyntax {
    fn from_document(document: &SyntaxDocument) -> Result<Self> {
        document.validate()?;
        if document.extends.is_some() {
            return Err(syntax_document_error(document, "unresolved-inheritance"));
        }

        let mut effective = BTreeMap::new();
        for keyword in REGISTERED_KEYWORDS {
            effective.insert(
                (SyntaxMappingCategory::Keyword, (*keyword).to_owned()),
                (*keyword).to_owned(),
            );
        }
        effective.insert(
            (SyntaxMappingCategory::Sigil, "§".to_owned()),
            "§".to_owned(),
        );
        for delimiter in OPEN_DELIMITERS {
            effective.insert(
                (
                    SyntaxMappingCategory::OpenDelimiter,
                    (*delimiter).to_owned(),
                ),
                (*delimiter).to_owned(),
            );
        }
        for delimiter in CLOSE_DELIMITERS {
            effective.insert(
                (
                    SyntaxMappingCategory::CloseDelimiter,
                    (*delimiter).to_owned(),
                ),
                (*delimiter).to_owned(),
            );
        }
        effective.insert(
            (SyntaxMappingCategory::Terminator, ";".to_owned()),
            ";".to_owned(),
        );

        for mapping in &document.mappings {
            validate_coordinate(mapping.category, &mapping.canonical)
                .map_err(|_| syntax_mapping_error(document, mapping, "category-mismatch"))?;
            validate_surface(mapping.category, &mapping.surface)
                .map_err(|_| syntax_mapping_error(document, mapping, "invalid-surface"))?;
            effective.insert(
                (mapping.category, mapping.canonical.clone()),
                mapping.surface.clone(),
            );
        }

        let mut surface_owner: BTreeMap<&str, (&str, SyntaxMappingCategory)> = BTreeMap::new();
        for ((category, canonical), surface) in &effective {
            if let Some((existing, existing_category)) =
                surface_owner.insert(surface, (canonical, *category))
                && existing != canonical
            {
                let Some(mapping) =
                    diagnostic_mapping(document, *category, canonical, existing_category, existing)
                else {
                    return Err(syntax_document_error(document, "ambiguous-surface"));
                };
                return Err(syntax_mapping_error(document, mapping, "ambiguous-surface"));
            }
        }

        let punctuation: Vec<_> = effective
            .iter()
            .filter(|((category, _), _)| is_punctuation(*category))
            .map(|((category, canonical), surface)| {
                (*category, canonical.as_str(), surface.as_str())
            })
            .collect();
        for (category, canonical, left) in &punctuation {
            for (right_category, right_canonical, right) in &punctuation {
                if left != right && right.starts_with(left) {
                    let Some(mapping) = diagnostic_mapping(
                        document,
                        *right_category,
                        right_canonical,
                        *category,
                        canonical,
                    ) else {
                        return Err(syntax_document_error(document, "punctuation-prefix"));
                    };
                    return Err(syntax_mapping_error(
                        document,
                        mapping,
                        "punctuation-prefix",
                    ));
                }
            }
            if FIXED_LEXICAL_TOKENS
                .iter()
                .any(|fixed| left.starts_with(fixed) || fixed.starts_with(left))
            {
                return Err(mapping_for(document, *category, canonical).map_or_else(
                    || syntax_document_error(document, "token-capture"),
                    |mapping| syntax_mapping_error(document, mapping, "token-capture"),
                ));
            }
        }

        let aliases: Vec<_> = effective
            .iter()
            .filter(|((category, _), _)| *category == SyntaxMappingCategory::Alias)
            .map(|((_, canonical), surface)| (canonical.as_str(), surface.as_str()))
            .collect();
        for (canonical, surface) in &aliases {
            let Some(mapping) = mapping_for(document, SyntaxMappingCategory::Alias, canonical)
            else {
                return Err(syntax_document_error(document, "invalid-effective-alias"));
            };
            if surface.starts_with("bhcp/") && canonical != surface {
                return Err(syntax_mapping_error(document, mapping, "core-override"));
            }
            if aliases
                .iter()
                .any(|(_, candidate_surface)| canonical == candidate_surface)
            {
                return Err(syntax_mapping_error(document, mapping, "recursive-alias"));
            }
            if FIXED_BARE_WORDS.contains(surface) {
                return Err(syntax_mapping_error(document, mapping, "token-capture"));
            }
        }

        let mut keyword_by_surface = BTreeMap::new();
        let mut keyword_surface_by_canonical = BTreeMap::new();
        let mut sigil_surface = String::new();
        let mut punctuation_by_surface = Vec::new();
        let mut punctuation_surface_by_canonical = BTreeMap::new();
        let mut alias_by_surface = Vec::new();
        let mut alias_surface_by_canonical = BTreeMap::new();
        for ((category, canonical), surface) in effective {
            match category {
                SyntaxMappingCategory::Keyword => {
                    keyword_by_surface.insert(surface.clone(), canonical.clone());
                    keyword_surface_by_canonical.insert(canonical, surface);
                }
                SyntaxMappingCategory::Sigil => sigil_surface = surface,
                SyntaxMappingCategory::OpenDelimiter
                | SyntaxMappingCategory::CloseDelimiter
                | SyntaxMappingCategory::Terminator => {
                    punctuation_by_surface.push((surface.clone(), canonical.clone()));
                    punctuation_surface_by_canonical.insert(canonical, surface);
                }
                SyntaxMappingCategory::Alias => {
                    alias_by_surface.push((surface.clone(), canonical.clone()));
                    alias_surface_by_canonical.insert(canonical, surface);
                }
            }
        }
        punctuation_by_surface.sort_by(|left, right| {
            right
                .0
                .len()
                .cmp(&left.0.len())
                .then_with(|| left.0.cmp(&right.0))
        });
        alias_by_surface.sort_by(|left, right| {
            right
                .0
                .len()
                .cmp(&left.0.len())
                .then_with(|| left.0.cmp(&right.0))
        });

        Ok(Self {
            syntax_symbol: document.symbol.clone(),
            keyword_by_surface,
            keyword_surface_by_canonical,
            sigil_surface,
            punctuation_by_surface,
            punctuation_surface_by_canonical,
            alias_by_surface,
            alias_surface_by_canonical,
        })
    }

    fn normalize(&self, source: &str, source_name: &str) -> Result<MappedSource> {
        let source_points = source_points(source);
        let mut mapped = MappedSource {
            text: String::with_capacity(source.len()),
            origins: vec![source_points[0].clone()],
        };
        let mut cursor = 0;
        while cursor < source.len() {
            if source[cursor..].starts_with("//") {
                let end = source[cursor..]
                    .find('\n')
                    .map_or(source.len(), |relative| cursor + relative + 1);
                append_original(&mut mapped, source, &source_points, cursor, end);
                cursor = end;
                continue;
            }
            if source[cursor..].starts_with("/*") {
                let end = source[cursor + 2..]
                    .find("*/")
                    .map_or(source.len(), |relative| cursor + 2 + relative + 2);
                append_original(&mut mapped, source, &source_points, cursor, end);
                cursor = end;
                continue;
            }
            if source[cursor..].starts_with('"') {
                let end = string_end(source, cursor);
                append_original(&mut mapped, source, &source_points, cursor, end);
                cursor = end;
                continue;
            }
            if let Some(end) = match_at(source, cursor, &self.sigil_surface) {
                append_replacement(&mut mapped, &source_points, cursor, end, "§");
                cursor = end;
                if let Some(keyword_end) = identifier_end(source, cursor) {
                    let surface = &source[cursor..keyword_end];
                    if let Some(canonical) = self.keyword_by_surface.get(surface) {
                        append_replacement(
                            &mut mapped,
                            &source_points,
                            cursor,
                            keyword_end,
                            canonical,
                        );
                        cursor = keyword_end;
                    } else if let Some(mapped_surface) =
                        self.keyword_surface_by_canonical.get(surface)
                    {
                        return Err(normalization_error(
                            self.mapped_away_reason(
                                SyntaxMappingCategory::Keyword,
                                surface,
                                mapped_surface,
                                "mapped-away-keyword",
                            ),
                            source_name,
                            &source_points[cursor],
                        ));
                    }
                }
                continue;
            }
            if source[cursor..].starts_with('§') && self.sigil_surface != "§" {
                return Err(normalization_error(
                    self.mapped_away_reason(
                        SyntaxMappingCategory::Sigil,
                        "§",
                        &self.sigil_surface,
                        "mapped-away-sigil",
                    ),
                    source_name,
                    &source_points[cursor],
                ));
            }
            if let Some((canonical, end)) =
                self.punctuation_by_surface
                    .iter()
                    .find_map(|(surface, canonical)| {
                        match_at(source, cursor, surface).map(|end| (canonical.as_str(), end))
                    })
            {
                append_replacement(&mut mapped, &source_points, cursor, end, canonical);
                cursor = end;
                continue;
            }
            if let Some(character) = source[cursor..].chars().next()
                && let Some(surface) = self
                    .punctuation_surface_by_canonical
                    .get(&character.to_string())
                && surface != &character.to_string()
            {
                return Err(normalization_error(
                    self.mapped_away_reason(
                        punctuation_category(character),
                        &character.to_string(),
                        surface,
                        "mapped-away-punctuation",
                    ),
                    source_name,
                    &source_points[cursor],
                ));
            }
            if let Some((_, canonical, end)) =
                self.alias_by_surface
                    .iter()
                    .find_map(|(surface, canonical)| {
                        token_match_at(source, cursor, surface)
                            .map(|end| (surface.as_str(), canonical.as_str(), end))
                    })
            {
                append_replacement(&mut mapped, &source_points, cursor, end, canonical);
                cursor = end;
                continue;
            }
            if let Some((canonical, surface)) =
                self.alias_surface_by_canonical
                    .iter()
                    .find(|(canonical, surface)| {
                        canonical.as_str() != surface.as_str()
                            && token_match_at(source, cursor, canonical).is_some()
                    })
            {
                return Err(normalization_error(
                    self.mapped_away_reason(
                        SyntaxMappingCategory::Alias,
                        canonical,
                        surface,
                        "mapped-away-alias",
                    ),
                    source_name,
                    &source_points[cursor],
                ));
            }
            let character = source[cursor..].chars().next().expect("cursor in source");
            let end = cursor + character.len_utf8();
            append_original(&mut mapped, source, &source_points, cursor, end);
            cursor = end;
        }
        Ok(mapped)
    }

    fn mapped_away_reason(
        &self,
        category: SyntaxMappingCategory,
        canonical: &str,
        surface: &str,
        rule: &str,
    ) -> String {
        format!(
            "syntax={} mapping={}:{}=>{} rule={rule}",
            self.syntax_symbol,
            category.as_str(),
            canonical,
            surface,
        )
    }
}

fn punctuation_category(character: char) -> SyntaxMappingCategory {
    match character {
        '{' | '(' | '[' => SyntaxMappingCategory::OpenDelimiter,
        '}' | ')' | ']' => SyntaxMappingCategory::CloseDelimiter,
        ';' => SyntaxMappingCategory::Terminator,
        _ => unreachable!("only mapped canonical punctuation reaches this boundary"),
    }
}

fn validate_coordinate(category: SyntaxMappingCategory, canonical: &str) -> Result<()> {
    let valid = match category {
        SyntaxMappingCategory::Keyword => REGISTERED_KEYWORDS.contains(&canonical),
        SyntaxMappingCategory::Sigil => canonical == "§",
        SyntaxMappingCategory::OpenDelimiter => OPEN_DELIMITERS.contains(&canonical),
        SyntaxMappingCategory::CloseDelimiter => CLOSE_DELIMITERS.contains(&canonical),
        SyntaxMappingCategory::Terminator => canonical == ";",
        SyntaxMappingCategory::Alias => is_symbol(canonical),
    };
    if valid {
        Ok(())
    } else {
        Err(syntax_error("category-mismatch"))
    }
}

fn validate_surface(category: SyntaxMappingCategory, surface: &str) -> Result<()> {
    if surface.is_empty()
        || !is_nfc(surface)
        || surface.contains("#!bhcp-profile")
        || surface.chars().any(char::is_control)
    {
        return Err(syntax_error("invalid-surface"));
    }
    let valid = match category {
        SyntaxMappingCategory::Keyword => identifier_token(surface),
        SyntaxMappingCategory::Sigil
        | SyntaxMappingCategory::OpenDelimiter
        | SyntaxMappingCategory::CloseDelimiter
        | SyntaxMappingCategory::Terminator => surface.chars().all(|character| {
            !character.is_alphanumeric() && !character.is_whitespace() && !character.is_control()
        }),
        SyntaxMappingCategory::Alias => identifier_token(surface) || is_symbol(surface),
    };
    if valid {
        Ok(())
    } else {
        Err(syntax_error("invalid-surface"))
    }
}

fn is_punctuation(category: SyntaxMappingCategory) -> bool {
    matches!(
        category,
        SyntaxMappingCategory::Sigil
            | SyntaxMappingCategory::OpenDelimiter
            | SyntaxMappingCategory::CloseDelimiter
            | SyntaxMappingCategory::Terminator
    )
}

fn mapping_for<'a>(
    document: &'a SyntaxDocument,
    category: SyntaxMappingCategory,
    canonical: &str,
) -> Option<&'a SyntaxMapping> {
    document
        .mappings
        .iter()
        .find(|mapping| mapping.category == category && mapping.canonical == canonical)
}

fn diagnostic_mapping<'a>(
    document: &'a SyntaxDocument,
    category: SyntaxMappingCategory,
    canonical: &str,
    related_category: SyntaxMappingCategory,
    related_canonical: &str,
) -> Option<&'a SyntaxMapping> {
    mapping_for(document, category, canonical)
        .or_else(|| mapping_for(document, related_category, related_canonical))
}

fn syntax_document_error(document: &SyntaxDocument, rule: &str) -> Diagnostic {
    Diagnostic::new(
        "BHCP9002",
        format!("syntax={} rule={rule}", document.symbol),
        &document.symbol,
        1,
        1,
    )
}

fn syntax_mapping_error(
    document: &SyntaxDocument,
    mapping: &SyntaxMapping,
    rule: &str,
) -> Diagnostic {
    let line = document
        .mappings
        .iter()
        .position(|candidate| std::ptr::eq(candidate, mapping))
        .map_or(1, |index| index + 1);
    Diagnostic::new(
        "BHCP9002",
        format!(
            "syntax={} mapping={}:{}=>{} rule={rule}",
            document.symbol,
            mapping.category.as_str(),
            mapping.canonical,
            mapping.surface,
        ),
        &document.symbol,
        line,
        1,
    )
}

fn identifier_token(value: &str) -> bool {
    let mut characters = value.chars();
    characters.next().is_some_and(unicode_identifier_start)
        && characters.all(unicode_identifier_continue)
}

fn unicode_identifier_start(character: char) -> bool {
    character == '_' || character.is_alphabetic()
}

fn unicode_identifier_continue(character: char) -> bool {
    unicode_identifier_start(character) || character.is_alphanumeric() || character == '-'
}

fn syntax_error(reason: impl Into<String>) -> Diagnostic {
    Diagnostic::new("BHCP9002", reason, "<effective-syntax>", 1, 1)
}

fn normalization_error(reason: impl Into<String>, source_name: &str, point: &Point) -> Diagnostic {
    Diagnostic::new("BHCP0005", reason, source_name, point.line, point.column)
}

fn source_points(source: &str) -> Vec<Point> {
    let mut points = vec![
        Point {
            byte: 0,
            line: 1,
            column: 1,
        };
        source.len() + 1
    ];
    let mut line = 1;
    let mut column = 1;
    for (byte, character) in source.char_indices() {
        let point = Point { byte, line, column };
        for slot in &mut points[byte..byte + character.len_utf8()] {
            *slot = point.clone();
        }
        if character == '\n' {
            line += 1;
            column = 1;
        } else if !(byte == 0 && character == '\u{feff}') {
            column += 1;
        }
        points[byte + character.len_utf8()] = Point {
            byte: byte + character.len_utf8(),
            line,
            column,
        };
    }
    points
}

fn append_original(
    mapped: &mut MappedSource,
    source: &str,
    source_points: &[Point],
    start: usize,
    end: usize,
) {
    mapped.text.push_str(&source[start..end]);
    mapped
        .origins
        .extend_from_slice(&source_points[start + 1..=end]);
}

fn append_replacement(
    mapped: &mut MappedSource,
    source_points: &[Point],
    start: usize,
    end: usize,
    replacement: &str,
) {
    mapped.text.push_str(replacement);
    for offset in 1..=replacement.len() {
        mapped.origins.push(if offset == replacement.len() {
            source_points[end].clone()
        } else {
            source_points[start].clone()
        });
    }
}

fn string_end(source: &str, start: usize) -> usize {
    let mut escaped = false;
    for (relative, character) in source[start + 1..].char_indices() {
        let end = start + 1 + relative + character.len_utf8();
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' || character == '\n' {
            return end;
        }
    }
    source.len()
}

fn match_at(source: &str, cursor: usize, candidate: &str) -> Option<usize> {
    source[cursor..]
        .starts_with(candidate)
        .then_some(cursor + candidate.len())
}

fn identifier_end(source: &str, cursor: usize) -> Option<usize> {
    let mut characters = source[cursor..].char_indices();
    let (_, first) = characters.next()?;
    if !unicode_identifier_start(first) {
        return None;
    }
    let mut end = cursor + first.len_utf8();
    for (relative, character) in characters {
        if !unicode_identifier_continue(character) {
            break;
        }
        end = cursor + relative + character.len_utf8();
    }
    Some(end)
}

fn token_match_at(source: &str, cursor: usize, candidate: &str) -> Option<usize> {
    let end = match_at(source, cursor, candidate)?;
    let before = source[..cursor].chars().next_back();
    let after = source[end..].chars().next();
    (!before.is_some_and(unicode_identifier_continue)
        && !after.is_some_and(unicode_identifier_continue))
    .then_some(end)
}

#[derive(Clone, Debug)]
pub enum SurfaceType {
    Primitive(&'static str),
    Exact(&'static str),
    Record(Vec<SurfaceFieldType>),
    Parameter(String),
    Dynamic,
    Reduction(Box<SurfaceType>),
    Meta {
        kind: &'static str,
        input: Box<SurfaceType>,
        output: Box<SurfaceType>,
    },
    Nominal {
        symbol: String,
        arguments: Vec<SurfaceType>,
    },
    Never,
    StructuralRecord {
        fields: Vec<SurfaceDefinitionField>,
        open: bool,
    },
    Tuple(Vec<SurfaceType>),
    List(Box<SurfaceType>),
    Set(Box<SurfaceType>),
    Map {
        key: Box<SurfaceType>,
        value: Box<SurfaceType>,
    },
    Option(Box<SurfaceType>),
    Result {
        ok: Box<SurfaceType>,
        error: Box<SurfaceType>,
    },
    Variant(Vec<SurfaceVariantCase>),
    Goal {
        input: Box<SurfaceType>,
        output: Box<SurfaceType>,
        effects: Option<SurfaceEffectRow>,
        evidence: Option<Box<SurfaceType>>,
    },
    Union(Vec<SurfaceType>),
    Intersection(Vec<SurfaceType>),
    Handle {
        ownership: String,
        access: Option<String>,
        usage: Option<String>,
        lifetime: Option<String>,
        value_type: Box<SurfaceType>,
    },
    Refined {
        value_type: Box<SurfaceType>,
        binder: String,
        predicate: Box<SurfaceExpression>,
    },
}

#[derive(Clone, Debug)]
pub struct SurfaceDefinitionField {
    pub name: String,
    pub optional: bool,
    pub value_type: SurfaceType,
}

#[derive(Clone, Debug)]
pub struct SurfaceVariantCase {
    pub name: String,
    pub payload: Vec<SurfaceType>,
}

#[derive(Clone, Debug)]
pub struct SurfaceEffectRow {
    pub effects: Vec<String>,
    pub tail: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SurfaceFieldType {
    pub name: String,
    pub value_type: SurfaceType,
}

#[derive(Clone, Debug)]
pub enum SurfaceExpression {
    Literal {
        value: SurfaceLiteral,
        at: Point,
    },
    Reference {
        name: String,
        at: Point,
    },
    Unary {
        operator: String,
        operand: Box<SurfaceExpression>,
        at: Point,
    },
    Binary {
        operator: String,
        left: Box<SurfaceExpression>,
        right: Box<SurfaceExpression>,
        at: Point,
    },
    Call {
        function: String,
        arguments: Vec<SurfaceExpression>,
        at: Point,
    },
    If {
        condition: Box<SurfaceExpression>,
        consequent: Box<SurfaceExpression>,
        alternative: Box<SurfaceExpression>,
        at: Point,
    },
}

impl SurfaceExpression {
    pub fn at(&self) -> &Point {
        match self {
            Self::Literal { at, .. }
            | Self::Reference { at, .. }
            | Self::Unary { at, .. }
            | Self::Binary { at, .. }
            | Self::Call { at, .. }
            | Self::If { at, .. } => at,
        }
    }
}

#[derive(Clone, Debug)]
pub enum SurfaceLiteral {
    Bool(bool),
    Integer(i64),
    Text(String),
}

#[derive(Clone, Debug)]
pub struct SurfaceEffect {
    pub symbol: String,
    pub arguments: Vec<SurfaceExpression>,
    pub at: Point,
}

#[derive(Clone, Debug)]
pub struct SurfaceClause {
    pub label: Option<String>,
    pub kind: SurfaceClauseKind,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Debug)]
pub enum SurfaceClauseKind {
    Fact {
        kind: &'static str,
        name: String,
        value_type: SurfaceType,
    },
    Contract {
        kind: &'static str,
        dimension: Option<String>,
        condition: SurfaceExpression,
    },
    Authority {
        kind: &'static str,
        effects: Vec<SurfaceEffect>,
    },
    Preference {
        priority: i64,
        objective: SurfaceExpression,
    },
    Verify {
        verifier: String,
        obligation_labels: Vec<String>,
    },
    SyntaxOnly {
        kind: String,
    },
}

#[derive(Clone, Debug)]
pub struct SurfaceUnsupported {
    pub message: String,
    pub at: Point,
}

#[derive(Clone, Debug)]
pub struct SurfaceGoal {
    pub symbol: String,
    pub clauses: Vec<SurfaceClause>,
    pub body: Option<SurfaceComposition>,
    pub unsupported: Option<SurfaceUnsupported>,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Debug)]
pub enum SurfaceComposition {
    DerivedAll {
        branches: Vec<SurfaceBranch>,
        at: Point,
    },
    DerivedAny {
        branches: Vec<SurfaceBranch>,
        at: Point,
    },
    DerivedNone {
        branches: Vec<SurfaceBranch>,
        at: Point,
    },
    DerivedChain {
        branches: Vec<SurfaceBranch>,
        at: Point,
    },
    DerivedGate {
        condition: SurfaceExpression,
        branches: Vec<SurfaceBranch>,
        at: Point,
    },
    Compose {
        reducer: String,
        branches: Vec<SurfaceBranch>,
        at: Point,
    },
    SyntaxOnly {
        branches: Vec<SurfaceBranch>,
        at: Point,
    },
}

impl SurfaceComposition {
    pub fn branches(&self) -> &[SurfaceBranch] {
        match self {
            Self::DerivedAll { branches, .. }
            | Self::DerivedAny { branches, .. }
            | Self::DerivedNone { branches, .. }
            | Self::DerivedChain { branches, .. }
            | Self::DerivedGate { branches, .. }
            | Self::Compose { branches, .. }
            | Self::SyntaxOnly { branches, .. } => branches,
        }
    }

    pub fn at(&self) -> &Point {
        match self {
            Self::DerivedAll { at, .. }
            | Self::DerivedAny { at, .. }
            | Self::DerivedNone { at, .. }
            | Self::DerivedChain { at, .. }
            | Self::DerivedGate { at, .. }
            | Self::Compose { at, .. }
            | Self::SyntaxOnly { at, .. } => at,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SurfaceBranch {
    pub tag: String,
    pub goal: String,
    pub arguments: Vec<SurfaceGoalArgument>,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceArgumentMode {
    Value,
    Move,
    Borrow,
    Share,
}

#[derive(Clone, Debug)]
pub struct SurfaceGoalArgument {
    pub name: String,
    pub mode: SurfaceArgumentMode,
    pub source: String,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Debug)]
pub struct SurfaceFunction {
    pub symbol: String,
    pub type_parameters: Vec<String>,
    pub type_parameter_bounds: Vec<Option<SurfaceType>>,
    pub parameters: Vec<SurfaceParameter>,
    pub result: SurfaceType,
    pub definition: SurfaceExpression,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Debug)]
pub struct SurfaceTypeDefinition {
    pub symbol: String,
    pub type_parameters: Vec<String>,
    pub type_parameter_bounds: Vec<Option<SurfaceType>>,
    pub definition: SurfaceType,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Debug)]
pub struct SurfaceVerifierBinding {
    pub symbol: String,
    pub arguments: Vec<SurfaceVerifierArgument>,
}

#[derive(Clone, Debug)]
pub struct SurfaceVerifierArgument {
    pub name: String,
    pub mode: SurfaceArgumentMode,
    pub value: SurfaceExpression,
}

#[derive(Clone, Debug)]
pub struct SurfacePredicate {
    pub symbol: String,
    pub type_parameters: Vec<String>,
    pub type_parameter_bounds: Vec<Option<SurfaceType>>,
    pub parameters: Vec<SurfaceParameter>,
    pub definition: Option<SurfaceExpression>,
    pub verifier: Option<SurfaceVerifierBinding>,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Debug)]
pub struct SurfaceRefinement {
    pub subtype: SurfaceType,
    pub supertype: SurfaceType,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Debug)]
pub struct SurfaceParameter {
    pub name: String,
    pub value_type: SurfaceType,
    pub at: Point,
}

#[derive(Clone, Debug)]
pub struct SurfacePolicy {
    pub document: SourcePolicyDocument,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Debug)]
pub struct SurfaceMetaField {
    pub name: String,
    pub label: Option<String>,
    pub value: Value,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Debug)]
pub struct SurfaceSyntax {
    pub document: SyntaxDocument,
    pub fields: Vec<SurfaceMetaField>,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Debug)]
pub struct SurfaceProfile {
    pub document: ProfileDocument,
    pub fields: Vec<SurfaceMetaField>,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Debug)]
pub struct SurfaceWaiver {
    pub symbol: String,
    pub fields: Vec<SurfaceMetaField>,
    pub document: Option<WaiverDocument>,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceExtensionKind {
    Derived,
    Native,
}

impl SurfaceExtensionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Derived => "derived",
            Self::Native => "native",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SurfaceExtension {
    pub symbol: String,
    pub extension_kind: SurfaceExtensionKind,
    pub fields: Vec<SurfaceMetaField>,
    pub descriptor: Option<Value>,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Debug)]
pub struct ParsedProgram {
    pub types: Vec<SurfaceTypeDefinition>,
    pub functions: Vec<SurfaceFunction>,
    pub predicates: Vec<SurfacePredicate>,
    pub refinements: Vec<SurfaceRefinement>,
    pub goals: Vec<SurfaceGoal>,
    pub policies: Vec<SurfacePolicy>,
    pub syntaxes: Vec<SurfaceSyntax>,
    pub profiles: Vec<SurfaceProfile>,
    pub waivers: Vec<SurfaceWaiver>,
    pub extensions: Vec<SurfaceExtension>,
    pub ast: AstNode,
}

pub const CANONICAL_PROFILE: &str = "bhcp/canonical@0";
const PROFILE_PREAMBLE: &[u8] = b"#!bhcp-profile";
const UTF8_BOM: &[u8] = &[0xef, 0xbb, 0xbf];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfilePreamble {
    pub profile: String,
    pub canonical_source: String,
    pub body_start: usize,
    pub had_preamble: bool,
}

pub fn scan_profile_preamble(source: &[u8], source_name: &str) -> Result<ProfilePreamble> {
    let decoded = std::str::from_utf8(source).map_err(|_| {
        preamble_error(
            "profile-selected source must be valid UTF-8",
            source_name,
            1,
            1,
        )
    })?;

    let bom_length = usize::from(source.starts_with(UTF8_BOM)) * UTF8_BOM.len();
    if let Some(relative) = source[bom_length..]
        .windows(UTF8_BOM.len())
        .position(|window| window == UTF8_BOM)
    {
        let offset = bom_length + relative;
        let (line, column) = source_point(decoded, offset);
        return Err(preamble_error(
            "the UTF-8 BOM may occur only once at byte zero",
            source_name,
            line,
            column,
        ));
    }

    let mut profile = CANONICAL_PROFILE.to_owned();
    let mut body_start = bom_length;
    let mut had_preamble = false;
    let remainder = &source[bom_length..];
    if remainder.starts_with(PROFILE_PREAMBLE) {
        had_preamble = true;
        let line_end = remainder
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or_else(|| {
                preamble_error("profile preamble must end with ASCII LF", source_name, 1, 1)
            })?;
        let line = &remainder[..line_end];
        if !line.is_ascii() || line.iter().any(|byte| byte.is_ascii_control()) {
            return Err(preamble_error(
                "profile preamble permits ASCII text and spaces followed by LF only",
                source_name,
                1,
                1,
            ));
        }
        let suffix = &line[PROFILE_PREAMBLE.len()..];
        let spaces = suffix.iter().take_while(|byte| **byte == b' ').count();
        if spaces == 0 {
            return Err(preamble_error(
                "profile preamble requires an ASCII space before the exact profile symbol",
                source_name,
                1,
                1,
            ));
        }
        let selected = &suffix[spaces..];
        if !valid_profile_symbol(selected) {
            return Err(preamble_error(
                "profile preamble requires one exact namespace/name@version symbol with no alias or trailing text",
                source_name,
                1,
                1,
            ));
        }
        profile = String::from_utf8(selected.to_vec()).expect("validated ASCII profile symbol");
        body_start = bom_length + line_end + 1;
    } else if remainder.starts_with(b"#!") {
        return Err(preamble_error(
            "unrecognized or aliased profile preamble directive",
            source_name,
            1,
            1,
        ));
    }

    if let Some((line, column, repeated_bom)) = misplaced_preamble(source, body_start) {
        let message = if repeated_bom {
            "the UTF-8 BOM may occur only once at byte zero"
        } else if had_preamble {
            "duplicate profile preamble is forbidden"
        } else {
            "profile preamble must be the first non-BOM bytes"
        };
        return Err(preamble_error(message, source_name, line, column));
    }

    let mut masked = source.to_vec();
    for byte in &mut masked[bom_length..body_start] {
        if *byte != b'\n' {
            *byte = b' ';
        }
    }
    let canonical_source = String::from_utf8(masked).expect("validated and ASCII-masked UTF-8");
    Ok(ProfilePreamble {
        profile,
        canonical_source,
        body_start,
        had_preamble,
    })
}

fn source_point(source: &str, byte_offset: usize) -> (usize, usize) {
    let prefix = &source[..byte_offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit_once('\n')
        .map_or(prefix, |(_, tail)| tail)
        .chars()
        .count()
        + 1;
    (line, column)
}

fn valid_profile_symbol(value: &[u8]) -> bool {
    let Some(at) = value.iter().position(|byte| *byte == b'@') else {
        return false;
    };
    if value[at + 1..].contains(&b'@') {
        return false;
    }
    let (path, version_with_at) = value.split_at(at);
    let version = &version_with_at[1..];
    let mut components = path.split(|byte| *byte == b'/');
    let Some(namespace) = components.next() else {
        return false;
    };
    let remaining: Vec<_> = components.collect();
    !namespace.is_empty()
        && !remaining.is_empty()
        && valid_symbol_component(namespace)
        && remaining
            .iter()
            .all(|component| valid_symbol_component(component))
        && valid_symbol_component(version)
}

fn valid_symbol_component(value: &[u8]) -> bool {
    !value.is_empty()
        && value
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'_' | b'-'))
}

fn misplaced_preamble(source: &[u8], scan_start: usize) -> Option<(usize, usize, bool)> {
    let mut line_start = scan_start;
    while line_start < source.len() {
        let line_end = source[line_start..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(source.len(), |offset| line_start + offset);
        let line = &source[line_start..line_end];
        let indentation = line.iter().take_while(|byte| **byte == b' ').count();
        let candidate = &line[indentation..];
        let repeated_bom = candidate.starts_with(UTF8_BOM);
        if repeated_bom || candidate.starts_with(b"#!") {
            let line_number = 1 + source[..line_start]
                .iter()
                .filter(|byte| **byte == b'\n')
                .count();
            return Some((line_number, indentation + 1, repeated_bom));
        }
        if line_end == source.len() {
            break;
        }
        line_start = line_end + 1;
    }
    None
}

fn preamble_error(
    message: impl Into<String>,
    source_name: &str,
    line: usize,
    column: usize,
) -> Diagnostic {
    Diagnostic::new("BHCP0003", message, source_name, line, column)
}

pub fn parse_canonical(
    source: &str,
    source_name: &str,
    source_ref: ContentReference,
) -> Result<ParsedProgram> {
    let tokens = lex(source, source_name)?;
    Parser {
        tokens,
        cursor: 0,
        source_name,
        source_ref,
        ast_counter: 0,
    }
    .program()
}

pub fn validate_effective_syntax(document: &SyntaxDocument) -> Result<()> {
    EffectiveSyntax::from_document(document).map(|_| ())
}

pub fn normalize_syntax_tokens(
    source: &str,
    source_name: &str,
    document: &SyntaxDocument,
) -> Result<Vec<NormalizedToken>> {
    let effective = EffectiveSyntax::from_document(document)?;
    let mapped = effective.normalize(source, source_name)?;
    Ok(
        lex_with_origins(&mapped.text, source_name, Some(&mapped.origins))?
            .into_iter()
            .filter(|token| token.kind != TokenKind::Eof)
            .map(|token| NormalizedToken {
                text: token.text,
                start: token.start,
                end: token.end,
            })
            .collect(),
    )
}

pub(crate) fn normalize_source_for_formatting(
    source: &str,
    source_name: &str,
    document: &SyntaxDocument,
) -> Result<String> {
    let effective = EffectiveSyntax::from_document(document)?;
    Ok(effective.normalize(source, source_name)?.text)
}

pub fn parse_with_syntax(
    source: &str,
    source_name: &str,
    source_ref: ContentReference,
    document: &SyntaxDocument,
) -> Result<ParsedProgram> {
    let effective = EffectiveSyntax::from_document(document)?;
    let mapped = effective.normalize(source, source_name)?;
    let tokens = lex_with_origins(&mapped.text, source_name, Some(&mapped.origins))?;
    Parser {
        tokens,
        cursor: 0,
        source_name,
        source_ref,
        ast_counter: 0,
    }
    .program()
}

fn lex(source: &str, source_name: &str) -> Result<Vec<Token>> {
    lex_with_origins(source, source_name, None)
}

fn lex_with_origins(
    source: &str,
    source_name: &str,
    origins: Option<&[Point]>,
) -> Result<Vec<Token>> {
    if let Some((byte, _)) = source.char_indices().find(|(byte, character)| {
        !character.is_ascii() && *character != '§' && !(*byte == 0 && *character == '\u{feff}')
    }) {
        let point = origins
            .and_then(|points| points.get(byte))
            .cloned()
            .unwrap_or(Point {
                byte,
                line: 1,
                column: 1,
            });
        return Err(Diagnostic::new(
            "BHCP0002",
            "dependency-free canonical source currently accepts ASCII plus the precomposed § sigil; unsupported Unicode is rejected",
            source_name,
            point.line,
            point.column,
        ));
    }
    let characters: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let (mut index, mut byte, mut line, mut column) = if characters.first() == Some(&'\u{feff}') {
        (1usize, UTF8_BOM.len(), 1usize, 1usize)
    } else {
        (0usize, 0usize, 1usize, 1usize)
    };
    let point = |byte, line, column| {
        origins
            .and_then(|points| points.get(byte))
            .cloned()
            .unwrap_or(Point { byte, line, column })
    };
    let advance =
        |index: &mut usize, byte: &mut usize, line: &mut usize, column: &mut usize| -> char {
            let character = characters[*index];
            *index += 1;
            *byte += character.len_utf8();
            if character == '\n' {
                *line += 1;
                *column = 1;
            } else {
                *column += 1;
            }
            character
        };
    while index < characters.len() {
        let current = characters[index];
        let next = characters.get(index + 1).copied().unwrap_or('\0');
        if current.is_ascii_whitespace() {
            advance(&mut index, &mut byte, &mut line, &mut column);
            continue;
        }
        if current == '/' && next == '/' {
            advance(&mut index, &mut byte, &mut line, &mut column);
            advance(&mut index, &mut byte, &mut line, &mut column);
            while index < characters.len() && characters[index] != '\n' {
                advance(&mut index, &mut byte, &mut line, &mut column);
            }
            continue;
        }
        if current == '/' && next == '*' {
            let start = point(byte, line, column);
            advance(&mut index, &mut byte, &mut line, &mut column);
            advance(&mut index, &mut byte, &mut line, &mut column);
            while index < characters.len()
                && !(characters[index] == '*' && characters.get(index + 1) == Some(&'/'))
            {
                advance(&mut index, &mut byte, &mut line, &mut column);
            }
            if index == characters.len() {
                return Err(at(
                    "BHCP0001",
                    "unterminated block comment",
                    source_name,
                    &start,
                ));
            }
            advance(&mut index, &mut byte, &mut line, &mut column);
            advance(&mut index, &mut byte, &mut line, &mut column);
            continue;
        }
        let start_index = index;
        let start = point(byte, line, column);
        if current == '§' {
            advance(&mut index, &mut byte, &mut line, &mut column);
            if index == characters.len() || !identifier_start(characters[index]) {
                return Err(at(
                    "BHCP0001",
                    "expected keyword after §",
                    source_name,
                    &start,
                ));
            }
            while index < characters.len() && identifier_continue(characters[index]) {
                advance(&mut index, &mut byte, &mut line, &mut column);
            }
            tokens.push(Token {
                kind: TokenKind::Keyword,
                text: characters[start_index..index].iter().collect(),
                value: None,
                start,
                end: point(byte, line, column),
            });
            continue;
        }
        if current == 'h' && next == '\'' {
            advance(&mut index, &mut byte, &mut line, &mut column);
            advance(&mut index, &mut byte, &mut line, &mut column);
            let hex_start = index;
            while index < characters.len() && characters[index] != '\'' {
                if !characters[index].is_ascii_hexdigit() {
                    return Err(at(
                        "BHCP0001",
                        "byte string literal must contain only hexadecimal digits",
                        source_name,
                        &start,
                    ));
                }
                advance(&mut index, &mut byte, &mut line, &mut column);
            }
            if index == characters.len() || (index - hex_start) % 2 != 0 {
                return Err(at(
                    "BHCP0001",
                    "byte string literal must be closed and contain pairs of hexadecimal digits",
                    source_name,
                    &start,
                ));
            }
            let hex: String = characters[hex_start..index].iter().collect();
            let value = (0..hex.len())
                .step_by(2)
                .map(|offset| {
                    u8::from_str_radix(&hex[offset..offset + 2], 16).expect("validated hex")
                })
                .collect();
            advance(&mut index, &mut byte, &mut line, &mut column);
            tokens.push(Token {
                kind: TokenKind::Bytes,
                text: characters[start_index..index].iter().collect(),
                value: Some(TokenValue::Bytes(value)),
                start,
                end: point(byte, line, column),
            });
            continue;
        }
        if identifier_start(current) {
            advance(&mut index, &mut byte, &mut line, &mut column);
            while index < characters.len() && identifier_continue(characters[index]) {
                advance(&mut index, &mut byte, &mut line, &mut column);
            }
            let text: String = characters[start_index..index].iter().collect();
            tokens.push(Token {
                kind: TokenKind::Identifier,
                value: Some(TokenValue::Text(text.clone())),
                text,
                start,
                end: point(byte, line, column),
            });
            continue;
        }
        if current.is_ascii_digit() {
            advance(&mut index, &mut byte, &mut line, &mut column);
            while index < characters.len() && characters[index].is_ascii_digit() {
                advance(&mut index, &mut byte, &mut line, &mut column);
            }
            let text: String = characters[start_index..index].iter().collect();
            let value = text.parse().map_err(|_| {
                at(
                    "BHCP0003",
                    "integer literal exceeds i64",
                    source_name,
                    &start,
                )
            })?;
            tokens.push(Token {
                kind: TokenKind::Number,
                value: Some(TokenValue::Integer(value)),
                text,
                start,
                end: point(byte, line, column),
            });
            continue;
        }
        if current == '"' {
            advance(&mut index, &mut byte, &mut line, &mut column);
            let mut decoded = String::new();
            let mut closed = false;
            while index < characters.len() {
                let character = advance(&mut index, &mut byte, &mut line, &mut column);
                match character {
                    '"' => {
                        closed = true;
                        break;
                    }
                    '\n' => {
                        return Err(at(
                            "BHCP0001",
                            "newline in string literal",
                            source_name,
                            &start,
                        ));
                    }
                    '\\' => {
                        if index == characters.len() {
                            break;
                        }
                        let escaped = advance(&mut index, &mut byte, &mut line, &mut column);
                        decoded.push(match escaped {
                            '"' => '"',
                            '\\' => '\\',
                            '/' => '/',
                            'b' => '\u{0008}',
                            'f' => '\u{000c}',
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            _ => {
                                return Err(at(
                                    "BHCP0001",
                                    "unsupported string escape",
                                    source_name,
                                    &start,
                                ));
                            }
                        });
                    }
                    character => decoded.push(character),
                }
            }
            if !closed {
                return Err(at(
                    "BHCP0001",
                    "unterminated string literal",
                    source_name,
                    &start,
                ));
            }
            let text: String = characters[start_index..index].iter().collect();
            tokens.push(Token {
                kind: TokenKind::String,
                text,
                value: Some(TokenValue::Text(decoded)),
                start,
                end: point(byte, line, column),
            });
            continue;
        }
        let pair = format!("{current}{next}");
        if matches!(
            pair.as_str(),
            "<=" | ">=" | "==" | "!=" | "&&" | "||" | "=>" | "::"
        ) {
            advance(&mut index, &mut byte, &mut line, &mut column);
            advance(&mut index, &mut byte, &mut line, &mut column);
            tokens.push(Token {
                kind: TokenKind::Operator,
                text: pair,
                value: None,
                start,
                end: point(byte, line, column),
            });
            continue;
        }
        if "+-*/%!=<>&|".contains(current) || "{}[]();:,.@?'".contains(current) {
            let kind = if "+-*/%!=<>&|".contains(current) {
                TokenKind::Operator
            } else {
                TokenKind::Punctuation
            };
            advance(&mut index, &mut byte, &mut line, &mut column);
            tokens.push(Token {
                kind,
                text: current.to_string(),
                value: None,
                start,
                end: point(byte, line, column),
            });
            continue;
        }
        return Err(at(
            "BHCP0001",
            format!("unexpected character {current:?}"),
            source_name,
            &start,
        ));
    }
    let end = point(byte, line, column);
    tokens.push(Token {
        kind: TokenKind::Eof,
        text: String::new(),
        value: None,
        start: end.clone(),
        end,
    });
    Ok(tokens)
}

struct Parser<'a> {
    tokens: Vec<Token>,
    cursor: usize,
    source_name: &'a str,
    source_ref: ContentReference,
    ast_counter: usize,
}

impl Parser<'_> {
    fn program(mut self) -> Result<ParsedProgram> {
        let mut types = Vec::new();
        let mut functions = Vec::new();
        let mut predicates = Vec::new();
        let mut refinements = Vec::new();
        let mut goals = Vec::new();
        let mut policies = Vec::new();
        let mut syntaxes = Vec::new();
        let mut profiles = Vec::new();
        let mut waivers = Vec::new();
        let mut extensions = Vec::new();
        let mut definitions = Vec::new();
        let mut symbols = BTreeMap::new();
        let mut refinement_edges = BTreeMap::new();
        while self.current().kind != TokenKind::Eof {
            let (symbol, at, ast) = match self.current().text.as_str() {
                "§type" => {
                    let definition = self.type_definition()?;
                    let result = (
                        Some(definition.symbol.clone()),
                        definition.at.clone(),
                        definition.ast.clone(),
                    );
                    types.push(definition);
                    result
                }
                "§function" => {
                    let function = self.function()?;
                    let result = (
                        Some(function.symbol.clone()),
                        function.at.clone(),
                        function.ast.clone(),
                    );
                    functions.push(function);
                    result
                }
                "§predicate" => {
                    let predicate = self.predicate()?;
                    let result = (
                        Some(predicate.symbol.clone()),
                        predicate.at.clone(),
                        predicate.ast.clone(),
                    );
                    predicates.push(predicate);
                    result
                }
                "§refines" => {
                    let refinement = self.refinement()?;
                    let edge = format!(
                        "{} -> {}",
                        type_name(&refinement.subtype),
                        type_name(&refinement.supertype)
                    );
                    if refinement_edges
                        .insert(edge.clone(), refinement.at.clone())
                        .is_some()
                    {
                        return Err(at(
                            "BHCP1003",
                            format!("duplicate refines edge {edge}"),
                            self.source_name,
                            &refinement.at,
                        ));
                    }
                    let result = (None, refinement.at.clone(), refinement.ast.clone());
                    refinements.push(refinement);
                    result
                }
                "§goal" => {
                    let goal = self.goal()?;
                    let result = (Some(goal.symbol.clone()), goal.at.clone(), goal.ast.clone());
                    goals.push(goal);
                    result
                }
                "§policy" => {
                    let policy = self.policy()?;
                    if policies.iter().any(|existing: &SurfacePolicy| {
                        existing.document.symbol == policy.document.symbol
                    }) {
                        return Err(at(
                            "BHCP1003",
                            format!("duplicate policy symbol {}", policy.document.symbol),
                            self.source_name,
                            &policy.at,
                        ));
                    }
                    let result = (
                        Some(policy.document.symbol.clone()),
                        policy.at.clone(),
                        policy.ast.clone(),
                    );
                    policies.push(policy);
                    result
                }
                "§syntax" => {
                    let syntax = self.syntax_definition()?;
                    let result = (
                        Some(syntax.document.symbol.clone()),
                        syntax.at.clone(),
                        syntax.ast.clone(),
                    );
                    syntaxes.push(syntax);
                    result
                }
                "§profile" => {
                    let profile = self.profile_definition()?;
                    let result = (
                        Some(profile.document.symbol.clone()),
                        profile.at.clone(),
                        profile.ast.clone(),
                    );
                    profiles.push(profile);
                    result
                }
                "§waiver" => {
                    let waiver = self.waiver_definition()?;
                    let result = (
                        Some(waiver.symbol.clone()),
                        waiver.at.clone(),
                        waiver.ast.clone(),
                    );
                    waivers.push(waiver);
                    result
                }
                "§extension" => {
                    let extension = self.extension_definition()?;
                    let result = (
                        Some(extension.symbol.clone()),
                        extension.at.clone(),
                        extension.ast.clone(),
                    );
                    extensions.push(extension);
                    result
                }
                _ => {
                    let code = if self.current().kind == TokenKind::Keyword {
                        "BHCP1004"
                    } else {
                        "BHCP1001"
                    };
                    return self.fail(
                        code,
                        format!(
                            "top-level syntax {:?} is outside the implemented vertical slice",
                            self.current().text
                        ),
                    );
                }
            };
            if let Some(symbol) = symbol
                && symbols.insert(symbol.clone(), at.clone()).is_some()
            {
                return Err(at_fn(
                    "BHCP1003",
                    format!("duplicate definition symbol {symbol}"),
                    self.source_name,
                    &at,
                ));
            }
            definitions.push(ast);
        }
        if definitions.is_empty() {
            return self.fail(
                "BHCP1001",
                "a canonical source file must contain at least one definition",
            );
        }
        let start = definitions[0].span.start.clone();
        let end = definitions.last().unwrap().span.end.clone();
        let ast = self.ast("program", None, start, end, vec![], definitions);
        Ok(ParsedProgram {
            types,
            functions,
            predicates,
            refinements,
            goals,
            policies,
            syntaxes,
            profiles,
            waivers,
            extensions,
            ast,
        })
    }

    fn type_definition(&mut self) -> Result<SurfaceTypeDefinition> {
        let keyword = self.expect("§type")?;
        let (symbol, _) = self.qualified_name()?;
        let (type_parameters, type_parameter_bounds) = self.type_parameters()?;
        self.expect("=")?;
        let definition = self.value_type(&type_parameters)?;
        let end = self.expect(";")?.end;
        let ast = self.ast(
            "type",
            Some("§type"),
            keyword.start.clone(),
            end,
            vec![
                ("symbol".to_owned(), Value::Text(symbol.clone())),
                (
                    "type_parameters".to_owned(),
                    type_parameters_value(&type_parameters, &type_parameter_bounds),
                ),
                ("definition".to_owned(), surface_type_value(&definition)),
            ],
            vec![],
        );
        Ok(SurfaceTypeDefinition {
            symbol,
            type_parameters,
            type_parameter_bounds,
            definition,
            at: keyword.start,
            ast,
        })
    }

    fn predicate(&mut self) -> Result<SurfacePredicate> {
        let keyword = self.expect("§predicate")?;
        let (symbol, _) = self.qualified_name()?;
        let (type_parameters, type_parameter_bounds) = self.type_parameters()?;
        let parameters = self.parameters(&type_parameters)?;
        self.expect(":")?;
        let result = self.identifier("predicate result type")?;
        if result.text != "Bool" {
            return Err(at(
                "BHCP1001",
                "predicate result must be Bool",
                self.source_name,
                &result.start,
            ));
        }
        let definition = if self.matches("=") {
            self.consume();
            Some(self.expression(0)?)
        } else {
            None
        };
        let verifier = if self.matches("with") {
            Some(self.verifier_binding()?)
        } else {
            None
        };
        let end = self.expect(";")?.end;
        let mut attributes = definition_attributes(
            &symbol,
            &type_parameters,
            &type_parameter_bounds,
            &parameters,
            Some(&SurfaceType::Primitive("Bool")),
            definition.as_ref(),
        );
        if let Some(binding) = &verifier {
            attributes.push(("verifier".to_owned(), Value::Text(binding.symbol.clone())));
            attributes.push((
                "verifier_arguments".to_owned(),
                Value::Array(
                    binding
                        .arguments
                        .iter()
                        .map(verifier_argument_value)
                        .collect(),
                ),
            ));
        }
        let ast = self.ast(
            "predicate",
            Some("§predicate"),
            keyword.start.clone(),
            end,
            attributes,
            vec![],
        );
        Ok(SurfacePredicate {
            symbol,
            type_parameters,
            type_parameter_bounds,
            parameters,
            definition,
            verifier,
            at: keyword.start,
            ast,
        })
    }

    fn refinement(&mut self) -> Result<SurfaceRefinement> {
        let keyword = self.expect("§refines")?;
        let (subtype_symbol, _) = self.qualified_name()?;
        let (supertype_symbol, _) = self.qualified_name()?;
        let end = self.expect(";")?.end;
        let subtype = SurfaceType::Nominal {
            symbol: subtype_symbol,
            arguments: vec![],
        };
        let supertype = SurfaceType::Nominal {
            symbol: supertype_symbol,
            arguments: vec![],
        };
        let ast = self.ast(
            "refines",
            Some("§refines"),
            keyword.start.clone(),
            end,
            vec![
                ("subtype".to_owned(), surface_type_value(&subtype)),
                ("supertype".to_owned(), surface_type_value(&supertype)),
            ],
            vec![],
        );
        Ok(SurfaceRefinement {
            subtype,
            supertype,
            at: keyword.start,
            ast,
        })
    }

    fn type_parameters(&mut self) -> Result<(Vec<String>, Vec<Option<SurfaceType>>)> {
        let mut names = Vec::new();
        let mut bounds = Vec::new();
        if !self.matches("<") {
            return Ok((names, bounds));
        }
        self.consume();
        loop {
            let parameter = self.binder("type parameter")?;
            if names.contains(&parameter.text) {
                return Err(at(
                    "BHCP1003",
                    "duplicate type parameter",
                    self.source_name,
                    &parameter.start,
                ));
            }
            names.push(parameter.text);
            let bound = if self.matches(":") {
                self.consume();
                Some(self.value_type(&names)?)
            } else {
                None
            };
            bounds.push(bound);
            if !self.matches(",") {
                break;
            }
            self.consume();
        }
        self.expect(">")?;
        Ok((names, bounds))
    }

    fn parameters(&mut self, type_parameters: &[String]) -> Result<Vec<SurfaceParameter>> {
        self.expect("(")?;
        let mut parameters = Vec::new();
        if !self.matches(")") {
            loop {
                let name = self.binder("parameter name")?;
                if parameters
                    .iter()
                    .any(|existing: &SurfaceParameter| existing.name == name.text)
                {
                    return Err(at(
                        "BHCP1003",
                        "duplicate parameter",
                        self.source_name,
                        &name.start,
                    ));
                }
                self.expect(":")?;
                let value_type = self.value_type(type_parameters)?;
                parameters.push(SurfaceParameter {
                    name: name.text,
                    value_type,
                    at: name.start,
                });
                if !self.matches(",") {
                    break;
                }
                self.consume();
            }
        }
        self.expect(")")?;
        Ok(parameters)
    }

    fn verifier_binding(&mut self) -> Result<SurfaceVerifierBinding> {
        self.expect("with")?;
        let (symbol, _) = self.qualified_name()?;
        let mut arguments = Vec::new();
        if self.matches("(") {
            self.consume();
            if !self.matches(")") {
                loop {
                    let name = self.binder("verifier argument name")?;
                    if arguments
                        .iter()
                        .any(|argument: &SurfaceVerifierArgument| argument.name == name.text)
                    {
                        return Err(at(
                            "BHCP1003",
                            "duplicate verifier argument",
                            self.source_name,
                            &name.start,
                        ));
                    }
                    self.expect("=")?;
                    let mode = if self.matches("move") {
                        self.consume();
                        SurfaceArgumentMode::Move
                    } else if self.matches("borrow") {
                        self.consume();
                        SurfaceArgumentMode::Borrow
                    } else if self.matches("share") {
                        self.consume();
                        SurfaceArgumentMode::Share
                    } else {
                        SurfaceArgumentMode::Value
                    };
                    arguments.push(SurfaceVerifierArgument {
                        name: name.text,
                        mode,
                        value: self.expression(0)?,
                    });
                    if !self.matches(",") {
                        break;
                    }
                    self.consume();
                }
            }
            self.expect(")")?;
        }
        Ok(SurfaceVerifierBinding { symbol, arguments })
    }

    fn function(&mut self) -> Result<SurfaceFunction> {
        let keyword = self.expect("§function")?;
        let (symbol, _) = self.qualified_name()?;
        let (type_parameters, type_parameter_bounds) = self.type_parameters()?;
        let parameters = self.parameters(&type_parameters)?;
        self.expect(":")?;
        let result = self.value_type(&type_parameters)?;
        self.expect("=")?;
        let definition = self.expression(0)?;
        let end = self.expect(";")?.end;
        let ast = self.ast(
            "function",
            Some("§function"),
            keyword.start.clone(),
            end,
            definition_attributes(
                &symbol,
                &type_parameters,
                &type_parameter_bounds,
                &parameters,
                Some(&result),
                Some(&definition),
            ),
            vec![],
        );
        Ok(SurfaceFunction {
            symbol,
            type_parameters,
            type_parameter_bounds,
            parameters,
            result,
            definition,
            at: keyword.start,
            ast,
        })
    }

    fn policy(&mut self) -> Result<SurfacePolicy> {
        let keyword = self.expect("§policy")?;
        let (symbol, _) = self.qualified_name()?;
        let extends = if self.matches("§extends") {
            self.consume();
            Some(self.qualified_name()?.0)
        } else {
            None
        };
        self.expect("{")?;
        let mut layer = None;
        let mut rules = Vec::new();
        let mut children = Vec::new();
        while !self.matches("}") {
            if self.current().kind == TokenKind::Eof {
                return Err(at(
                    "BHCP1001",
                    "unterminated policy block",
                    self.source_name,
                    &keyword.start,
                ));
            }
            if self.matches("layer") {
                if layer.is_some() {
                    return self.fail("BHCP1003", "duplicate policy layer clause");
                }
                self.consume();
                let value = self.identifier("policy layer")?;
                layer = Some(match value.text.as_str() {
                    "organization" => PolicyLayer::Organization,
                    "team" => PolicyLayer::Team,
                    "repository" => PolicyLayer::Repository,
                    "user" => PolicyLayer::User,
                    _ => {
                        return Err(at(
                            "BHCP1001",
                            "policy layer must be organization, team, repository, or user",
                            self.source_name,
                            &value.start,
                        ));
                    }
                });
                self.expect(";")?;
            } else if self.matches("rule") {
                if layer.is_none() {
                    return self.fail("BHCP1001", "policy layer must be declared before rules");
                }
                let (rule, ast) = self.policy_rule()?;
                if rules
                    .last()
                    .is_some_and(|previous: &PolicyRule| previous.id() >= rule.id())
                {
                    return Err(at(
                        "BHCP8001",
                        "source policy rules must be sorted by unique rule ID",
                        self.source_name,
                        &ast.span.start,
                    ));
                }
                rules.push(rule);
                children.push(ast);
            } else {
                let clause = self.current().clone();
                return Err(at(
                    "BHCP1004",
                    format!(
                        "policy clause {:?} is outside the implemented policy slice",
                        clause.text
                    ),
                    self.source_name,
                    &clause.start,
                ));
            }
        }
        let end = self.consume().end;
        let Some(layer) = layer else {
            return Err(at(
                "BHCP1001",
                "source policy requires exactly one layer clause",
                self.source_name,
                &keyword.start,
            ));
        };
        let document = SourcePolicyDocument {
            header: PolicyHeader {
                features: vec![],
                semantic_id: None,
                artifact_id: None,
                provenance: None,
                authorization: None,
            },
            symbol: symbol.clone(),
            layer,
            extends: extends.clone(),
            rules,
        };
        PolicyDocument::Source(document.clone())
            .validate()
            .map_err(|diagnostic| {
                at(
                    diagnostic.code,
                    diagnostic.message,
                    self.source_name,
                    &keyword.start,
                )
            })?;
        let mut attributes = vec![
            ("symbol".to_owned(), Value::Text(symbol)),
            (
                "layer".to_owned(),
                Value::Text(policy_layer_name(layer).to_owned()),
            ),
        ];
        if let Some(extends) = extends {
            attributes.push(("extends".to_owned(), Value::Text(extends)));
        }
        let ast = self.ast(
            "policy",
            Some("§policy"),
            keyword.start.clone(),
            end,
            attributes,
            children,
        );
        Ok(SurfacePolicy {
            document,
            at: keyword.start,
            ast,
        })
    }

    fn policy_rule(&mut self) -> Result<(PolicyRule, AstNode)> {
        let keyword = self.expect("rule")?;
        let id = self.identifier("policy rule ID")?;
        let label = if self.current().kind == TokenKind::String {
            let token = self.consume();
            let Some(TokenValue::Text(label)) = token.value else {
                unreachable!()
            };
            Some(label)
        } else {
            None
        };
        self.expect(":")?;
        let category = self.identifier("policy category")?;
        let operation = self.identifier("policy operation")?;
        let value = self.meta_value()?;
        if !matches!(self.current().kind, TokenKind::Identifier) {
            return self.fail(
                "BHCP1004",
                "expression-valued policy clauses are outside canonical v0",
            );
        }
        let waiver = self.identifier("policy waivability")?;
        let (waivable, authorized_issuers) = match waiver.text.as_str() {
            "nonwaivable" => (false, Vec::new()),
            "waivable" => {
                self.expect("by")?;
                self.expect("[")?;
                let mut issuers = Vec::new();
                if !self.matches("]") {
                    loop {
                        let issuer_token = self.consume();
                        let Some(TokenValue::Text(issuer)) = issuer_token.value else {
                            return Err(at(
                                "BHCP1001",
                                "authorized issuer must be a string",
                                self.source_name,
                                &issuer_token.start,
                            ));
                        };
                        if issuer_token.kind != TokenKind::String {
                            return Err(at(
                                "BHCP1001",
                                "authorized issuer must be a string",
                                self.source_name,
                                &issuer_token.start,
                            ));
                        }
                        issuers.push(issuer);
                        if !self.matches(",") {
                            break;
                        }
                        self.consume();
                    }
                }
                self.expect("]")?;
                (true, issuers)
            }
            _ => {
                return Err(at(
                    "BHCP1001",
                    "policy waivability must be nonwaivable or waivable by a non-empty issuer list",
                    self.source_name,
                    &waiver.start,
                ));
            }
        };
        let end = self.expect(";")?.end;
        let mut entries = vec![
            ("id".to_owned(), Value::Text(id.text.clone())),
            ("category".to_owned(), Value::Text(category.text.clone())),
            ("operation".to_owned(), Value::Text(operation.text.clone())),
            ("value".to_owned(), value.clone()),
            ("waivable".to_owned(), Value::Bool(waivable)),
        ];
        if waivable {
            entries.push((
                "authorized_issuers".to_owned(),
                Value::Array(
                    authorized_issuers
                        .iter()
                        .cloned()
                        .map(Value::Text)
                        .collect(),
                ),
            ));
        }
        let rule = PolicyRule::from_value(&Value::owned_map(entries)).map_err(|diagnostic| {
            at(
                diagnostic.code,
                diagnostic.message,
                self.source_name,
                &keyword.start,
            )
        })?;
        let mut attributes = vec![
            ("id".to_owned(), Value::Text(id.text)),
            ("category".to_owned(), Value::Text(category.text)),
            ("operation".to_owned(), Value::Text(operation.text)),
            ("value".to_owned(), value),
            ("waivable".to_owned(), Value::Bool(waivable)),
        ];
        if let Some(label) = label {
            attributes.push(("label".to_owned(), Value::Text(label)));
        }
        if waivable {
            attributes.push((
                "authorized_issuers".to_owned(),
                Value::Array(authorized_issuers.into_iter().map(Value::Text).collect()),
            ));
        }
        let ast = self.ast("policy-rule", None, keyword.start, end, attributes, vec![]);
        Ok((rule, ast))
    }

    fn syntax_definition(&mut self) -> Result<SurfaceSyntax> {
        let keyword = self.expect("§syntax")?;
        let (symbol, _) = self.qualified_name()?;
        let (fields, end) = self.closed_meta_fields(
            "syntax",
            &[("preamble", true), ("mappings", true), ("formatting", true)],
        )?;
        let value = governance_document_value("syntax", &symbol, None, &fields);
        let document = match PresentationDocument::from_value(&value).map_err(|diagnostic| {
            at(
                "BHCP1001",
                format!("invalid syntax definition: {}", diagnostic.message),
                self.source_name,
                &keyword.start,
            )
        })? {
            PresentationDocument::Syntax(document) => document,
            PresentationDocument::Profile(_) => unreachable!(),
        };
        let ast = self.governance_ast(
            "syntax",
            "§syntax",
            &symbol,
            None,
            None,
            keyword.start.clone(),
            end,
            &fields,
        );
        Ok(SurfaceSyntax {
            document,
            fields,
            at: keyword.start,
            ast,
        })
    }

    fn profile_definition(&mut self) -> Result<SurfaceProfile> {
        let keyword = self.expect("§profile")?;
        let (symbol, _) = self.qualified_name()?;
        let extends = if self.matches("§extends") {
            self.consume();
            Some(self.qualified_name()?.0)
        } else {
            None
        };
        let (fields, end) = self.closed_meta_fields(
            "profile",
            &[
                ("syntax", true),
                ("type_mode", true),
                ("policy_overlays", true),
            ],
        )?;
        let value = governance_document_value("profile", &symbol, extends.as_deref(), &fields);
        let document = match PresentationDocument::from_value(&value).map_err(|diagnostic| {
            at(
                "BHCP1001",
                format!("invalid profile definition: {}", diagnostic.message),
                self.source_name,
                &keyword.start,
            )
        })? {
            PresentationDocument::Profile(document) => document,
            PresentationDocument::Syntax(_) => unreachable!(),
        };
        let ast = self.governance_ast(
            "profile",
            "§profile",
            &symbol,
            extends.as_deref(),
            None,
            keyword.start.clone(),
            end,
            &fields,
        );
        Ok(SurfaceProfile {
            document,
            fields,
            at: keyword.start,
            ast,
        })
    }

    fn waiver_definition(&mut self) -> Result<SurfaceWaiver> {
        let keyword = self.expect("§waiver")?;
        let (symbol, _) = self.qualified_name()?;
        let (fields, end) = self.closed_meta_fields(
            "waiver",
            &[
                ("issuer", true),
                ("targets", true),
                ("justification", true),
                ("authority_chain", false),
                ("issued_at", true),
                ("not_before", true),
                ("expires_at", true),
                ("authorization", true),
                ("audit_reference", true),
            ],
        )?;
        let document = validate_waiver_source(&symbol, &fields)
            .map_err(|message| at("BHCP1001", message, self.source_name, &keyword.start))?;
        let ast = self.governance_ast(
            "waiver",
            "§waiver",
            &symbol,
            None,
            None,
            keyword.start.clone(),
            end,
            &fields,
        );
        Ok(SurfaceWaiver {
            symbol,
            fields,
            document,
            at: keyword.start,
            ast,
        })
    }

    fn extension_definition(&mut self) -> Result<SurfaceExtension> {
        let keyword = self.expect("§extension")?;
        let (symbol, _) = self.qualified_name()?;
        let extension_kind_token = self.identifier("extension kind")?;
        let extension_kind = match extension_kind_token.text.as_str() {
            "derived" => SurfaceExtensionKind::Derived,
            "native" => SurfaceExtensionKind::Native,
            _ => {
                return Err(at(
                    "BHCP1001",
                    "extension kind must be derived or native",
                    self.source_name,
                    &extension_kind_token.start,
                ));
            }
        };
        let specifications = match extension_kind {
            SurfaceExtensionKind::Derived => &[
                ("lowering", true),
                ("input", false),
                ("output", false),
                ("children", false),
                ("type_rule", false),
                ("effect_rule", false),
                ("policy_rule", false),
                ("normalization_rule", false),
                ("evidence_rule", false),
            ][..],
            SurfaceExtensionKind::Native => &[
                ("payload_schema", true),
                ("type_rule", true),
                ("effect_rule", true),
                ("policy_rule", true),
                ("normalization_rule", true),
                ("evidence_rule", true),
            ][..],
        };
        let context = format!("{} extension", extension_kind.as_str());
        let (fields, end) = self.closed_meta_fields(&context, specifications)?;
        let descriptor = validate_extension_source(&symbol, extension_kind, &fields)
            .map_err(|message| at("BHCP1001", message, self.source_name, &keyword.start))?;
        let ast = self.governance_ast(
            "extension",
            "§extension",
            &symbol,
            None,
            Some(extension_kind),
            keyword.start.clone(),
            end,
            &fields,
        );
        Ok(SurfaceExtension {
            symbol,
            extension_kind,
            fields,
            descriptor,
            at: keyword.start,
            ast,
        })
    }

    fn closed_meta_fields(
        &mut self,
        context: &str,
        specifications: &[(&str, bool)],
    ) -> Result<(Vec<SurfaceMetaField>, Point)> {
        self.expect("{")?;
        let mut fields = Vec::new();
        let mut previous_index = None;
        while !self.matches("}") {
            if self.current().kind == TokenKind::Eof {
                return self.fail("BHCP1001", format!("unterminated {context} block"));
            }
            let name = self.identifier(&format!("{context} field"))?;
            let Some(index) = specifications
                .iter()
                .position(|(candidate, _)| *candidate == name.text)
            else {
                return Err(at(
                    "BHCP1004",
                    format!("unknown {context} field {:?}", name.text),
                    self.source_name,
                    &name.start,
                ));
            };
            if fields
                .iter()
                .any(|field: &SurfaceMetaField| field.name == name.text)
            {
                return Err(at(
                    "BHCP1003",
                    format!("duplicate {context} field {:?}", name.text),
                    self.source_name,
                    &name.start,
                ));
            }
            if previous_index.is_some_and(|previous| index < previous) {
                return Err(at(
                    "BHCP1003",
                    format!("{context} field order is closed and canonical"),
                    self.source_name,
                    &name.start,
                ));
            }
            previous_index = Some(index);
            let label = if self.current().kind == TokenKind::String && self.peek().text == ":" {
                self.label()?
            } else {
                None
            };
            let value = self.meta_value_with_order(true)?;
            let end = self.expect(";")?.end;
            let mut attributes = vec![
                ("name".to_owned(), Value::Text(name.text.clone())),
                ("value".to_owned(), value.clone()),
            ];
            if let Some(label) = &label {
                attributes.push(("label".to_owned(), Value::Text(label.clone())));
            }
            let ast = self.ast(
                "meta-field",
                None,
                name.start.clone(),
                end,
                attributes,
                vec![],
            );
            fields.push(SurfaceMetaField {
                name: name.text,
                label,
                value,
                at: name.start,
                ast,
            });
        }
        let end = self.consume().end;
        for (name, required) in specifications {
            if *required && !fields.iter().any(|field| field.name == *name) {
                return Err(at(
                    "BHCP1001",
                    format!("{context} definition requires field {name:?}"),
                    self.source_name,
                    &end,
                ));
            }
        }
        Ok((fields, end))
    }

    #[allow(clippy::too_many_arguments)]
    fn governance_ast(
        &mut self,
        kind: &str,
        token: &str,
        symbol: &str,
        extends: Option<&str>,
        extension_kind: Option<SurfaceExtensionKind>,
        start: Point,
        end: Point,
        fields: &[SurfaceMetaField],
    ) -> AstNode {
        let mut attributes = vec![("symbol".to_owned(), Value::Text(symbol.to_owned()))];
        if let Some(extends) = extends {
            attributes.push(("extends".to_owned(), Value::Text(extends.to_owned())));
        }
        if let Some(extension_kind) = extension_kind {
            attributes.push((
                "extension_kind".to_owned(),
                Value::Text(extension_kind.as_str().to_owned()),
            ));
            attributes.push((
                "must_understand".to_owned(),
                Value::Bool(extension_kind == SurfaceExtensionKind::Native),
            ));
        }
        attributes.extend(
            fields
                .iter()
                .map(|field| (field.name.clone(), field.value.clone())),
        );
        self.ast(
            kind,
            Some(token),
            start,
            end,
            attributes,
            fields.iter().map(|field| field.ast.clone()).collect(),
        )
    }

    fn meta_value(&mut self) -> Result<Value> {
        self.meta_value_with_order(false)
    }

    fn meta_value_with_order(&mut self, enforce_record_order: bool) -> Result<Value> {
        if self.matches("time") {
            let at = self.consume().start;
            let value = self.consume();
            let Some(TokenValue::Text(value)) = value.value else {
                return Err(at_fn(
                    "BHCP1001",
                    "time meta-value requires a string literal",
                    self.source_name,
                    &at,
                ));
            };
            return Ok(Value::Tag(0, Box::new(Value::Text(value))));
        }
        if self.matches("-") {
            let start = self.consume().start;
            let number = self.consume();
            let Some(TokenValue::Integer(value)) = number.value else {
                return Err(at(
                    "BHCP1001",
                    "expected integer after minus in policy value",
                    self.source_name,
                    &start,
                ));
            };
            return Ok(Value::Integer(-value));
        }
        if self.matches("[") {
            self.consume();
            let mut values = Vec::new();
            if !self.matches("]") {
                loop {
                    values.push(self.meta_value_with_order(enforce_record_order)?);
                    if !self.matches(",") {
                        break;
                    }
                    self.consume();
                }
            }
            self.expect("]")?;
            return Ok(Value::Array(values));
        }
        if self.matches("{") {
            let start = self.consume().start;
            let mut entries = Vec::new();
            if !self.matches("}") {
                loop {
                    let key = self.identifier("policy map key")?;
                    if entries
                        .iter()
                        .any(|(existing, _): &(String, Value)| existing == &key.text)
                    {
                        return Err(at(
                            "BHCP1003",
                            format!("duplicate policy map key {:?}", key.text),
                            self.source_name,
                            &key.start,
                        ));
                    }
                    self.expect(":")?;
                    let enforce_value_order = enforce_record_order && key.text != "parameters";
                    entries.push((key.text, self.meta_value_with_order(enforce_value_order)?));
                    if !self.matches(",") {
                        break;
                    }
                    self.consume();
                }
            }
            self.expect("}")?;
            if enforce_record_order {
                validate_meta_map_order(&entries)
                    .map_err(|message| at("BHCP1003", message, self.source_name, &start))?;
            }
            return Ok(Value::owned_map(entries));
        }
        let token = self.current().clone();
        match token.kind {
            TokenKind::String => {
                self.consume();
                let Some(TokenValue::Text(value)) = token.value else {
                    unreachable!()
                };
                Ok(Value::Text(value))
            }
            TokenKind::Bytes => {
                self.consume();
                let Some(TokenValue::Bytes(value)) = token.value else {
                    unreachable!()
                };
                Ok(Value::Bytes(value))
            }
            TokenKind::Number => {
                self.consume();
                let Some(TokenValue::Integer(value)) = token.value else {
                    unreachable!()
                };
                Ok(Value::Integer(value))
            }
            TokenKind::Identifier if token.text == "true" || token.text == "false" => {
                self.consume();
                Ok(Value::Bool(token.text == "true"))
            }
            TokenKind::Identifier if self.starts_qualified_name() => {
                Ok(Value::Text(self.qualified_name()?.0))
            }
            TokenKind::Identifier => {
                self.consume();
                Ok(Value::Text(token.text))
            }
            TokenKind::Keyword => Err(at(
                "BHCP1004",
                format!(
                    "policy value syntax {:?} is outside the implemented policy slice",
                    token.text
                ),
                self.source_name,
                &token.start,
            )),
            _ => Err(at(
                "BHCP1001",
                format!("expected policy value, found {:?}", token.text),
                self.source_name,
                &token.start,
            )),
        }
    }

    fn starts_qualified_name(&self) -> bool {
        if self.current().kind != TokenKind::Identifier {
            return false;
        }
        let mut cursor = self.cursor + 1;
        let mut segments = 1usize;
        while self
            .tokens
            .get(cursor)
            .is_some_and(|token| token.text == "/" || token.text == "." || token.text == "::")
            && self
                .tokens
                .get(cursor + 1)
                .is_some_and(|token| token.kind == TokenKind::Identifier)
        {
            if self.tokens[cursor].text == "/" || self.tokens[cursor].text == "::" {
                segments += 1;
            }
            cursor += 2;
        }
        segments >= 2
            && self
                .tokens
                .get(cursor)
                .is_some_and(|token| token.text == "@")
    }

    fn goal(&mut self) -> Result<SurfaceGoal> {
        let keyword = self.expect("§goal")?;
        let (symbol, _) = self.qualified_name()?;
        let (type_parameters, type_parameter_bounds) = self.type_parameters()?;
        let refines = if self.matches("§refines") {
            self.consume();
            Some(self.value_type(&type_parameters)?)
        } else {
            None
        };
        let structured_fact_types = !type_parameters.is_empty() || refines.is_some();
        self.expect("{")?;
        let mut clauses = Vec::new();
        let mut body = None;
        let mut compositions = Vec::new();
        let mut children = Vec::new();
        let mut bindings = Vec::new();
        let mut syntax_bindings = Vec::new();
        let mut labels = Vec::new();
        let mut validate_goal_uniqueness = structured_fact_types;
        let mut unsupported =
            (!type_parameters.is_empty() || refines.is_some()).then(|| SurfaceUnsupported {
                message: "goal syntax is outside the implemented executable slice".to_owned(),
                at: keyword.start.clone(),
            });
        while !self.matches("}") {
            if self.current().kind == TokenKind::Eof {
                return Err(at(
                    "BHCP1001",
                    "unterminated goal block",
                    self.source_name,
                    &keyword.start,
                ));
            }
            if self.matches("§all")
                || self.matches("§any")
                || self.matches("§none")
                || self.matches("§chain")
                || self.matches("§gate")
                || self.matches("§compose")
            {
                if self.extended_composition_follows() {
                    let (composition, ast) = self.syntax_composition(true)?;
                    validate_goal_uniqueness = true;
                    self.ensure_unique_goal_items(&bindings, &labels, composition.at())?;
                    unsupported.get_or_insert(SurfaceUnsupported {
                        message: "nested composition is outside the implemented vertical slice"
                            .to_owned(),
                        at: composition.at().clone(),
                    });
                    children.push(ast);
                    if body.is_none() {
                        body = Some(composition.clone());
                    }
                    compositions.push(composition);
                    continue;
                }
                let (composition, ast) = self.composition()?;
                if body.is_some() {
                    validate_goal_uniqueness = true;
                    self.ensure_unique_goal_items(&bindings, &labels, composition.at())?;
                    unsupported.get_or_insert(SurfaceUnsupported {
                        message:
                            "multiple composition bodies are outside the implemented vertical slice"
                                .to_owned(),
                        at: composition.at().clone(),
                    });
                }
                children.push(ast);
                if body.is_none() {
                    body = Some(composition.clone());
                }
                compositions.push(composition);
            } else if self.starts_goal_call_statement() {
                let (clause, ast) = self.goal_call_statement()?;
                validate_goal_uniqueness = true;
                self.ensure_unique_goal_items(&bindings, &labels, &clause.at)?;
                unsupported.get_or_insert(SurfaceUnsupported {
                    message: "goal syntax goal-call is outside the implemented executable slice"
                        .to_owned(),
                    at: clause.at.clone(),
                });
                self.register_goal_names(
                    &clause,
                    &mut bindings,
                    &mut syntax_bindings,
                    validate_goal_uniqueness,
                )?;
                self.register_goal_label(&clause, &mut labels, validate_goal_uniqueness)?;
                children.push(ast);
                clauses.push(clause);
            } else {
                let clause = self.clause(&type_parameters, structured_fact_types)?;
                if let SurfaceClauseKind::SyntaxOnly { kind } = &clause.kind {
                    validate_goal_uniqueness = true;
                    self.ensure_unique_goal_items(&bindings, &labels, &clause.at)?;
                    unsupported.get_or_insert(SurfaceUnsupported {
                        message: format!(
                            "goal syntax {kind} is outside the implemented executable slice"
                        ),
                        at: clause.at.clone(),
                    });
                }
                self.register_goal_names(
                    &clause,
                    &mut bindings,
                    &mut syntax_bindings,
                    validate_goal_uniqueness,
                )?;
                self.register_goal_label(&clause, &mut labels, validate_goal_uniqueness)?;
                children.push(clause.ast.clone());
                clauses.push(clause);
            }
        }
        let end = self.consume().end;
        if unsupported.is_some() {
            for clause in &mut clauses {
                self.enrich_goal_clause_ast(clause);
                if let Some(child) = children.iter_mut().find(|child| child.id == clause.ast.id) {
                    *child = clause.ast.clone();
                }
            }
            for composition in &compositions {
                self.enrich_goal_composition_ast(composition, &mut children);
            }
        }
        let mut attributes = vec![("symbol".to_owned(), Value::Text(symbol.clone()))];
        if !type_parameters.is_empty() {
            attributes.push((
                "type_parameters".to_owned(),
                type_parameters_value(&type_parameters, &type_parameter_bounds),
            ));
        }
        if let Some(refines) = &refines {
            attributes.push(("refines".to_owned(), surface_type_value(refines)));
        }
        let ast = self.ast(
            "goal",
            Some("§goal"),
            keyword.start.clone(),
            end,
            attributes,
            children,
        );
        Ok(SurfaceGoal {
            symbol,
            clauses,
            body,
            unsupported,
            at: keyword.start,
            ast,
        })
    }

    fn enrich_goal_clause_ast(&self, clause: &mut SurfaceClause) {
        let additions = match &clause.kind {
            SurfaceClauseKind::Fact { value_type, .. } => {
                vec![("type".to_owned(), surface_type_value(value_type))]
            }
            SurfaceClauseKind::Contract { condition, .. } => {
                vec![("condition".to_owned(), surface_expression_value(condition))]
            }
            SurfaceClauseKind::Authority { effects, .. } => vec![(
                "effects".to_owned(),
                Value::Array(effects.iter().map(surface_effect_value).collect()),
            )],
            SurfaceClauseKind::Preference { objective, .. } => {
                vec![("objective".to_owned(), surface_expression_value(objective))]
            }
            SurfaceClauseKind::Verify { .. } | SurfaceClauseKind::SyntaxOnly { .. } => Vec::new(),
        };
        for (name, value) in additions {
            if let Some((_, current)) = clause
                .ast
                .attributes
                .iter_mut()
                .find(|(candidate, _)| candidate == &name)
            {
                *current = value;
            } else {
                clause.ast.attributes.push((name, value));
            }
        }
    }

    fn enrich_goal_composition_ast(
        &self,
        composition: &SurfaceComposition,
        children: &mut [AstNode],
    ) {
        let SurfaceComposition::DerivedGate { condition, at, .. } = composition else {
            return;
        };
        if let Some(ast) = children.iter_mut().find(|child| {
            child.kind == "gate"
                && child.span.start.line == at.line
                && child.span.start.column == at.column
                && !child.attributes.iter().any(|(name, _)| name == "condition")
        }) {
            ast.attributes
                .push(("condition".to_owned(), surface_expression_value(condition)));
        }
    }

    fn register_goal_names(
        &self,
        clause: &SurfaceClause,
        bindings: &mut Vec<String>,
        syntax_bindings: &mut Vec<String>,
        reject_all_duplicates: bool,
    ) -> Result<()> {
        let binding = clause.ast.attributes.iter().find_map(|(name, value)| {
            ((name == "name" || name == "binding") && matches!(value, Value::Text(_))).then(|| {
                match value {
                    Value::Text(value) => value.clone(),
                    _ => unreachable!(),
                }
            })
        });
        if let Some(binding) = binding {
            let syntax_only = matches!(clause.kind, SurfaceClauseKind::SyntaxOnly { .. });
            if (reject_all_duplicates && bindings.contains(&binding))
                || (syntax_only && bindings.contains(&binding))
                || (!syntax_only && syntax_bindings.contains(&binding))
            {
                return Err(at(
                    "BHCP1003",
                    format!("duplicate goal binding {binding:?}"),
                    self.source_name,
                    &clause.at,
                ));
            }
            if syntax_only {
                syntax_bindings.push(binding.clone());
            }
            bindings.push(binding);
        }
        Ok(())
    }

    fn register_goal_label(
        &self,
        clause: &SurfaceClause,
        labels: &mut Vec<String>,
        reject_duplicates: bool,
    ) -> Result<()> {
        if let Some(label) = &clause.label {
            if reject_duplicates && labels.contains(label) {
                return Err(at(
                    "BHCP1003",
                    format!("duplicate goal clause label {label:?}"),
                    self.source_name,
                    &clause.at,
                ));
            }
            labels.push(label.clone());
        }
        Ok(())
    }

    fn ensure_unique_goal_items(
        &self,
        bindings: &[String],
        labels: &[String],
        point: &Point,
    ) -> Result<()> {
        for (index, binding) in bindings.iter().enumerate() {
            if bindings[index + 1..].contains(binding) {
                return Err(at(
                    "BHCP1003",
                    format!("duplicate goal binding {binding:?}"),
                    self.source_name,
                    point,
                ));
            }
        }
        for (index, label) in labels.iter().enumerate() {
            if labels[index + 1..].contains(label) {
                return Err(at(
                    "BHCP1003",
                    format!("duplicate goal clause label {label:?}"),
                    self.source_name,
                    point,
                ));
            }
        }
        Ok(())
    }

    fn starts_goal_call_statement(&self) -> bool {
        self.starts_qualified_name()
            || (self.current().kind == TokenKind::Identifier && self.peek().text == "=")
    }

    fn extended_composition_follows(&self) -> bool {
        let mut cursor = self.cursor;
        let mut depth = 0usize;
        let legacy_permits_arguments = match self.current().text.as_str() {
            "§chain" | "§gate" => true,
            "§compose" => {
                let header_end = self.tokens[self.cursor..]
                    .iter()
                    .position(|token| token.text == "{")
                    .map(|offset| self.cursor + offset);
                header_end.is_some_and(|header_end| {
                    self.tokens[self.cursor + 2..header_end]
                        .iter()
                        .map(|token| token.text.as_str())
                        .collect::<String>()
                        == "bhcp/prelude.chain-reducer@0"
                })
            }
            _ => false,
        };
        while let Some(token) = self.tokens.get(cursor) {
            if matches!(token.text.as_str(), "forall" | "exists") && depth == 0 {
                return true;
            }
            if token.text == "{" {
                depth += 1;
            } else if token.text == "}" {
                if depth == 1 {
                    return false;
                }
                depth = depth.saturating_sub(1);
            } else if (token.kind == TokenKind::Keyword && cursor != self.cursor)
                || (depth > 0
                    && token.text == "("
                    && self
                        .tokens
                        .get(cursor + 1)
                        .is_some_and(|next| next.text != ")")
                    && (!legacy_permits_arguments || !self.legacy_goal_arguments_follow(cursor)))
            {
                return true;
            }
            cursor += 1;
        }
        false
    }

    fn legacy_goal_arguments_follow(&self, open: usize) -> bool {
        let mut cursor = open + 1;
        loop {
            if self.tokens.get(cursor).is_none_or(|token| {
                token.kind != TokenKind::Identifier || reserved_goal_binder(&token.text)
            }) {
                return false;
            }
            cursor += 1;
            if self
                .tokens
                .get(cursor)
                .is_none_or(|token| token.text != "=")
            {
                return false;
            }
            cursor += 1;
            if self
                .tokens
                .get(cursor)
                .is_some_and(|token| matches!(token.text.as_str(), "move" | "borrow" | "share"))
            {
                cursor += 1;
            }
            if self.tokens.get(cursor).is_none_or(|token| {
                token.kind != TokenKind::Identifier || reserved_goal_binder(&token.text)
            }) {
                return false;
            }
            cursor += 1;
            match self.tokens.get(cursor).map(|token| token.text.as_str()) {
                Some(")") => return true,
                Some(",") => cursor += 1,
                _ => return false,
            }
        }
    }

    fn syntax_composition(&mut self, terminated: bool) -> Result<(SurfaceComposition, AstNode)> {
        let keyword = self.consume();
        let kind = keyword.text.trim_start_matches('§').to_owned();
        let reducer = if keyword.text == "§compose" {
            self.expect("using")?;
            Some(self.qualified_name()?.0)
        } else {
            None
        };
        let condition = if keyword.text == "§gate" {
            self.expect("when")?;
            Some(self.expression(0)?)
        } else {
            None
        };
        let quantifier = if matches!(keyword.text.as_str(), "§all" | "§any" | "§none")
            && matches!(self.current().text.as_str(), "forall" | "exists")
        {
            let quantifier = self.consume();
            let binder = self.goal_binder("quantifier binder")?;
            self.expect("in")?;
            let domain = self.expression(0)?;
            Some((quantifier.text, binder.text, domain))
        } else {
            None
        };
        self.expect("{")?;
        let mut branch_nodes = Vec::new();
        let mut branches = Vec::new();
        let mut tags = Vec::new();
        while !self.matches("}") {
            let tag = self.goal_binder("branch tag")?;
            if tags.contains(&tag.text) {
                return Err(at(
                    "BHCP1003",
                    "duplicate branch tag",
                    self.source_name,
                    &tag.start,
                ));
            }
            tags.push(tag.text.clone());
            self.expect("=")?;
            if matches!(
                self.current().text.as_str(),
                "§all" | "§any" | "§none" | "§chain" | "§gate" | "§compose"
            ) {
                let (_, nested) = self.syntax_composition(false)?;
                let end = self.expect(";")?.end;
                branch_nodes.push(self.ast(
                    "branch",
                    None,
                    tag.start,
                    end,
                    vec![("tag".to_owned(), Value::Text(tag.text))],
                    vec![nested],
                ));
            } else {
                if self.current().kind == TokenKind::Keyword {
                    return self.fail("BHCP1004", "unsupported nested composition form");
                }
                let (goal, arguments, argument_nodes) = self.goal_call()?;
                let end = self.expect(";")?.end;
                let branch_ast = self.ast(
                    "branch",
                    None,
                    tag.start.clone(),
                    end,
                    vec![
                        ("tag".to_owned(), Value::Text(tag.text.clone())),
                        ("goal".to_owned(), Value::Text(goal.clone())),
                    ],
                    argument_nodes,
                );
                branches.push(SurfaceBranch {
                    tag: tag.text,
                    goal,
                    arguments,
                    at: tag.start,
                    ast: branch_ast.clone(),
                });
                branch_nodes.push(branch_ast);
            }
        }
        let mut end = self.expect("}")?.end;
        if kind == "gate" && branch_nodes.len() != 1 {
            return Err(at(
                "BHCP1001",
                "gate composition requires exactly one branch",
                self.source_name,
                &keyword.start,
            ));
        }
        if terminated {
            end = self.expect(";")?.end;
        }
        let mut attributes = Vec::new();
        if let Some(reducer) = reducer {
            attributes.push(("reducer".to_owned(), Value::Text(reducer)));
        }
        if let Some(condition) = condition {
            attributes.push(("condition".to_owned(), surface_expression_value(&condition)));
        }
        if let Some((kind, binder, domain)) = quantifier {
            attributes.push((
                "quantifier".to_owned(),
                Value::map([
                    ("kind", Value::Text(kind)),
                    ("binder", Value::Text(binder)),
                    ("domain", surface_expression_value(&domain)),
                ]),
            ));
        }
        let ast = self.ast(
            &kind,
            Some(&keyword.text),
            keyword.start.clone(),
            end,
            attributes,
            branch_nodes,
        );
        Ok((
            SurfaceComposition::SyntaxOnly {
                branches,
                at: keyword.start,
            },
            ast,
        ))
    }

    fn goal_call(&mut self) -> Result<(String, Vec<SurfaceGoalArgument>, Vec<AstNode>)> {
        let (goal, _) = self.qualified_name()?;
        self.expect("(")?;
        let mut arguments = Vec::new();
        let mut nodes = Vec::new();
        while !self.matches(")") {
            let name = self.goal_binder("argument name")?;
            if arguments
                .iter()
                .any(|argument: &SurfaceGoalArgument| argument.name == name.text)
            {
                return Err(at(
                    "BHCP1003",
                    "duplicate argument",
                    self.source_name,
                    &name.start,
                ));
            }
            self.expect("=")?;
            let (mode, mode_name) = if self.matches("move") {
                self.consume();
                (SurfaceArgumentMode::Move, "move")
            } else if self.matches("borrow") {
                self.consume();
                (SurfaceArgumentMode::Borrow, "borrow")
            } else if self.matches("share") {
                self.consume();
                (SurfaceArgumentMode::Share, "share")
            } else {
                (SurfaceArgumentMode::Value, "value")
            };
            let value = self.expression(0)?;
            let source = match &value {
                SurfaceExpression::Reference { name, .. } => name.clone(),
                _ => "<expression>".to_owned(),
            };
            let end = self.tokens[self.cursor.saturating_sub(1)].end.clone();
            nodes.push(self.ast(
                "argument",
                None,
                name.start.clone(),
                end,
                vec![
                    ("name".to_owned(), Value::Text(name.text.clone())),
                    ("mode".to_owned(), Value::Text(mode_name.to_owned())),
                    ("value".to_owned(), surface_expression_value(&value)),
                ],
                vec![],
            ));
            arguments.push(SurfaceGoalArgument {
                name: name.text,
                mode,
                source,
                at: name.start,
                ast: nodes.last().expect("argument AST was added").clone(),
            });
            if !self.matches(",") {
                break;
            }
            self.consume();
            if self.matches(")") {
                return self.fail("BHCP1001", "goal-call arguments cannot end with a comma");
            }
        }
        self.expect(")")?;
        Ok((goal, arguments, nodes))
    }

    fn goal_call_statement(&mut self) -> Result<(SurfaceClause, AstNode)> {
        let start = self.current().start.clone();
        let binding = if self.current().kind == TokenKind::Identifier && self.peek().text == "=" {
            let binding = self.goal_binder("goal-call binding")?.text;
            self.expect("=")?;
            Some(binding)
        } else {
            None
        };
        let (goal, _, arguments) = self.goal_call()?;
        let end = self.expect(";")?.end;
        let mut attributes = vec![("goal".to_owned(), Value::Text(goal))];
        if let Some(binding) = binding {
            attributes.push(("binding".to_owned(), Value::Text(binding)));
        }
        let ast = self.ast("goal-call", None, start.clone(), end, attributes, arguments);
        Ok((
            SurfaceClause {
                label: None,
                kind: SurfaceClauseKind::SyntaxOnly {
                    kind: "goal-call".to_owned(),
                },
                at: start,
                ast: ast.clone(),
            },
            ast,
        ))
    }

    fn composition(&mut self) -> Result<(SurfaceComposition, AstNode)> {
        let keyword = self.consume();
        let derived = keyword.text.clone();
        let reducer = if keyword.text == "§compose" {
            self.expect("using")?;
            Some(self.qualified_name()?.0)
        } else {
            None
        };
        let condition = if derived == "§gate" {
            self.expect("when")?;
            Some(self.expression(0)?)
        } else {
            None
        };
        let permits_arguments = derived == "§chain"
            || derived == "§gate"
            || reducer.as_deref() == Some("bhcp/prelude.chain-reducer@0");
        self.expect("{")?;
        let mut branches = Vec::new();
        let mut tags = Vec::new();
        while !self.matches("}") {
            if self.current().kind == TokenKind::Eof {
                return Err(at(
                    "BHCP1001",
                    "unterminated composition block",
                    self.source_name,
                    &keyword.start,
                ));
            }
            let tag = self.goal_binder("branch tag")?;
            if tags.contains(&tag.text) {
                return Err(at(
                    "BHCP1003",
                    "duplicate branch tag",
                    self.source_name,
                    &tag.start,
                ));
            }
            tags.push(tag.text.clone());
            self.expect("=")?;
            if self.current().kind == TokenKind::Keyword {
                return self.fail(
                    "BHCP1004",
                    "nested composition is outside the implemented vertical slice",
                );
            }
            let (goal, _) = self.qualified_name()?;
            self.expect("(")?;
            if !self.matches(")") && !permits_arguments {
                return self.fail(
                    "BHCP1004",
                    "goal-call arguments are outside the implemented composition slice",
                );
            }
            let mut arguments = Vec::new();
            if !self.matches(")") {
                loop {
                    let name = self.goal_binder("argument name")?;
                    if arguments
                        .iter()
                        .any(|argument: &SurfaceGoalArgument| argument.name == name.text)
                    {
                        return Err(at(
                            "BHCP1003",
                            "duplicate argument",
                            self.source_name,
                            &name.start,
                        ));
                    }
                    self.expect("=")?;
                    let (mode, mode_name) = if self.matches("move") {
                        self.consume();
                        (SurfaceArgumentMode::Move, "move")
                    } else if self.matches("borrow") {
                        self.consume();
                        (SurfaceArgumentMode::Borrow, "borrow")
                    } else if self.matches("share") {
                        self.consume();
                        (SurfaceArgumentMode::Share, "share")
                    } else {
                        (SurfaceArgumentMode::Value, "value")
                    };
                    let source = self.goal_binder("argument source")?;
                    let argument_ast = self.ast(
                        "argument",
                        None,
                        name.start.clone(),
                        source.end.clone(),
                        vec![
                            ("name".to_owned(), Value::Text(name.text.clone())),
                            ("mode".to_owned(), Value::Text(mode_name.to_owned())),
                            ("source".to_owned(), Value::Text(source.text.clone())),
                        ],
                        vec![],
                    );
                    arguments.push(SurfaceGoalArgument {
                        name: name.text,
                        mode,
                        source: source.text,
                        at: name.start,
                        ast: argument_ast,
                    });
                    if !self.matches(",") {
                        break;
                    }
                    self.consume();
                }
            }
            self.expect(")")?;
            let branch_end = self.expect(";")?.end;
            let branch_ast = self.ast(
                "branch",
                None,
                tag.start.clone(),
                branch_end,
                vec![
                    ("tag".to_owned(), Value::Text(tag.text.clone())),
                    ("goal".to_owned(), Value::Text(goal.clone())),
                ],
                arguments
                    .iter()
                    .map(|argument| argument.ast.clone())
                    .collect(),
            );
            branches.push(SurfaceBranch {
                tag: tag.text,
                goal,
                arguments,
                at: tag.start,
                ast: branch_ast,
            });
        }
        self.consume();
        if !permits_arguments && branches.iter().any(|branch| !branch.arguments.is_empty()) {
            return self.fail(
                "BHCP1004",
                "goal-call arguments are outside the implemented composition slice",
            );
        }
        let end = self.expect(";")?.end;
        let ast = self.ast(
            if reducer.is_some() {
                "compose"
            } else if derived == "§any" {
                "any"
            } else if derived == "§none" {
                "none"
            } else if derived == "§chain" {
                "chain"
            } else if derived == "§gate" {
                "gate"
            } else {
                "all"
            },
            Some(&keyword.text),
            keyword.start.clone(),
            end,
            reducer
                .as_ref()
                .map(|value| vec![("reducer".to_owned(), Value::Text(value.clone()))])
                .unwrap_or_default(),
            branches.iter().map(|branch| branch.ast.clone()).collect(),
        );
        let composition = if let Some(reducer) = reducer {
            SurfaceComposition::Compose {
                reducer,
                branches,
                at: keyword.start,
            }
        } else if derived == "§any" {
            SurfaceComposition::DerivedAny {
                branches,
                at: keyword.start,
            }
        } else if derived == "§none" {
            SurfaceComposition::DerivedNone {
                branches,
                at: keyword.start,
            }
        } else if derived == "§chain" {
            SurfaceComposition::DerivedChain {
                branches,
                at: keyword.start,
            }
        } else if derived == "§gate" {
            SurfaceComposition::DerivedGate {
                condition: condition.expect("gate parsed a condition"),
                branches,
                at: keyword.start,
            }
        } else {
            SurfaceComposition::DerivedAll {
                branches,
                at: keyword.start,
            }
        };
        Ok((composition, ast))
    }

    fn clause(
        &mut self,
        type_parameters: &[String],
        structured_fact_types: bool,
    ) -> Result<SurfaceClause> {
        let keyword = self.consume();
        if keyword.kind != TokenKind::Keyword {
            return Err(at(
                "BHCP1001",
                "expected goal clause",
                self.source_name,
                &keyword.start,
            ));
        }
        let label = self.label()?;
        let (kind, attributes) = match keyword.text.as_str() {
            "§input" | "§output" | "§resource" | "§state" => {
                let name = self.goal_binder("binding name")?.text;
                self.expect(":")?;
                let value_type = self.value_type(type_parameters)?;
                let initializer = if self.matches("=") {
                    self.consume();
                    Some(self.expression(0)?)
                } else {
                    None
                };
                let kind = match keyword.text.as_str() {
                    "§input" => "input",
                    "§output" => "output",
                    "§resource" => "resource",
                    _ => "state",
                };
                let clause_kind = if matches!(kind, "input" | "output") && initializer.is_none() {
                    SurfaceClauseKind::Fact {
                        kind,
                        name: name.clone(),
                        value_type: value_type.clone(),
                    }
                } else {
                    SurfaceClauseKind::SyntaxOnly {
                        kind: kind.to_owned(),
                    }
                };
                let type_attribute = if structured_fact_types
                    || matches!(kind, "resource" | "state")
                    || initializer.is_some()
                {
                    surface_type_value(&value_type)
                } else {
                    Value::Text(type_name(&value_type))
                };
                let mut attributes = vec![
                    ("name".to_owned(), Value::Text(name)),
                    ("type".to_owned(), type_attribute),
                ];
                if let Some(initializer) = &initializer {
                    attributes.push((
                        "initializer".to_owned(),
                        surface_expression_value(initializer),
                    ));
                }
                (clause_kind, attributes)
            }
            "§requires" | "§ensures" | "§invariant" | "§limit" => {
                let kind = match keyword.text.as_str() {
                    "§requires" => "requires",
                    "§ensures" => "ensures",
                    "§invariant" => "invariant",
                    _ => "limit",
                };
                let dimension = if kind == "limit" && self.starts_qualified_name() {
                    let dimension = self.qualified_name()?.0;
                    self.expect(":")?;
                    Some(dimension)
                } else {
                    None
                };
                let mut attributes = Vec::new();
                if let Some(dimension) = &dimension {
                    attributes.push(("dimension".to_owned(), Value::Text(dimension.clone())));
                }
                let condition = self.expression(0)?;
                if kind == "invariant" {
                    attributes.push(("condition".to_owned(), surface_expression_value(&condition)));
                    (
                        SurfaceClauseKind::SyntaxOnly {
                            kind: kind.to_owned(),
                        },
                        attributes,
                    )
                } else {
                    if structured_fact_types {
                        attributes
                            .push(("condition".to_owned(), surface_expression_value(&condition)));
                    }
                    (
                        SurfaceClauseKind::Contract {
                            kind,
                            dimension,
                            condition,
                        },
                        attributes,
                    )
                }
            }
            "§allows" | "§forbids" => {
                let kind = if keyword.text == "§allows" {
                    "allows"
                } else {
                    "forbids"
                };
                let mut effects = vec![self.effect()?];
                while self.matches(",") {
                    self.consume();
                    effects.push(self.effect()?);
                }
                let attributes = if structured_fact_types {
                    vec![(
                        "effects".to_owned(),
                        Value::Array(effects.iter().map(surface_effect_value).collect()),
                    )]
                } else {
                    Vec::new()
                };
                (SurfaceClauseKind::Authority { kind, effects }, attributes)
            }
            "§prefer" => {
                let positive_priority =
                    self.current().kind == TokenKind::Number && self.peek().text == ":";
                let negative_priority = self.matches("-")
                    && self
                        .tokens
                        .get(self.cursor + 1)
                        .is_some_and(|token| token.kind == TokenKind::Number)
                    && self
                        .tokens
                        .get(self.cursor + 2)
                        .is_some_and(|token| token.text == ":");
                if label.is_some() && (positive_priority || negative_priority) {
                    return self.fail(
                        "BHCP1001",
                        "preference priority must precede its optional label",
                    );
                }
                let mut priority = 0;
                if negative_priority {
                    self.consume();
                    priority = match self.consume().value {
                        Some(TokenValue::Integer(value)) if value > 0 => -value,
                        _ => {
                            return self.fail(
                                "BHCP1001",
                                "negative preference priority requires a positive integer",
                            );
                        }
                    };
                    self.consume();
                } else if positive_priority {
                    priority = match self.consume().value {
                        Some(TokenValue::Integer(value)) => value,
                        _ => unreachable!(),
                    };
                    self.consume();
                }
                let preference_label = if label.is_none() { self.label()? } else { None };
                let final_label = label.or(preference_label);
                let objective = self.expression(0)?;
                let end = self.expect(";")?.end;
                let mut attributes = vec![(
                    "priority".to_owned(),
                    Value::Array(vec![
                        Value::Text("integer".to_owned()),
                        Value::Integer(priority),
                    ]),
                )];
                if structured_fact_types {
                    attributes.push(("objective".to_owned(), surface_expression_value(&objective)));
                }
                if let Some(label) = &final_label {
                    attributes.push(("label".to_owned(), Value::Text(label.clone())));
                }
                let ast = self.ast(
                    "prefer",
                    Some(&keyword.text),
                    keyword.start.clone(),
                    end,
                    attributes,
                    vec![],
                );
                return Ok(SurfaceClause {
                    label: final_label,
                    kind: SurfaceClauseKind::Preference {
                        priority,
                        objective,
                    },
                    at: keyword.start,
                    ast,
                });
            }
            "§verify" => {
                let binding = self.verifier_binding()?;
                let verifier = binding.symbol;
                let mut obligation_labels = Vec::new();
                if self.matches("for") {
                    self.consume();
                    loop {
                        let target = self.consume();
                        let Some(TokenValue::Text(label)) = target.value else {
                            return Err(at(
                                "BHCP1001",
                                "expected quoted obligation label after for",
                                self.source_name,
                                &target.start,
                            ));
                        };
                        if target.kind != TokenKind::String {
                            return Err(at(
                                "BHCP1001",
                                "expected quoted obligation label after for",
                                self.source_name,
                                &target.start,
                            ));
                        }
                        if obligation_labels.contains(&label) {
                            return Err(at(
                                "BHCP1003",
                                "duplicate verifier obligation label",
                                self.source_name,
                                &target.start,
                            ));
                        }
                        obligation_labels.push(label);
                        if !self.matches(",") {
                            break;
                        }
                        self.consume();
                    }
                }
                let mut attributes = vec![("verifier".to_owned(), Value::Text(verifier.clone()))];
                if !obligation_labels.is_empty() {
                    attributes.push((
                        "obligations".to_owned(),
                        Value::Array(obligation_labels.iter().cloned().map(Value::Text).collect()),
                    ));
                }
                if !binding.arguments.is_empty() {
                    attributes.push((
                        "verifier_arguments".to_owned(),
                        Value::Array(
                            binding
                                .arguments
                                .iter()
                                .map(verifier_argument_value)
                                .collect(),
                        ),
                    ));
                }
                (
                    if binding.arguments.is_empty() {
                        SurfaceClauseKind::Verify {
                            verifier: verifier.clone(),
                            obligation_labels,
                        }
                    } else {
                        SurfaceClauseKind::SyntaxOnly {
                            kind: "verify".to_owned(),
                        }
                    },
                    attributes,
                )
            }
            "§case" => return self.case_clause(keyword, label),
            _ => {
                return Err(at(
                    "BHCP1004",
                    format!(
                        "syntax {} is outside the implemented vertical slice",
                        keyword.text
                    ),
                    self.source_name,
                    &keyword.start,
                ));
            }
        };
        let end = self.expect(";")?.end;
        let mut attributes = attributes;
        if let Some(label) = &label {
            attributes.push(("label".to_owned(), Value::Text(label.clone())));
        }
        let ast = self.ast(
            keyword.text.trim_start_matches('§'),
            Some(&keyword.text),
            keyword.start.clone(),
            end,
            attributes,
            vec![],
        );
        Ok(SurfaceClause {
            label,
            kind,
            at: keyword.start,
            ast,
        })
    }

    fn case_clause(&mut self, keyword: Token, label: Option<String>) -> Result<SurfaceClause> {
        self.expect("{")?;
        let mut children = Vec::new();
        let mut bindings = Vec::new();
        while !self.matches("}") {
            if self.matches("expect") {
                let start = self.consume().start;
                let pattern = if self.matches("completed") {
                    self.consume();
                    let verdict = self.identifier("verdict state")?;
                    if !matches!(
                        verdict.text.as_str(),
                        "satisfied" | "refuted" | "unresolved"
                    ) {
                        return Err(at(
                            "BHCP1001",
                            "expected verdict state satisfied, refuted, or unresolved",
                            self.source_name,
                            &verdict.start,
                        ));
                    }
                    format!("completed:{}", verdict.text)
                } else if self.matches("faulted") {
                    self.consume();
                    "faulted".to_owned()
                } else {
                    return self.fail("BHCP1001", "expected completed or faulted");
                };
                let condition = if self.matches(";") {
                    None
                } else {
                    Some(self.expression(0)?)
                };
                let end = self.expect(";")?.end;
                let mut attributes = vec![("pattern".to_owned(), Value::Text(pattern))];
                if let Some(condition) = condition {
                    attributes.push(("condition".to_owned(), surface_expression_value(&condition)));
                }
                children.push(self.ast("expectation", None, start, end, attributes, vec![]));
            } else {
                let name = self.goal_binder("case binding")?;
                if bindings.contains(&name.text) {
                    return Err(at(
                        "BHCP1003",
                        "duplicate case binding",
                        self.source_name,
                        &name.start,
                    ));
                }
                bindings.push(name.text.clone());
                self.expect("=")?;
                let value = self.expression(0)?;
                let end = self.expect(";")?.end;
                children.push(self.ast(
                    "binding",
                    None,
                    name.start,
                    end,
                    vec![
                        ("name".to_owned(), Value::Text(name.text)),
                        ("value".to_owned(), surface_expression_value(&value)),
                    ],
                    vec![],
                ));
            }
        }
        self.expect("}")?;
        let end = self.expect(";")?.end;
        let mut attributes = Vec::new();
        if let Some(label) = &label {
            attributes.push(("label".to_owned(), Value::Text(label.clone())));
        }
        let ast = self.ast(
            "case",
            Some("§case"),
            keyword.start.clone(),
            end,
            attributes,
            children,
        );
        Ok(SurfaceClause {
            label,
            kind: SurfaceClauseKind::SyntaxOnly {
                kind: "case".to_owned(),
            },
            at: keyword.start,
            ast,
        })
    }

    fn effect(&mut self) -> Result<SurfaceEffect> {
        let (symbol, at) = self.qualified_name()?;
        let mut arguments = Vec::new();
        if self.matches("(") {
            self.consume();
            if !self.matches(")") {
                loop {
                    arguments.push(self.expression(0)?);
                    if !self.matches(",") {
                        break;
                    }
                    self.consume();
                }
            }
            self.expect(")")?;
        }
        Ok(SurfaceEffect {
            symbol,
            arguments,
            at,
        })
    }

    fn expression(&mut self, minimum: u8) -> Result<SurfaceExpression> {
        if minimum == 0 && self.matches("if") {
            let keyword = self.consume();
            let condition = self.expression(0)?;
            self.expect("then")?;
            let consequent = self.expression(0)?;
            self.expect("else")?;
            let alternative = self.expression(0)?;
            return Ok(SurfaceExpression::If {
                condition: Box::new(condition),
                consequent: Box::new(consequent),
                alternative: Box::new(alternative),
                at: keyword.start,
            });
        }
        let mut left = self.unary()?;
        while let Some(precedence) = operator_precedence(&self.current().text) {
            if precedence < minimum {
                break;
            }
            let operator = self.consume();
            let right = self.expression(precedence + 1)?;
            left = SurfaceExpression::Binary {
                operator: operator.text,
                left: Box::new(left),
                right: Box::new(right),
                at: operator.start,
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<SurfaceExpression> {
        if matches!(
            self.current().text.as_str(),
            "let" | "match" | "forall" | "exists" | "set" | "map" | "{" | "["
        ) {
            return self.fail(
                "BHCP1004",
                format!(
                    "expression syntax {:?} is outside the implemented vertical slice",
                    self.current().text
                ),
            );
        }
        if self.matches("!") || self.matches("-") {
            let operator = self.consume();
            return Ok(SurfaceExpression::Unary {
                operator: operator.text,
                operand: Box::new(self.unary()?),
                at: operator.start,
            });
        }
        let token = self.consume();
        let mut value = match (token.kind, token.value) {
            (TokenKind::Number, Some(TokenValue::Integer(value))) => SurfaceExpression::Literal {
                value: SurfaceLiteral::Integer(value),
                at: token.start,
            },
            (TokenKind::String, Some(TokenValue::Text(value))) => SurfaceExpression::Literal {
                value: SurfaceLiteral::Text(value),
                at: token.start,
            },
            (TokenKind::Identifier, _) if token.text == "true" || token.text == "false" => {
                SurfaceExpression::Literal {
                    value: SurfaceLiteral::Bool(token.text == "true"),
                    at: token.start,
                }
            }
            (TokenKind::Identifier, _) => {
                let at = token.start.clone();
                let mut name = token.text;
                if self.semantic_name_suffix_follows() {
                    while self.matches("/") || self.matches(".") || self.matches("::") {
                        let separator = self.consume().text;
                        name.push_str(if separator == "::" { "/" } else { &separator });
                        name.push_str(&self.identifier("name segment")?.text);
                    }
                    name.push_str(&self.consume().text);
                    let version = self.consume();
                    if !matches!(version.kind, TokenKind::Number | TokenKind::Identifier) {
                        return self.fail("BHCP1001", "expected semantic-name version");
                    }
                    name.push_str(&version.text);
                    while self.matches(".") {
                        name.push_str(&self.consume().text);
                        name.push_str(&self.consume().text);
                    }
                }
                SurfaceExpression::Reference { name, at }
            }
            (_, _) if token.text == "(" => {
                let value = self.expression(0)?;
                self.expect(")")?;
                value
            }
            _ => {
                return Err(at(
                    "BHCP1001",
                    format!("expected expression, found {:?}", token.text),
                    self.source_name,
                    &token.start,
                ));
            }
        };
        loop {
            if self.matches("(") {
                let at = value.at().clone();
                let SurfaceExpression::Reference { name: function, .. } = value else {
                    return self.fail(
                        "BHCP1004",
                        "only named function calls are implemented in this expression slice",
                    );
                };
                self.consume();
                let mut arguments = Vec::new();
                if !self.matches(")") {
                    loop {
                        arguments.push(self.expression(0)?);
                        if !self.matches(",") {
                            break;
                        }
                        self.consume();
                    }
                }
                self.expect(")")?;
                value = SurfaceExpression::Call {
                    function,
                    arguments,
                    at,
                };
            } else {
                break;
            }
        }
        Ok(value)
    }

    fn semantic_name_suffix_follows(&self) -> bool {
        let mut cursor = self.cursor;
        let mut has_path_separator = false;
        while matches!(
            self.tokens.get(cursor).map(|token| token.text.as_str()),
            Some("/") | Some(".") | Some("::")
        ) && matches!(
            self.tokens.get(cursor + 1).map(|token| token.kind),
            Some(TokenKind::Identifier)
        ) {
            has_path_separator = true;
            cursor += 2;
        }
        has_path_separator
            && matches!(
                self.tokens.get(cursor).map(|token| token.text.as_str()),
                Some("@")
            )
    }

    fn value_type(&mut self, parameters: &[String]) -> Result<SurfaceType> {
        self.union_type(parameters)
    }

    fn union_type(&mut self, parameters: &[String]) -> Result<SurfaceType> {
        let first = self.intersection_type(parameters)?;
        if !self.matches("|") {
            return Ok(first);
        }
        let mut members = vec![first];
        while self.matches("|") {
            self.consume();
            members.push(self.intersection_type(parameters)?);
        }
        Ok(SurfaceType::Union(members))
    }

    fn intersection_type(&mut self, parameters: &[String]) -> Result<SurfaceType> {
        let first = self.prefix_type(parameters)?;
        if !self.matches("&") {
            return Ok(first);
        }
        let mut members = vec![first];
        while self.matches("&") {
            self.consume();
            members.push(self.prefix_type(parameters)?);
        }
        Ok(SurfaceType::Intersection(members))
    }

    fn prefix_type(&mut self, parameters: &[String]) -> Result<SurfaceType> {
        let handle = if matches!(
            self.current().text.as_str(),
            "owned" | "shared" | "borrowed"
        ) {
            let ownership = self.consume().text;
            let access = if matches!(self.current().text.as_str(), "read" | "write") {
                Some(self.consume().text)
            } else {
                None
            };
            let usage = if matches!(
                self.current().text.as_str(),
                "unrestricted" | "affine" | "linear"
            ) {
                Some(self.consume().text)
            } else {
                None
            };
            let lifetime = if self.matches("'") {
                self.consume();
                Some(self.identifier("lifetime")?.text)
            } else {
                None
            };
            Some((ownership, access, usage, lifetime))
        } else {
            None
        };
        let mut value = self.primary_type(parameters)?;
        if let Some((ownership, access, usage, lifetime)) = handle {
            value = SurfaceType::Handle {
                ownership,
                access,
                usage,
                lifetime,
                value_type: Box::new(value),
            };
        }
        if self.matches("where") {
            self.consume();
            let binder = self.binder("refinement binder")?;
            self.expect("=>")?;
            value = SurfaceType::Refined {
                value_type: Box::new(value),
                binder: binder.text,
                predicate: Box::new(self.expression(0)?),
            };
        }
        Ok(value)
    }

    fn primary_type(&mut self, parameters: &[String]) -> Result<SurfaceType> {
        if self.matches("[") {
            self.consume();
            let element = self.value_type(parameters)?;
            self.expect("]")?;
            return Ok(SurfaceType::List(Box::new(element)));
        }
        if self.matches("{") {
            self.consume();
            let mut fields = Vec::new();
            let mut open = false;
            while !self.matches("}") {
                if self.matches(".")
                    && self.peek().text == "."
                    && self
                        .tokens
                        .get(self.cursor + 2)
                        .is_some_and(|token| token.text == ".")
                {
                    self.consume();
                    self.consume();
                    self.consume();
                    open = true;
                    break;
                }
                let name = self.identifier("field name")?;
                if fields
                    .iter()
                    .any(|field: &SurfaceDefinitionField| field.name == name.text)
                {
                    return Err(at(
                        "BHCP1003",
                        "duplicate record field",
                        self.source_name,
                        &name.start,
                    ));
                }
                let optional = if self.matches("?") {
                    self.consume();
                    true
                } else {
                    false
                };
                self.expect(":")?;
                fields.push(SurfaceDefinitionField {
                    name: name.text,
                    optional,
                    value_type: self.value_type(parameters)?,
                });
                if !self.matches(",") {
                    break;
                }
                self.consume();
                if self.matches("}") {
                    return self.fail("BHCP1001", "trailing record comma requires ...");
                }
            }
            self.expect("}")?;
            if !open && fields.iter().all(|field| !field.optional) {
                return Ok(SurfaceType::Record(
                    fields
                        .into_iter()
                        .map(|field| SurfaceFieldType {
                            name: field.name,
                            value_type: field.value_type,
                        })
                        .collect(),
                ));
            }
            return Ok(SurfaceType::StructuralRecord { fields, open });
        }
        if self.matches("(") {
            self.consume();
            let first = self.value_type(parameters)?;
            if !self.matches(",") {
                self.expect(")")?;
                return Ok(first);
            }
            self.consume();
            let mut members = vec![first];
            if !self.matches(")") {
                loop {
                    members.push(self.value_type(parameters)?);
                    if !self.matches(",") {
                        break;
                    }
                    self.consume();
                    if self.matches(")") {
                        break;
                    }
                }
            }
            self.expect(")")?;
            return Ok(SurfaceType::Tuple(members));
        }
        if self.matches("variant") {
            self.consume();
            self.expect("{")?;
            let mut cases = Vec::new();
            while !self.matches("}") {
                let name = self.identifier("variant tag")?;
                if cases
                    .iter()
                    .any(|case: &SurfaceVariantCase| case.name == name.text)
                {
                    return Err(at(
                        "BHCP1003",
                        "duplicate variant tag",
                        self.source_name,
                        &name.start,
                    ));
                }
                let mut payload = Vec::new();
                if self.matches("(") {
                    self.consume();
                    if !self.matches(")") {
                        loop {
                            payload.push(self.value_type(parameters)?);
                            if !self.matches(",") {
                                break;
                            }
                            self.consume();
                        }
                    }
                    self.expect(")")?;
                }
                cases.push(SurfaceVariantCase {
                    name: name.text,
                    payload,
                });
                if !self.matches(",") {
                    break;
                }
                self.consume();
                if self.matches("}") {
                    return self.fail("BHCP1001", "variant cases cannot end with a comma");
                }
            }
            if cases.is_empty() {
                return self.fail("BHCP1001", "variant type requires at least one case");
            }
            self.expect("}")?;
            return Ok(SurfaceType::Variant(cases));
        }
        if self.matches("Goal") {
            self.consume();
            self.expect("<")?;
            let input = self.value_type(parameters)?;
            self.expect(",")?;
            let output = self.value_type(parameters)?;
            let mut effects = None;
            let mut evidence = None;
            if self.matches(",") {
                self.consume();
                effects = Some(self.effect_row()?);
                if self.matches(",") {
                    self.consume();
                    evidence = Some(Box::new(self.value_type(parameters)?));
                }
            }
            self.expect(">")?;
            return Ok(SurfaceType::Goal {
                input: Box::new(input),
                output: Box::new(output),
                effects,
                evidence,
            });
        }
        if self.starts_qualified_name() {
            let (symbol, _) = self.qualified_name()?;
            let arguments = self.type_arguments(parameters)?;
            return Ok(SurfaceType::Nominal { symbol, arguments });
        }
        let token = self.identifier("type")?;
        let value = match token.text.as_str() {
            "Bool" => SurfaceType::Primitive("Bool"),
            "Text" => SurfaceType::Primitive("Text"),
            "Bytes" => SurfaceType::Primitive("Bytes"),
            "Unit" => SurfaceType::Primitive("Unit"),
            "Timestamp" => SurfaceType::Primitive("Timestamp"),
            "Duration" => SurfaceType::Primitive("Duration"),
            "Integer" => SurfaceType::Exact("Integer"),
            "Rational" => SurfaceType::Exact("Rational"),
            "Decimal" => SurfaceType::Exact("Decimal"),
            "Dynamic" => SurfaceType::Dynamic,
            "Never" => SurfaceType::Never,
            "Reduction" => {
                self.expect("<")?;
                let output = self.value_type(parameters)?;
                self.expect(">")?;
                SurfaceType::Reduction(Box::new(output))
            }
            "Meta" => {
                self.expect("<")?;
                let kind = self.identifier("meta kind")?;
                let kind = match kind.text.as_str() {
                    "DerivedForm" => "derived-form",
                    "NetworkShape" => "network-shape",
                    _ => {
                        return Err(at(
                            "BHCP1004",
                            "unsupported meta type",
                            self.source_name,
                            &kind.start,
                        ));
                    }
                };
                self.expect(",")?;
                let input = self.value_type(parameters)?;
                self.expect(",")?;
                let output = self.value_type(parameters)?;
                self.expect(">")?;
                SurfaceType::Meta {
                    kind,
                    input: Box::new(input),
                    output: Box::new(output),
                }
            }
            "List" | "Set" => {
                let mut arguments = self.type_arguments(parameters)?;
                if arguments.len() != 1 {
                    return Err(at(
                        "BHCP1001",
                        format!("{} requires exactly one type argument", token.text),
                        self.source_name,
                        &token.start,
                    ));
                }
                let element = Box::new(arguments.remove(0));
                if token.text == "List" {
                    SurfaceType::List(element)
                } else {
                    SurfaceType::Set(element)
                }
            }
            "Map" => {
                let mut arguments = self.type_arguments(parameters)?;
                if arguments.len() != 2 {
                    return Err(at(
                        "BHCP1001",
                        "Map requires exactly two type arguments",
                        self.source_name,
                        &token.start,
                    ));
                }
                let value = Box::new(arguments.pop().expect("Map arity was checked"));
                let key = Box::new(arguments.pop().expect("Map arity was checked"));
                SurfaceType::Map { key, value }
            }
            "Option" => {
                let mut arguments = self.type_arguments(parameters)?;
                if arguments.len() != 1 {
                    return Err(at(
                        "BHCP1001",
                        "Option requires exactly one type argument",
                        self.source_name,
                        &token.start,
                    ));
                }
                SurfaceType::Option(Box::new(arguments.remove(0)))
            }
            "Result" => {
                let mut arguments = self.type_arguments(parameters)?;
                if arguments.len() != 2 {
                    return Err(at(
                        "BHCP1001",
                        "Result requires exactly two type arguments",
                        self.source_name,
                        &token.start,
                    ));
                }
                let error = Box::new(arguments.pop().expect("Result arity was checked"));
                let ok = Box::new(arguments.pop().expect("Result arity was checked"));
                SurfaceType::Result { ok, error }
            }
            name if parameters.iter().any(|parameter| parameter == name) => {
                SurfaceType::Parameter(name.to_owned())
            }
            _ => {
                return Err(at(
                    "BHCP1002",
                    "semantic names must use namespace/name@version",
                    self.source_name,
                    &token.start,
                ));
            }
        };
        Ok(value)
    }

    fn type_arguments(&mut self, parameters: &[String]) -> Result<Vec<SurfaceType>> {
        if !self.matches("<") {
            return Ok(vec![]);
        }
        self.consume();
        let mut arguments = vec![self.value_type(parameters)?];
        while self.matches(",") {
            self.consume();
            arguments.push(self.value_type(parameters)?);
        }
        self.expect(">")?;
        Ok(arguments)
    }

    fn effect_row(&mut self) -> Result<SurfaceEffectRow> {
        self.expect("!")?;
        self.expect("{")?;
        let mut effects = Vec::new();
        let mut tail = None;
        if !self.matches("}") && !self.matches("|") {
            loop {
                let (effect, at) = self.qualified_name()?;
                if effects.contains(&effect) {
                    return Err(at_fn(
                        "BHCP1003",
                        "duplicate effect-row member",
                        self.source_name,
                        &at,
                    ));
                }
                effects.push(effect);
                if !self.matches(",") {
                    break;
                }
                self.consume();
                if self.matches("|") {
                    return self.fail(
                        "BHCP1001",
                        "effect-row tail follows the final member without a comma",
                    );
                }
            }
        }
        if self.matches("|") {
            self.consume();
            tail = Some(self.binder("effect-row tail")?.text);
        }
        self.expect("}")?;
        Ok(SurfaceEffectRow { effects, tail })
    }

    fn qualified_name(&mut self) -> Result<(String, Point)> {
        let first = self.identifier("qualified name")?;
        let at = first.start.clone();
        let mut segments = vec![first.text];
        while self.matches("/") || self.matches(".") || self.matches("::") {
            let separator = self.consume().text;
            let component = self.identifier("name segment")?.text;
            if separator == "/" || separator == "::" {
                segments.push(component);
            } else {
                let last = segments.last_mut().expect("qualified name has a component");
                last.push('.');
                last.push_str(&component);
            }
        }
        if segments.len() < 2 || !self.matches("@") {
            return Err(crate::diagnostic::Diagnostic::new(
                "BHCP1002",
                "semantic names must use namespace/name@version",
                self.source_name,
                at.line,
                at.column,
            ));
        }
        self.consume();
        let version = self.consume();
        if !matches!(version.kind, TokenKind::Number | TokenKind::Identifier) {
            return Err(at_fn(
                "BHCP1001",
                "expected semantic-name version",
                self.source_name,
                &version.start,
            ));
        }
        let mut version_text = version.text;
        while self.matches(".") {
            version_text.push_str(&self.consume().text);
            let part = self.consume();
            if !matches!(part.kind, TokenKind::Number | TokenKind::Identifier) {
                return Err(at_fn(
                    "BHCP1001",
                    "expected version component",
                    self.source_name,
                    &part.start,
                ));
            }
            version_text.push_str(&part.text);
        }
        Ok((format!("{}@{version_text}", segments.join("/")), at))
    }

    fn label(&mut self) -> Result<Option<String>> {
        if self.current().kind != TokenKind::String {
            return Ok(None);
        }
        let token = self.consume();
        let Some(TokenValue::Text(label)) = token.value else {
            unreachable!()
        };
        self.expect(":")?;
        Ok(Some(label))
    }

    fn ast(
        &mut self,
        kind: &str,
        token: Option<&str>,
        start: Point,
        end: Point,
        attributes: Vec<(String, Value)>,
        children: Vec<AstNode>,
    ) -> AstNode {
        self.ast_counter += 1;
        AstNode {
            id: format!("ast-{}", self.ast_counter),
            kind: kind.to_owned(),
            token: token.map(str::to_owned),
            children,
            span: TokenSpan {
                source: self.source_ref.clone(),
                start,
                end,
            },
            attributes,
        }
    }
    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }
    fn peek(&self) -> &Token {
        self.tokens.get(self.cursor + 1).unwrap_or(self.current())
    }
    fn matches(&self, text: &str) -> bool {
        self.current().text == text
    }
    fn consume(&mut self) -> Token {
        let token = self.current().clone();
        self.cursor += 1;
        token
    }
    fn expect(&mut self, text: &str) -> Result<Token> {
        if self.matches(text) {
            Ok(self.consume())
        } else {
            self.fail(
                "BHCP1001",
                format!("expected {text:?}, found {:?}", self.current().text),
            )
        }
    }
    fn identifier(&mut self, description: &str) -> Result<Token> {
        if self.current().kind == TokenKind::Identifier {
            Ok(self.consume())
        } else {
            self.fail(
                "BHCP1001",
                format!("expected {description}, found {:?}", self.current().text),
            )
        }
    }
    fn binder(&mut self, description: &str) -> Result<Token> {
        let token = self.identifier(description)?;
        if reserved_binder(&token.text) {
            return Err(at(
                "BHCP1001",
                format!(
                    "reserved spelling {:?} cannot be used as {description}",
                    token.text
                ),
                self.source_name,
                &token.start,
            ));
        }
        Ok(token)
    }
    fn goal_binder(&mut self, description: &str) -> Result<Token> {
        let token = self.identifier(description)?;
        if reserved_goal_binder(&token.text) {
            return Err(at(
                "BHCP1001",
                format!(
                    "reserved spelling {:?} cannot be used as {description}",
                    token.text
                ),
                self.source_name,
                &token.start,
            ));
        }
        Ok(token)
    }
    fn fail<T>(&self, code: &'static str, message: impl Into<String>) -> Result<T> {
        Err(at(code, message, self.source_name, &self.current().start))
    }
}

fn identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}
fn identifier_continue(character: char) -> bool {
    identifier_start(character) || character.is_ascii_digit() || character == '-'
}
fn reserved_binder(value: &str) -> bool {
    FIXED_BARE_WORDS.contains(&value)
        || matches!(
            value,
            "completed"
                | "derived"
                | "duration"
                | "exists"
                | "expect"
                | "faulted"
                | "float"
                | "forall"
                | "in"
                | "let"
                | "map"
                | "match"
                | "native"
                | "set"
                | "time"
        )
}
fn reserved_goal_binder(value: &str) -> bool {
    matches!(
        value,
        "Bool"
            | "Bytes"
            | "Decimal"
            | "Duration"
            | "Dynamic"
            | "Goal"
            | "Integer"
            | "List"
            | "Map"
            | "Meta"
            | "NetworkShape"
            | "Never"
            | "Option"
            | "Rational"
            | "Reduction"
            | "Result"
            | "Set"
            | "Text"
            | "Timestamp"
            | "Unit"
            | "borrow"
            | "borrowed"
            | "completed"
            | "else"
            | "exists"
            | "expect"
            | "false"
            | "faulted"
            | "float"
            | "for"
            | "forall"
            | "if"
            | "in"
            | "let"
            | "linear"
            | "map"
            | "match"
            | "move"
            | "owned"
            | "read"
            | "refuted"
            | "satisfied"
            | "set"
            | "share"
            | "shared"
            | "then"
            | "time"
            | "true"
            | "unit"
            | "unrestricted"
            | "unresolved"
            | "variant"
            | "when"
            | "where"
            | "write"
    )
}

fn governance_document_value(
    kind: &str,
    symbol: &str,
    extends: Option<&str>,
    fields: &[SurfaceMetaField],
) -> Value {
    let mut entries = vec![
        ("version".to_owned(), Value::Text("bhcp/v0".to_owned())),
        ("features".to_owned(), Value::Array(vec![])),
        ("kind".to_owned(), Value::Text(kind.to_owned())),
        ("symbol".to_owned(), Value::Text(symbol.to_owned())),
    ];
    if let Some(extends) = extends {
        entries.push(("extends".to_owned(), Value::Text(extends.to_owned())));
    }
    entries.extend(
        fields
            .iter()
            .map(|field| (field.name.clone(), field.value.clone())),
    );
    Value::owned_map(entries)
}

fn governance_field<'a>(fields: &'a [SurfaceMetaField], name: &str) -> &'a Value {
    &fields
        .iter()
        .find(|field| field.name == name)
        .expect("closed required governance field")
        .value
}

fn validate_waiver_source(
    symbol: &str,
    fields: &[SurfaceMetaField],
) -> std::result::Result<Option<WaiverDocument>, String> {
    for name in ["issuer", "justification"] {
        if !matches!(governance_field(fields, name), Value::Text(value) if !value.is_empty()) {
            return Err(format!("waiver field {name:?} must be a non-empty string"));
        }
    }
    let mut timestamps = Vec::new();
    for name in ["issued_at", "not_before", "expires_at"] {
        let Value::Tag(0, value) = governance_field(fields, name) else {
            return Err(format!("waiver field {name:?} must be a time string"));
        };
        let Value::Text(value) = value.as_ref() else {
            return Err(format!("waiver field {name:?} must be a time string"));
        };
        if value.is_empty() {
            return Err(format!(
                "waiver field {name:?} must be a non-empty time string"
            ));
        }
        validate_policy_timestamp(value, &format!("waiver {name}"))
            .map_err(|diagnostic| diagnostic.message)?;
        timestamps.push(value);
    }
    if !(timestamps[0] <= timestamps[1] && timestamps[1] < timestamps[2]) {
        return Err("waiver interval must satisfy issued_at <= not_before < expires_at".to_owned());
    }
    let Value::Array(targets) = governance_field(fields, "targets") else {
        return Err("waiver targets must be an array".to_owned());
    };
    if targets.is_empty() {
        return Err("waiver targets must be non-empty".to_owned());
    }
    for target in targets {
        WaiverTarget::from_value(target).map_err(|diagnostic| diagnostic.message)?;
    }
    validate_values_sorted_unique(targets, "waiver targets")
        .map_err(|diagnostic| diagnostic.message)?;
    let Value::Array(authorization) = governance_field(fields, "authorization") else {
        return Err("waiver authorization must be an array".to_owned());
    };
    if authorization.is_empty() {
        return Err("waiver authorization must be non-empty".to_owned());
    }
    let mut materialized = true;
    for value in authorization {
        materialized &= validate_authorization_source(value)?;
    }
    materialized &= validate_artifact_reference_source(
        governance_field(fields, "audit_reference"),
        "waiver audit_reference",
    )?;
    let delegations = match fields
        .iter()
        .find(|field| field.name == "authority_chain")
        .map(|field| &field.value)
    {
        Some(Value::Array(delegations)) => delegations.as_slice(),
        Some(_) => return Err("waiver authority_chain must be an array".to_owned()),
        None => &[],
    };
    let issuer = match governance_field(fields, "issuer") {
        Value::Text(issuer) => issuer.as_str(),
        _ => unreachable!(),
    };
    materialized &= validate_waiver_delegations(delegations, issuer)?;

    if !materialized {
        return Ok(None);
    }
    let value = Value::owned_map(vec![
        ("version".into(), Value::Text("bhcp/v0".into())),
        ("features".into(), Value::Array(vec![])),
        ("authorization".into(), Value::Array(authorization.clone())),
        ("kind".into(), Value::Text("waiver".into())),
        ("symbol".into(), Value::Text(symbol.to_owned())),
        ("targets".into(), Value::Array(targets.clone())),
        (
            "justification".into(),
            governance_field(fields, "justification").clone(),
        ),
        ("issuer".into(), governance_field(fields, "issuer").clone()),
        ("authority_chain".into(), Value::Array(delegations.to_vec())),
        (
            "issued_at".into(),
            governance_field(fields, "issued_at").clone(),
        ),
        (
            "not_before".into(),
            governance_field(fields, "not_before").clone(),
        ),
        (
            "expires_at".into(),
            governance_field(fields, "expires_at").clone(),
        ),
        (
            "audit_reference".into(),
            governance_field(fields, "audit_reference").clone(),
        ),
    ]);
    WaiverDocument::from_value(&value)
        .map(Some)
        .map_err(|diagnostic| diagnostic.message)
}

fn validate_authorization_source(value: &Value) -> std::result::Result<bool, String> {
    if matches!(value, Value::Text(symbol) if is_symbol(symbol)) {
        return Ok(false);
    }
    let Value::Map(entries) = value else {
        return Err("waiver authorization must be an authorization map or exact symbol".into());
    };
    validate_closed_map(
        entries,
        &["scheme", "issuer", "subject", "signature", "expires_at"],
        &["scheme", "issuer", "subject", "signature"],
        "waiver authorization",
    )?;
    if !matches!(map_value(entries, "scheme"), Some(Value::Text(value)) if is_symbol(value)) {
        return Err("waiver authorization scheme must be an exact symbol".into());
    }
    if !matches!(map_value(entries, "issuer"), Some(Value::Text(value)) if !value.is_empty()) {
        return Err("waiver authorization issuer must be non-empty".into());
    }
    validate_materialized_artifact_reference(
        map_value(entries, "subject").expect("required authorization subject"),
        "waiver authorization subject",
    )?;
    if !matches!(map_value(entries, "signature"), Some(Value::Bytes(_))) {
        return Err("waiver authorization signature must be a byte string".into());
    }
    if let Some(expires_at) = map_value(entries, "expires_at") {
        let Value::Tag(0, value) = expires_at else {
            return Err("waiver authorization expires_at must be a time string".into());
        };
        let Value::Text(value) = value.as_ref() else {
            return Err("waiver authorization expires_at must be a time string".into());
        };
        validate_policy_timestamp(value, "waiver authorization expires_at")
            .map_err(|diagnostic| diagnostic.message)?;
    }
    Ok(true)
}

fn validate_artifact_reference_source(
    value: &Value,
    context: &str,
) -> std::result::Result<bool, String> {
    if matches!(value, Value::Text(symbol) if is_symbol(symbol)) {
        Ok(false)
    } else {
        validate_materialized_artifact_reference(value, context)?;
        Ok(true)
    }
}

fn validate_materialized_artifact_reference(
    value: &Value,
    context: &str,
) -> std::result::Result<(), String> {
    ArtifactReference::from_value(value)
        .map(|_| ())
        .map_err(|diagnostic| format!("{context} is invalid: {}", diagnostic.message))
}

fn validate_waiver_delegations(
    delegations: &[Value],
    issuer: &str,
) -> std::result::Result<bool, String> {
    let mut materialized = true;
    let mut expected = None::<String>;
    let mut principals = std::collections::BTreeSet::new();
    for value in delegations {
        let Value::Map(entries) = value else {
            return Err("waiver delegation must be a map".into());
        };
        validate_closed_map(
            entries,
            &["delegator", "delegate", "authorization"],
            &["delegator", "delegate", "authorization"],
            "waiver delegation",
        )?;
        let delegator = text_map_value(entries, "delegator", "waiver delegation")?;
        let delegate = text_map_value(entries, "delegate", "waiver delegation")?;
        if delegator.is_empty() || delegate.is_empty() {
            return Err("waiver delegation principals must be non-empty".into());
        }
        if expected.as_deref().is_some_and(|value| value != delegator) {
            return Err("waiver authority chain is disconnected".into());
        }
        if expected.is_none() {
            principals.insert(delegator.to_owned());
        }
        if !principals.insert(delegate.to_owned()) {
            return Err("waiver authority chain repeats a principal".into());
        }
        materialized &= validate_artifact_reference_source(
            map_value(entries, "authorization").expect("required delegation authorization"),
            "waiver delegation authorization",
        )?;
        expected = Some(delegate.to_owned());
    }
    if expected.as_deref().is_some_and(|value| value != issuer) {
        return Err("waiver authority chain does not end at the issuer".into());
    }
    Ok(materialized)
}

fn validate_extension_source(
    symbol: &str,
    kind: SurfaceExtensionKind,
    fields: &[SurfaceMetaField],
) -> std::result::Result<Option<Value>, String> {
    let names = fields
        .iter()
        .map(|field| field.name.as_str())
        .collect::<Vec<_>>();
    if kind == SurfaceExtensionKind::Derived && names == ["lowering", "input", "output", "children"]
    {
        for field in fields {
            match (field.name.as_str(), &field.value) {
                ("lowering", Value::Text(value)) if is_symbol(value) => {}
                ("input" | "output", Value::Text(value))
                    if is_symbol(value)
                        || matches!(
                            value.as_str(),
                            "Bool"
                                | "Bytes"
                                | "Decimal"
                                | "Duration"
                                | "Integer"
                                | "Rational"
                                | "Text"
                                | "Timestamp"
                                | "Unit"
                        ) => {}
                ("children", Value::Array(children))
                    if children
                        .iter()
                        .all(|value| matches!(value, Value::Text(symbol) if is_symbol(symbol))) => {
                }
                ("children", _) => {
                    return Err(
                        "derived extension children must be an array of exact symbols".into(),
                    );
                }
                _ => {
                    return Err(format!(
                        "invalid legacy derived extension field {:?}",
                        field.name
                    ));
                }
            }
        }
        return Ok(None);
    }

    let expected = match kind {
        SurfaceExtensionKind::Derived => [
            "lowering",
            "type_rule",
            "effect_rule",
            "policy_rule",
            "normalization_rule",
            "evidence_rule",
        ]
        .as_slice(),
        SurfaceExtensionKind::Native => [
            "payload_schema",
            "type_rule",
            "effect_rule",
            "policy_rule",
            "normalization_rule",
            "evidence_rule",
        ]
        .as_slice(),
    };
    if names != expected {
        return Err(format!(
            "{} extension must declare exactly the wire descriptor fields",
            kind.as_str()
        ));
    }
    let first = governance_field(fields, expected[0]);
    match kind {
        SurfaceExtensionKind::Derived if !matches!(first, Value::Text(lowering) if is_symbol(lowering)) =>
        {
            return Err("derived extension lowering must be an exact symbol".into());
        }
        SurfaceExtensionKind::Native => {
            validate_materialized_artifact_reference(first, "native extension payload_schema")?;
        }
        _ => {}
    }
    for name in &expected[1..] {
        validate_materialized_artifact_reference(
            governance_field(fields, name),
            &format!("extension {name}"),
        )?;
    }
    let mut entries = vec![
        ("version".to_owned(), Value::Text("bhcp/v0".into())),
        ("features".to_owned(), Value::Array(vec![])),
        (
            "kind".to_owned(),
            Value::Text("extension-descriptor".into()),
        ),
        ("symbol".to_owned(), Value::Text(symbol.to_owned())),
        (
            "extension_kind".to_owned(),
            Value::Text(kind.as_str().into()),
        ),
        (
            "must_understand".to_owned(),
            Value::Bool(kind == SurfaceExtensionKind::Native),
        ),
    ];
    entries.extend(
        fields
            .iter()
            .map(|field| (field.name.clone(), field.value.clone())),
    );
    let descriptor = Value::owned_map(entries);
    crate::schema::validate_root(&descriptor, "extension-descriptor")
        .map_err(|diagnostic| diagnostic.message)?;
    Ok(Some(descriptor))
}

fn validate_meta_map_order(entries: &[(String, Value)]) -> std::result::Result<(), String> {
    let has = |name: &str| entries.iter().any(|(key, _)| key == name);
    let expected: Option<&[&str]> = if has("category") && has("operation") {
        Some(&["category", "operation", "value", "from", "to"])
    } else if has("rule") && has("weakening") {
        Some(&["rule", "scope", "weakening"])
    } else if has("delegator") || has("delegate") {
        Some(&["delegator", "delegate", "authorization"])
    } else if has("scheme") || has("subject") || has("signature") {
        Some(&["scheme", "issuer", "subject", "signature", "expires_at"])
    } else if has("media_type") || has("digests") || has("locations") {
        Some(&["media_type", "size", "digests", "locations"])
    } else if has("algorithm") || has("digest") {
        Some(&["algorithm", "digest"])
    } else if has("dimension") || has("maximum") {
        Some(&["dimension", "unit", "maximum", "scope", "parameters"])
    } else if has("effect") {
        Some(&["effect", "scope", "parameters"])
    } else if has("obligation") || has("classes") || has("minimum") {
        Some(&["obligation", "classes", "minimum", "scope", "parameters"])
    } else if has("requirement") {
        Some(&["requirement", "scope", "parameters"])
    } else if entries
        .iter()
        .all(|(key, _)| matches!(key.as_str(), "goals" | "resources" | "operations"))
    {
        Some(&["goals", "resources", "operations"])
    } else if has("canonical") || has("surface") {
        Some(&["category", "canonical", "surface"])
    } else if has("indent_width") || has("line_width") || has("final_newline") {
        Some(&["indent_width", "line_width", "final_newline"])
    } else {
        None
    };
    let Some(expected) = expected else {
        return Ok(());
    };
    let positions = entries
        .iter()
        .map(|(key, _)| expected.iter().position(|candidate| candidate == key))
        .collect::<Option<Vec<_>>>();
    if positions.is_some_and(|positions| positions.windows(2).all(|pair| pair[0] < pair[1])) {
        Ok(())
    } else {
        Err("nested governance map field order is closed and canonical".into())
    }
}

fn validate_closed_map(
    entries: &[(String, Value)],
    order: &[&str],
    required: &[&str],
    context: &str,
) -> std::result::Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for (name, _) in entries {
        if !order.iter().any(|candidate| *candidate == name) {
            return Err(format!("unknown {context} field {name:?}"));
        }
        if !seen.insert(name.as_str()) {
            return Err(format!("duplicate {context} field {name:?}"));
        }
    }
    for name in required {
        if !seen.contains(name) {
            return Err(format!("{context} requires field {name:?}"));
        }
    }
    Ok(())
}

fn map_value<'a>(entries: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    entries
        .iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, value)| value)
}

fn text_map_value<'a>(
    entries: &'a [(String, Value)],
    name: &str,
    context: &str,
) -> std::result::Result<&'a str, String> {
    match map_value(entries, name) {
        Some(Value::Text(value)) => Ok(value),
        _ => Err(format!("{context} field {name:?} must be text")),
    }
}
fn policy_layer_name(layer: PolicyLayer) -> &'static str {
    match layer {
        PolicyLayer::Organization => "organization",
        PolicyLayer::Team => "team",
        PolicyLayer::Repository => "repository",
        PolicyLayer::User => "user",
    }
}
fn type_name(value: &SurfaceType) -> String {
    match value {
        SurfaceType::Primitive(name) | SurfaceType::Exact(name) => (*name).to_owned(),
        SurfaceType::Record(_) | SurfaceType::StructuralRecord { .. } => "record".to_owned(),
        SurfaceType::Parameter(name) => name.clone(),
        SurfaceType::Dynamic => "Dynamic".to_owned(),
        SurfaceType::Never => "Never".to_owned(),
        SurfaceType::Reduction(_) => "Reduction".to_owned(),
        SurfaceType::Meta { kind, .. } => format!("Meta<{kind}>"),
        SurfaceType::Nominal { symbol, .. } => symbol.clone(),
        SurfaceType::Tuple(_) => "tuple".to_owned(),
        SurfaceType::List(_) => "list".to_owned(),
        SurfaceType::Set(_) => "set".to_owned(),
        SurfaceType::Map { .. } => "map".to_owned(),
        SurfaceType::Option(_) => "Option".to_owned(),
        SurfaceType::Result { .. } => "Result".to_owned(),
        SurfaceType::Variant(_) => "variant".to_owned(),
        SurfaceType::Goal { .. } => "Goal".to_owned(),
        SurfaceType::Union(_) => "union".to_owned(),
        SurfaceType::Intersection(_) => "intersection".to_owned(),
        SurfaceType::Handle { ownership, .. } => format!("{ownership} handle"),
        SurfaceType::Refined { value_type, .. } => type_name(value_type),
    }
}

fn surface_type_value(value: &SurfaceType) -> Value {
    match value {
        SurfaceType::Primitive(name) => Value::Array(vec![
            Value::Text("primitive".to_owned()),
            Value::Text((*name).to_owned()),
        ]),
        SurfaceType::Exact(name) => Value::Array(vec![
            Value::Text("exact-number".to_owned()),
            Value::Text((*name).to_owned()),
        ]),
        SurfaceType::Record(fields) => Value::Array(vec![
            Value::Text("record".to_owned()),
            Value::Array(
                fields
                    .iter()
                    .map(|field| {
                        Value::map([
                            ("name", Value::Text(field.name.clone())),
                            ("optional", Value::Bool(false)),
                            ("type", surface_type_value(&field.value_type)),
                        ])
                    })
                    .collect(),
            ),
            Value::Bool(false),
        ]),
        SurfaceType::StructuralRecord { fields, open } => Value::Array(vec![
            Value::Text("record".to_owned()),
            Value::Array(
                fields
                    .iter()
                    .map(|field| {
                        Value::map([
                            ("name", Value::Text(field.name.clone())),
                            ("optional", Value::Bool(field.optional)),
                            ("type", surface_type_value(&field.value_type)),
                        ])
                    })
                    .collect(),
            ),
            Value::Bool(*open),
        ]),
        SurfaceType::Parameter(name) => Value::Array(vec![
            Value::Text("parameter".to_owned()),
            Value::Text(name.clone()),
        ]),
        SurfaceType::Dynamic => Value::Array(vec![Value::Text("dynamic".to_owned())]),
        SurfaceType::Never => Value::Array(vec![Value::Text("never".to_owned())]),
        SurfaceType::Reduction(output) => Value::Array(vec![
            Value::Text("reduction".to_owned()),
            surface_type_value(output),
        ]),
        SurfaceType::Meta {
            kind,
            input,
            output,
        } => Value::Array(vec![
            Value::Text("meta".to_owned()),
            Value::Text((*kind).to_owned()),
            surface_type_value(input),
            surface_type_value(output),
        ]),
        SurfaceType::Nominal { symbol, arguments } => Value::Array(vec![
            Value::Text("nominal".to_owned()),
            Value::Text(symbol.clone()),
            Value::Array(arguments.iter().map(surface_type_value).collect()),
        ]),
        SurfaceType::Tuple(members) => Value::Array(vec![
            Value::Text("tuple".to_owned()),
            Value::Array(members.iter().map(surface_type_value).collect()),
        ]),
        SurfaceType::List(element) => Value::Array(vec![
            Value::Text("list".to_owned()),
            surface_type_value(element),
        ]),
        SurfaceType::Set(element) => Value::Array(vec![
            Value::Text("set".to_owned()),
            surface_type_value(element),
        ]),
        SurfaceType::Map { key, value } => Value::Array(vec![
            Value::Text("map".to_owned()),
            surface_type_value(key),
            surface_type_value(value),
        ]),
        SurfaceType::Option(element) => Value::Array(vec![
            Value::Text("option".to_owned()),
            surface_type_value(element),
        ]),
        SurfaceType::Result { ok, error } => Value::Array(vec![
            Value::Text("result".to_owned()),
            surface_type_value(ok),
            surface_type_value(error),
        ]),
        SurfaceType::Variant(cases) => Value::Array(vec![
            Value::Text("variant".to_owned()),
            Value::Array(
                cases
                    .iter()
                    .map(|case| {
                        Value::map([
                            ("name", Value::Text(case.name.clone())),
                            (
                                "payload",
                                Value::Array(case.payload.iter().map(surface_type_value).collect()),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ]),
        SurfaceType::Goal {
            input,
            output,
            effects,
            evidence,
        } => Value::Array(vec![
            Value::Text("goal".to_owned()),
            surface_type_value(input),
            surface_type_value(output),
            effects.as_ref().map_or(Value::Null, effect_row_value),
            evidence
                .as_ref()
                .map_or(Value::Null, |value| surface_type_value(value)),
        ]),
        SurfaceType::Union(members) => Value::Array(vec![
            Value::Text("union".to_owned()),
            Value::Array(members.iter().map(surface_type_value).collect()),
        ]),
        SurfaceType::Intersection(members) => Value::Array(vec![
            Value::Text("intersection".to_owned()),
            Value::Array(members.iter().map(surface_type_value).collect()),
        ]),
        SurfaceType::Handle {
            ownership,
            access,
            usage,
            lifetime,
            value_type,
        } => Value::Array(vec![
            Value::Text("handle".to_owned()),
            Value::Text(ownership.clone()),
            access.clone().map_or(Value::Null, Value::Text),
            usage.clone().map_or(Value::Null, Value::Text),
            lifetime.clone().map_or(Value::Null, Value::Text),
            surface_type_value(value_type),
        ]),
        SurfaceType::Refined {
            value_type,
            binder,
            predicate,
        } => Value::Array(vec![
            Value::Text("refinement".to_owned()),
            surface_type_value(value_type),
            Value::Text(binder.clone()),
            surface_expression_value(predicate),
        ]),
    }
}

fn effect_row_value(row: &SurfaceEffectRow) -> Value {
    Value::map([
        (
            "effects",
            Value::Array(row.effects.iter().cloned().map(Value::Text).collect()),
        ),
        ("tail", row.tail.clone().map_or(Value::Null, Value::Text)),
    ])
}

fn surface_effect_value(effect: &SurfaceEffect) -> Value {
    Value::map([
        ("symbol", Value::Text(effect.symbol.clone())),
        (
            "arguments",
            Value::Array(
                effect
                    .arguments
                    .iter()
                    .map(surface_expression_value)
                    .collect(),
            ),
        ),
    ])
}

fn surface_expression_value(expression: &SurfaceExpression) -> Value {
    match expression {
        SurfaceExpression::Literal { value, .. } => Value::Array(vec![
            Value::Text("literal".to_owned()),
            match value {
                SurfaceLiteral::Bool(value) => Value::Bool(*value),
                SurfaceLiteral::Integer(value) => Value::Integer(*value),
                SurfaceLiteral::Text(value) => Value::Text(value.clone()),
            },
        ]),
        SurfaceExpression::Reference { name, .. } => Value::Array(vec![
            Value::Text("reference".to_owned()),
            Value::Text(name.clone()),
        ]),
        SurfaceExpression::Unary {
            operator, operand, ..
        } => Value::Array(vec![
            Value::Text("unary".to_owned()),
            Value::Text(operator.clone()),
            surface_expression_value(operand),
        ]),
        SurfaceExpression::Binary {
            operator,
            left,
            right,
            ..
        } => Value::Array(vec![
            Value::Text("binary".to_owned()),
            Value::Text(operator.clone()),
            surface_expression_value(left),
            surface_expression_value(right),
        ]),
        SurfaceExpression::Call {
            function,
            arguments,
            ..
        } => Value::Array(vec![
            Value::Text("call".to_owned()),
            Value::Text(function.clone()),
            Value::Array(arguments.iter().map(surface_expression_value).collect()),
        ]),
        SurfaceExpression::If {
            condition,
            consequent,
            alternative,
            ..
        } => Value::Array(vec![
            Value::Text("if".to_owned()),
            surface_expression_value(condition),
            surface_expression_value(consequent),
            surface_expression_value(alternative),
        ]),
    }
}

fn type_parameters_value(names: &[String], bounds: &[Option<SurfaceType>]) -> Value {
    Value::Array(
        names
            .iter()
            .zip(bounds)
            .map(|(name, bound)| {
                Value::map([
                    ("name", Value::Text(name.clone())),
                    (
                        "bound",
                        bound.as_ref().map_or(Value::Null, surface_type_value),
                    ),
                ])
            })
            .collect(),
    )
}

fn definition_attributes(
    symbol: &str,
    type_parameters: &[String],
    type_parameter_bounds: &[Option<SurfaceType>],
    parameters: &[SurfaceParameter],
    result: Option<&SurfaceType>,
    definition: Option<&SurfaceExpression>,
) -> Vec<(String, Value)> {
    let mut attributes = vec![
        ("symbol".to_owned(), Value::Text(symbol.to_owned())),
        (
            "type_parameters".to_owned(),
            type_parameters_value(type_parameters, type_parameter_bounds),
        ),
        (
            "parameters".to_owned(),
            Value::Array(
                parameters
                    .iter()
                    .map(|parameter| {
                        Value::map([
                            ("name", Value::Text(parameter.name.clone())),
                            ("type", surface_type_value(&parameter.value_type)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ];
    if let Some(result) = result {
        attributes.push(("result".to_owned(), surface_type_value(result)));
    }
    if let Some(definition) = definition {
        attributes.push((
            "definition".to_owned(),
            surface_expression_value(definition),
        ));
    }
    attributes
}

fn verifier_argument_value(argument: &SurfaceVerifierArgument) -> Value {
    let mode = match argument.mode {
        SurfaceArgumentMode::Value => "value",
        SurfaceArgumentMode::Move => "move",
        SurfaceArgumentMode::Borrow => "borrow",
        SurfaceArgumentMode::Share => "share",
    };
    Value::map([
        ("name", Value::Text(argument.name.clone())),
        ("mode", Value::Text(mode.to_owned())),
        ("value", surface_expression_value(&argument.value)),
    ])
}
fn operator_precedence(operator: &str) -> Option<u8> {
    Some(match operator {
        "||" => 1,
        "&&" => 2,
        "==" | "!=" => 3,
        "<" | "<=" | ">" | ">=" => 4,
        "+" | "-" => 5,
        "*" | "/" | "%" => 6,
        _ => return None,
    })
}
fn at(code: &'static str, message: impl Into<String>, source: &str, point: &Point) -> Diagnostic {
    Diagnostic::new(code, message, source, point.line, point.column)
}
fn at_fn(
    code: &'static str,
    message: impl Into<String>,
    source: &str,
    point: &Point,
) -> Diagnostic {
    at(code, message, source, point)
}
