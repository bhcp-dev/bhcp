//! Strongly typed v0 syntax and profile artifacts.

use std::collections::HashSet;

use crate::cbor::{decode_deterministic, encode_deterministic};
use crate::diagnostic::{Diagnostic, Result};
use crate::hash::{HashAlgorithm, artifact_hash_with};
use crate::model::{HashId, is_symbol};
use crate::policy::TypeMode;
use crate::value::Value;

const INVALID_PROFILE: &str = "BHCP9001";
const PROFILE_PREAMBLE: &str = "#!bhcp-profile";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresentationHeader {
    pub features: Vec<String>,
    pub semantic_id: Option<HashId>,
    pub artifact_id: Option<HashId>,
    pub provenance: Option<Value>,
    pub authorization: Option<Vec<Value>>,
}

impl PresentationHeader {
    fn from_entries(entries: &[(String, Value)], context: &str) -> Result<Self> {
        require_exact_text(entries, "version", "bhcp/v0", context)?;
        let features = parse_symbol_array(
            required(entries, "features", context)?,
            &format!("{context} features"),
            true,
        )?;
        let semantic_id = optional(entries, "semantic_id")
            .map(parse_hash_id)
            .transpose()?;
        let artifact_id = optional(entries, "artifact_id")
            .map(parse_hash_id)
            .transpose()?;
        let provenance = optional(entries, "provenance").cloned();
        if provenance
            .as_ref()
            .is_some_and(|value| !matches!(value, Value::Map(_)))
        {
            return Err(invalid(format!("{context} provenance must be a map")));
        }
        let authorization = optional(entries, "authorization")
            .map(|value| {
                let values = array_values(value, &format!("{context} authorization"))?;
                if values.iter().any(|value| !matches!(value, Value::Map(_))) {
                    return Err(invalid(format!(
                        "{context} authorization entries must be maps"
                    )));
                }
                Ok(values.to_vec())
            })
            .transpose()?;
        let header = Self {
            features,
            semantic_id,
            artifact_id,
            provenance,
            authorization,
        };
        header.validate(context)?;
        Ok(header)
    }

    fn entries(&self, include_artifact_id: bool) -> Vec<(String, Value)> {
        let mut entries = vec![
            ("version".to_owned(), text("bhcp/v0")),
            (
                "features".to_owned(),
                Value::Array(self.features.iter().cloned().map(Value::Text).collect()),
            ),
        ];
        if let Some(semantic_id) = &self.semantic_id {
            entries.push(("semantic_id".to_owned(), semantic_id.to_value()));
        }
        if include_artifact_id && let Some(artifact_id) = &self.artifact_id {
            entries.push(("artifact_id".to_owned(), artifact_id.to_value()));
        }
        if let Some(provenance) = &self.provenance {
            entries.push(("provenance".to_owned(), provenance.clone()));
        }
        if let Some(authorization) = &self.authorization {
            entries.push((
                "authorization".to_owned(),
                Value::Array(authorization.clone()),
            ));
        }
        entries
    }

    fn validate(&self, context: &str) -> Result<()> {
        validate_sorted_symbols(&self.features, &format!("{context} features"), true)?;
        if let Some(semantic_id) = &self.semantic_id {
            semantic_id.validate().map_err(profile_error)?;
        }
        if let Some(artifact_id) = &self.artifact_id {
            artifact_id.validate().map_err(profile_error)?;
        }
        if self
            .provenance
            .as_ref()
            .is_some_and(|value| !matches!(value, Value::Map(_)))
        {
            return Err(invalid(format!("{context} provenance must be a map")));
        }
        if self
            .authorization
            .as_ref()
            .is_some_and(|values| values.iter().any(|value| !matches!(value, Value::Map(_))))
        {
            return Err(invalid(format!(
                "{context} authorization entries must be maps"
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SyntaxMappingCategory {
    Keyword,
    Sigil,
    OpenDelimiter,
    CloseDelimiter,
    Terminator,
    Alias,
}

impl SyntaxMappingCategory {
    fn parse(value: &Value) -> Result<Self> {
        match text_value(value, "syntax mapping category")? {
            "keyword" => Ok(Self::Keyword),
            "sigil" => Ok(Self::Sigil),
            "open-delimiter" => Ok(Self::OpenDelimiter),
            "close-delimiter" => Ok(Self::CloseDelimiter),
            "terminator" => Ok(Self::Terminator),
            "alias" => Ok(Self::Alias),
            category => Err(invalid(format!(
                "unknown syntax mapping category {category:?}"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Sigil => "sigil",
            Self::OpenDelimiter => "open-delimiter",
            Self::CloseDelimiter => "close-delimiter",
            Self::Terminator => "terminator",
            Self::Alias => "alias",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxMapping {
    pub category: SyntaxMappingCategory,
    pub canonical: String,
    pub surface: String,
}

impl SyntaxMapping {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "syntax mapping")?;
        ensure_fields(
            entries,
            &["category", "canonical", "surface"],
            "syntax mapping",
        )?;
        let category =
            SyntaxMappingCategory::parse(required(entries, "category", "syntax mapping")?)?;
        let canonical = required_text(entries, "canonical", "syntax mapping")?;
        let surface = required_text(entries, "surface", "syntax mapping")?;
        let mapping = Self {
            category,
            canonical,
            surface,
        };
        mapping.validate()?;
        Ok(mapping)
    }

    fn to_value(&self) -> Value {
        Value::map([
            ("category", text(self.category.as_str())),
            ("canonical", text(&self.canonical)),
            ("surface", text(&self.surface)),
        ])
    }

    fn validate(&self) -> Result<()> {
        if self.canonical.is_empty() {
            return Err(invalid("syntax mapping canonical must not be empty"));
        }
        if self.surface.is_empty() {
            return Err(invalid("syntax mapping surface must not be empty"));
        }
        if self.category == SyntaxMappingCategory::Alias && !is_symbol(&self.canonical) {
            return Err(invalid(
                "alias syntax mapping canonical must be a symbol-id",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormattingRules {
    pub indent_width: u8,
    pub line_width: u16,
    pub final_newline: bool,
}

impl FormattingRules {
    fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "syntax formatting")?;
        ensure_fields(
            entries,
            &["indent_width", "line_width", "final_newline"],
            "syntax formatting",
        )?;
        let indent_width = integer_value(
            required(entries, "indent_width", "syntax formatting")?,
            "formatting indent_width",
        )?;
        let line_width = integer_value(
            required(entries, "line_width", "syntax formatting")?,
            "formatting line_width",
        )?;
        let final_newline = bool_value(
            required(entries, "final_newline", "syntax formatting")?,
            "formatting final_newline",
        )?;
        if !(0..=16).contains(&indent_width) {
            return Err(invalid("formatting indent_width must be in 0..=16"));
        }
        if !(1..=512).contains(&line_width) {
            return Err(invalid("formatting line_width must be in 1..=512"));
        }
        Ok(Self {
            indent_width: u8::try_from(indent_width).expect("validated indent width"),
            line_width: u16::try_from(line_width).expect("validated line width"),
            final_newline,
        })
    }

    fn to_value(self) -> Value {
        Value::map([
            ("indent_width", Value::Integer(i64::from(self.indent_width))),
            ("line_width", Value::Integer(i64::from(self.line_width))),
            ("final_newline", Value::Bool(self.final_newline)),
        ])
    }

    fn validate(self) -> Result<()> {
        if self.indent_width > 16 {
            return Err(invalid("formatting indent_width must be in 0..=16"));
        }
        if !(1..=512).contains(&self.line_width) {
            return Err(invalid("formatting line_width must be in 1..=512"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxDocument {
    pub header: PresentationHeader,
    pub symbol: String,
    pub extends: Option<String>,
    pub mappings: Vec<SyntaxMapping>,
    pub formatting: FormattingRules,
}

impl SyntaxDocument {
    fn from_entries(entries: &[(String, Value)]) -> Result<Self> {
        ensure_fields(
            entries,
            &[
                "version",
                "features",
                "semantic_id",
                "artifact_id",
                "provenance",
                "authorization",
                "kind",
                "symbol",
                "extends",
                "preamble",
                "mappings",
                "formatting",
            ],
            "syntax document",
        )?;
        require_exact_text(entries, "kind", "syntax", "syntax document")?;
        require_exact_text(entries, "preamble", PROFILE_PREAMBLE, "syntax document")?;
        let header = PresentationHeader::from_entries(entries, "syntax document")?;
        let symbol = required_symbol(entries, "symbol", "syntax document")?;
        let extends = optional(entries, "extends")
            .map(|value| symbol_value(value, "syntax document extends"))
            .transpose()?;
        let mappings = array_values(
            required(entries, "mappings", "syntax document")?,
            "syntax document mappings",
        )?
        .iter()
        .map(SyntaxMapping::from_value)
        .collect::<Result<Vec<_>>>()?;
        let formatting =
            FormattingRules::from_value(required(entries, "formatting", "syntax document")?)?;
        let document = Self {
            header,
            symbol,
            extends,
            mappings,
            formatting,
        };
        document.validate()?;
        Ok(document)
    }

    pub fn to_value(&self, include_artifact_id: bool) -> Value {
        let mut entries = self.header.entries(include_artifact_id);
        entries.extend([
            ("kind".to_owned(), text("syntax")),
            ("symbol".to_owned(), text(&self.symbol)),
        ]);
        if let Some(extends) = &self.extends {
            entries.push(("extends".to_owned(), text(extends)));
        }
        entries.extend([
            ("preamble".to_owned(), text(PROFILE_PREAMBLE)),
            (
                "mappings".to_owned(),
                Value::Array(self.mappings.iter().map(SyntaxMapping::to_value).collect()),
            ),
            ("formatting".to_owned(), self.formatting.to_value()),
        ]);
        Value::owned_map(entries)
    }

    pub fn validate(&self) -> Result<()> {
        self.header.validate("syntax document")?;
        validate_symbol(&self.symbol, "syntax document symbol")?;
        if let Some(extends) = &self.extends {
            validate_symbol(extends, "syntax document extends")?;
            if extends == &self.symbol {
                return Err(invalid("syntax document must not extend itself"));
            }
        }
        self.formatting.validate()?;
        for mapping in &self.mappings {
            mapping.validate()?;
        }
        let mut coordinates = HashSet::new();
        for mapping in &self.mappings {
            if !coordinates.insert((mapping.category, mapping.canonical.as_str())) {
                return Err(invalid("duplicate syntax mapping coordinate"));
            }
        }
        if !self.mappings.windows(2).all(|pair| {
            (pair[0].category, pair[0].canonical.as_str())
                < (pair[1].category, pair[1].canonical.as_str())
        }) {
            return Err(invalid(
                "syntax mappings must be sorted by category and canonical coordinate",
            ));
        }
        validate_artifact_id(
            &self.header,
            &self.to_value(false),
            "syntax artifact_id does not match document",
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileDocument {
    pub header: PresentationHeader,
    pub symbol: String,
    pub extends: Option<String>,
    pub syntax: String,
    pub policy_overlays: Vec<String>,
    pub type_mode: TypeMode,
}

impl ProfileDocument {
    fn from_entries(entries: &[(String, Value)]) -> Result<Self> {
        ensure_fields(
            entries,
            &[
                "version",
                "features",
                "semantic_id",
                "artifact_id",
                "provenance",
                "authorization",
                "kind",
                "symbol",
                "extends",
                "syntax",
                "policy_overlays",
                "type_mode",
            ],
            "profile document",
        )?;
        require_exact_text(entries, "kind", "profile", "profile document")?;
        let header = PresentationHeader::from_entries(entries, "profile document")?;
        let symbol = required_symbol(entries, "symbol", "profile document")?;
        let extends = optional(entries, "extends")
            .map(|value| symbol_value(value, "profile document extends"))
            .transpose()?;
        let syntax = required_symbol(entries, "syntax", "profile document")?;
        let policy_overlays = parse_symbol_list(
            required(entries, "policy_overlays", "profile document")?,
            "profile policy_overlays",
        )?;
        let type_mode = parse_type_mode(required(entries, "type_mode", "profile document")?)?;
        let document = Self {
            header,
            symbol,
            extends,
            syntax,
            policy_overlays,
            type_mode,
        };
        document.validate()?;
        Ok(document)
    }

    pub fn to_value(&self, include_artifact_id: bool) -> Value {
        let mut entries = self.header.entries(include_artifact_id);
        entries.extend([
            ("kind".to_owned(), text("profile")),
            ("symbol".to_owned(), text(&self.symbol)),
        ]);
        if let Some(extends) = &self.extends {
            entries.push(("extends".to_owned(), text(extends)));
        }
        entries.extend([
            ("syntax".to_owned(), text(&self.syntax)),
            (
                "policy_overlays".to_owned(),
                Value::Array(
                    self.policy_overlays
                        .iter()
                        .cloned()
                        .map(Value::Text)
                        .collect(),
                ),
            ),
            ("type_mode".to_owned(), text(self.type_mode.as_str())),
        ]);
        Value::owned_map(entries)
    }

    pub fn validate(&self) -> Result<()> {
        self.header.validate("profile document")?;
        validate_symbol(&self.symbol, "profile document symbol")?;
        if let Some(extends) = &self.extends {
            validate_symbol(extends, "profile document extends")?;
            if extends == &self.symbol {
                return Err(invalid("profile document must not extend itself"));
            }
        }
        validate_symbol(&self.syntax, "profile document syntax")?;
        let mut overlays = HashSet::new();
        for overlay in &self.policy_overlays {
            validate_symbol(overlay, "profile policy overlay")?;
            if !overlays.insert(overlay) {
                return Err(invalid(
                    "profile policy_overlays must contain unique symbols",
                ));
            }
        }
        validate_artifact_id(
            &self.header,
            &self.to_value(false),
            "profile artifact_id does not match document",
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PresentationDocument {
    Syntax(SyntaxDocument),
    Profile(ProfileDocument),
}

impl PresentationDocument {
    pub fn from_value(value: &Value) -> Result<Self> {
        let entries = map_entries(value, "presentation document")?;
        match required_text(entries, "kind", "presentation document")?.as_str() {
            "syntax" => Ok(Self::Syntax(SyntaxDocument::from_entries(entries)?)),
            "profile" => Ok(Self::Profile(ProfileDocument::from_entries(entries)?)),
            kind => Err(invalid(format!(
                "presentation document kind must be syntax or profile, got {kind:?}"
            ))),
        }
    }

    pub fn from_cbor(bytes: &[u8]) -> Result<Self> {
        Self::from_value(&decode_deterministic(bytes)?)
    }

    pub fn to_value(&self, include_artifact_id: bool) -> Value {
        match self {
            Self::Syntax(document) => document.to_value(include_artifact_id),
            Self::Profile(document) => document.to_value(include_artifact_id),
        }
    }

    pub fn to_cbor(&self, include_artifact_id: bool) -> Result<Vec<u8>> {
        self.validate()?;
        encode_deterministic(&self.to_value(include_artifact_id))
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Syntax(document) => document.validate(),
            Self::Profile(document) => document.validate(),
        }
    }
}

fn parse_type_mode(value: &Value) -> Result<TypeMode> {
    match text_value(value, "profile type_mode")? {
        "dynamic" => Ok(TypeMode::Dynamic),
        "gradual" => Ok(TypeMode::Gradual),
        "infer-strict" => Ok(TypeMode::InferStrict),
        "strict" => Ok(TypeMode::Strict),
        _ => Err(invalid(
            "profile type_mode must be dynamic, gradual, infer-strict, or strict",
        )),
    }
}

fn validate_artifact_id(header: &PresentationHeader, value: &Value, mismatch: &str) -> Result<()> {
    let Some(materialized) = &header.artifact_id else {
        return Ok(());
    };
    let algorithm = HashAlgorithm::from_id(&materialized.algorithm).map_err(profile_error)?;
    let computed = artifact_hash_with(value, algorithm).map_err(profile_error)?;
    if &computed == materialized {
        Ok(())
    } else {
        Err(invalid(mismatch))
    }
}

fn parse_hash_id(value: &Value) -> Result<HashId> {
    let entries = map_entries(value, "profile hash ID")?;
    ensure_fields(entries, &["algorithm", "digest"], "profile hash ID")?;
    let hash = HashId {
        algorithm: required_symbol(entries, "algorithm", "profile hash ID")?,
        digest: bytes_value(
            required(entries, "digest", "profile hash ID")?,
            "profile hash digest",
        )?
        .to_vec(),
    };
    hash.validate().map_err(profile_error)?;
    Ok(hash)
}

fn parse_symbol_array(value: &Value, context: &str, allow_empty: bool) -> Result<Vec<String>> {
    let values = array_values(value, context)?
        .iter()
        .map(|value| symbol_value(value, context))
        .collect::<Result<Vec<_>>>()?;
    validate_sorted_symbols(&values, context, allow_empty)?;
    Ok(values)
}

fn parse_symbol_list(value: &Value, context: &str) -> Result<Vec<String>> {
    array_values(value, context)?
        .iter()
        .map(|value| symbol_value(value, context))
        .collect()
}

fn validate_sorted_symbols(values: &[String], context: &str, allow_empty: bool) -> Result<()> {
    if (!allow_empty && values.is_empty()) || !values.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(invalid(format!(
            "{context} must be a {}sorted set",
            if allow_empty { "" } else { "non-empty " }
        )));
    }
    for value in values {
        validate_symbol(value, context)?;
    }
    Ok(())
}

fn required_symbol(entries: &[(String, Value)], key: &str, context: &str) -> Result<String> {
    symbol_value(
        required(entries, key, context)?,
        &format!("{context} {key}"),
    )
}

fn symbol_value(value: &Value, context: &str) -> Result<String> {
    let value = text_value(value, context)?;
    validate_symbol(value, context)?;
    Ok(value.to_owned())
}

fn validate_symbol(value: &str, context: &str) -> Result<()> {
    if is_symbol(value) {
        Ok(())
    } else {
        Err(invalid(format!("{context} must be a symbol-id")))
    }
}

fn ensure_fields(entries: &[(String, Value)], allowed: &[&str], context: &str) -> Result<()> {
    let mut seen = HashSet::new();
    for (key, _) in entries {
        if !seen.insert(key.as_str()) {
            return Err(invalid(format!("duplicate {context} field {key:?}")));
        }
        if !allowed.contains(&key.as_str()) {
            return Err(invalid(format!("unknown {context} field {key:?}")));
        }
    }
    Ok(())
}

fn map_entries<'a>(value: &'a Value, context: &str) -> Result<&'a [(String, Value)]> {
    match value {
        Value::Map(entries) => Ok(entries),
        _ => Err(invalid(format!("{context} must be a map"))),
    }
}

fn array_values<'a>(value: &'a Value, context: &str) -> Result<&'a [Value]> {
    match value {
        Value::Array(values) => Ok(values),
        _ => Err(invalid(format!("{context} must be an array"))),
    }
}

fn required<'a>(entries: &'a [(String, Value)], key: &str, context: &str) -> Result<&'a Value> {
    optional(entries, key).ok_or_else(|| invalid(format!("{context} requires {key}")))
}

fn optional<'a>(entries: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    entries
        .iter()
        .find_map(|(candidate, value)| (candidate == key).then_some(value))
}

fn required_text(entries: &[(String, Value)], key: &str, context: &str) -> Result<String> {
    Ok(text_value(
        required(entries, key, context)?,
        &format!("{context} {key}"),
    )?
    .to_owned())
}

fn require_exact_text(
    entries: &[(String, Value)],
    key: &str,
    expected: &str,
    context: &str,
) -> Result<()> {
    if text_value(
        required(entries, key, context)?,
        &format!("{context} {key}"),
    )? == expected
    {
        Ok(())
    } else {
        Err(invalid(format!("{context} {key} must equal {expected:?}")))
    }
}

fn text_value<'a>(value: &'a Value, context: &str) -> Result<&'a str> {
    match value {
        Value::Text(value) => Ok(value),
        _ => Err(invalid(format!("{context} must be text"))),
    }
}

fn integer_value(value: &Value, context: &str) -> Result<i64> {
    match value {
        Value::Integer(value) => Ok(*value),
        _ => Err(invalid(format!("{context} must be an integer"))),
    }
}

fn bool_value(value: &Value, context: &str) -> Result<bool> {
    match value {
        Value::Bool(value) => Ok(*value),
        _ => Err(invalid(format!("{context} must be Boolean"))),
    }
}

fn bytes_value<'a>(value: &'a Value, context: &str) -> Result<&'a [u8]> {
    match value {
        Value::Bytes(value) => Ok(value),
        _ => Err(invalid(format!("{context} must be bytes"))),
    }
}

fn text(value: impl Into<String>) -> Value {
    Value::Text(value.into())
}

fn invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::plain(INVALID_PROFILE, message)
}

fn profile_error(diagnostic: Diagnostic) -> Diagnostic {
    invalid(diagnostic.message)
}
