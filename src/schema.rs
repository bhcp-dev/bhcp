//! CBOR diagnostic-notation, CDDL parsing, and v0 root-fixture validation.

use std::collections::HashSet;

use crate::diagnostic::{Diagnostic, Result};
use crate::model::is_symbol;
use crate::policy::{PolicyDocument, WaiverDocument};
use crate::profile::PresentationDocument;
use crate::value::Value;

pub fn parse_diagnostic(source: &str) -> Result<Value> {
    let mut parser = Parser {
        bytes: source.as_bytes(),
        cursor: 0,
    };
    let value = parser.value()?;
    parser.whitespace();
    if parser.cursor != parser.bytes.len() {
        return Err(parser.error("trailing diagnostic notation"));
    }
    Ok(value)
}

pub fn validate_root(value: &Value, expected_kind: &str) -> Result<()> {
    let Value::Map(entries) = value else {
        return Err(Diagnostic::plain("BHCP5002", "root fixture must be a map"));
    };
    require_text(value, "version", "bhcp/v0")?;
    require_array(value, "features")?;
    require_text(value, "kind", expected_kind)?;
    for field in required_fields(expected_kind) {
        if value.get(field).is_none() {
            return Err(Diagnostic::plain(
                "BHCP5002",
                format!("{expected_kind} fixture is missing {field}"),
            ));
        }
    }
    let mut keys = HashSet::new();
    for (key, _) in entries {
        if !keys.insert(key) {
            return Err(Diagnostic::plain("BHCP5002", "duplicate root key"));
        }
    }
    validate_hashes(value)?;
    if matches!(expected_kind, "syntax" | "profile") {
        PresentationDocument::from_value(value).map_err(|diagnostic| {
            Diagnostic::plain(
                "BHCP5002",
                format!("{expected_kind} fixture is invalid: {}", diagnostic.message),
            )
        })?;
    } else if expected_kind == "execution-result" {
        validate_execution_result(value)?;
    } else if expected_kind == "extension-descriptor" {
        validate_extension_descriptor(value)?;
    } else if expected_kind == "policy" {
        PolicyDocument::from_value(value).map_err(|diagnostic| {
            Diagnostic::plain(
                "BHCP5002",
                format!("policy fixture is invalid: {}", diagnostic.message),
            )
        })?;
    } else if expected_kind == "waiver" {
        WaiverDocument::from_value(value).map_err(|diagnostic| {
            Diagnostic::plain(
                "BHCP5002",
                format!("waiver fixture is invalid: {}", diagnostic.message),
            )
        })?;
    }
    Ok(())
}

pub fn validate_schema_inventory(schema: &str, expected_kinds: &[&str]) -> Result<()> {
    cddl::cddl_from_str(schema, false).map_err(|error| {
        Diagnostic::plain("BHCP5002", format!("CDDL schema does not parse: {error}"))
    })?;
    if !schema.contains("root-document = canonical-ast-document") {
        return Err(Diagnostic::plain(
            "BHCP5002",
            "CDDL root-document rule is missing",
        ));
    }
    for kind in expected_kinds {
        let rule = format!("{}-document", kind);
        if !schema.contains(&rule) {
            return Err(Diagnostic::plain(
                "BHCP5002",
                format!("CDDL is missing root rule {rule}"),
            ));
        }
    }
    if !schema.contains("? \"body\": kernel-network") {
        return Err(Diagnostic::plain(
            "BHCP5002",
            "CDDL declarative-goal kernel body rule is missing",
        ));
    }
    for rule in [
        "reduction = pending-reduction / concluded-reduction",
        "execution-result = completed-result / faulted-result",
        "verdict = satisfied-verdict / refuted-verdict / unresolved-verdict",
        "meta-type = [\"meta\", \"derived-form\" / \"network-shape\", type, type]",
        "derived-form-shape = {",
        "derived-child-shape = {\n  lowering-child-shape,\n  \"output\": type\n}",
        "network-shape = {",
        "\"required\": [1* tstr]",
    ] {
        if !schema.contains(rule) {
            return Err(Diagnostic::plain(
                "BHCP5002",
                format!("CDDL kernel rule is missing: {rule}"),
            ));
        }
    }
    for forbidden in [
        "? \"quantified\": quantified-family",
        "? \"parallel_eligible\": bool",
        "? \"parallel_reasons\": [* tstr]",
        "? \"budgets\": [* budget]",
        "\"rule\": symbol-id",
    ] {
        if schema.contains(forbidden) {
            return Err(Diagnostic::plain(
                "BHCP5002",
                format!("CDDL minimal-kernel boundary contains forbidden field: {forbidden}"),
            ));
        }
    }
    for rule in [
        "extension-descriptor-document = derived-extension-descriptor-document",
        "derived-extension-descriptor-document = {",
        "native-extension-descriptor-document = {",
    ] {
        if !schema.contains(rule) {
            return Err(Diagnostic::plain(
                "BHCP5002",
                format!("CDDL extension boundary is missing: {rule}"),
            ));
        }
    }
    Ok(())
}

fn required_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "canonical-ast" => &["profile", "root"],
        "semantic-ir" => &[
            "type_mode",
            "types",
            "functions",
            "predicates",
            "goals",
            "extensions",
            "entrypoints",
        ],
        "syntax" => &["symbol", "preamble", "mappings", "formatting"],
        "profile" => &["symbol", "syntax", "policy_overlays", "type_mode"],
        "policy" => &[],
        "waiver" => &[
            "authorization",
            "symbol",
            "targets",
            "justification",
            "issuer",
            "authority_chain",
            "issued_at",
            "not_before",
            "expires_at",
            "audit_reference",
        ],
        "extension-descriptor" => &[
            "symbol",
            "extension_kind",
            "must_understand",
            "type_rule",
            "effect_rule",
            "policy_rule",
            "normalization_rule",
            "evidence_rule",
        ],
        "obligation-graph" => &["semantic_ir", "nodes", "edges"],
        "capability-graph" => &["semantic_ir", "nodes", "edges"],
        "state-graph" => &["semantic_ir", "nodes", "edges", "transitions"],
        "execution-graph" => &["semantic_ir", "nodes", "edges", "entrypoints"],
        "evidence-bundle" => &[
            "semantic_ir",
            "execution_graph",
            "claims",
            "items",
            "gaps",
            "edges",
            "obligation_status",
        ],
        "execution-result" => &["goal", "result"],
        "planner-request" => &[
            "semantic_ir",
            "entrypoint",
            "input",
            "obligation_graph",
            "capability_graph",
            "state_graph",
            "budgets",
            "policy",
            "executors",
            "required_features",
        ],
        "planner-result" => &["request", "status"],
        "feature-manifest" => &[
            "implementation",
            "implementation_version",
            "documents",
            "features_supported",
            "native_extensions",
        ],
        "content-reference" => &["content"],
        _ => &[],
    }
}

fn require_text(value: &Value, field: &str, expected: &str) -> Result<()> {
    match value.get(field) {
        Some(Value::Text(actual)) if actual == expected => Ok(()),
        _ => Err(Diagnostic::plain(
            "BHCP5002",
            format!("field {field} must equal {expected:?}"),
        )),
    }
}

fn require_array(value: &Value, field: &str) -> Result<()> {
    match value.get(field) {
        Some(Value::Array(_)) => Ok(()),
        _ => Err(Diagnostic::plain(
            "BHCP5002",
            format!("field {field} must be an array"),
        )),
    }
}

fn validate_hashes(value: &Value) -> Result<()> {
    match value {
        Value::Array(items) => {
            for item in items {
                validate_hashes(item)?;
            }
        }
        Value::Map(entries) => {
            if let (Some(Value::Text(algorithm)), Some(Value::Bytes(digest))) =
                (value.get("algorithm"), value.get("digest"))
            {
                match algorithm.as_str() {
                    "bhcp.hash/sha3-512@0" if digest.len() != 64 => {
                        return Err(Diagnostic::plain(
                            "BHCP5002",
                            "sha3-512 digest must be 64 bytes",
                        ));
                    }
                    "bhcp.hash/sha3-512@0" => {}
                    _ if !is_symbol(algorithm) => {
                        return Err(Diagnostic::plain(
                            "BHCP5002",
                            "registered digest algorithm must be a symbol-id",
                        ));
                    }
                    _ => {}
                }
            }
            for (_, item) in entries {
                validate_hashes(item)?;
            }
        }
        Value::Tag(_, item) => validate_hashes(item)?,
        _ => {}
    }
    Ok(())
}

fn validate_execution_result(document: &Value) -> Result<()> {
    let Some(Value::Map(result)) = document.get("result") else {
        return Err(Diagnostic::plain(
            "BHCP5002",
            "execution-result result must be a map",
        ));
    };
    match map_text(result, "state") {
        Some("completed") => {
            let Some(Value::Map(verdict)) = map_get(result, "verdict") else {
                return Err(Diagnostic::plain(
                    "BHCP5002",
                    "completed execution requires a verdict",
                ));
            };
            match map_text(verdict, "state") {
                Some("satisfied") => {
                    require_present(verdict, "output", "satisfied verdict")?;
                    require_nonempty_array(verdict, "evidence", "satisfied verdict")
                }
                Some("refuted") => {
                    require_nonempty_array(verdict, "counter_evidence", "refuted verdict")
                }
                Some("unresolved") => {
                    require_map(verdict, "reason", "unresolved verdict")?;
                    require_array_in_map(verdict, "partial_evidence", "unresolved verdict")
                }
                _ => Err(Diagnostic::plain(
                    "BHCP5002",
                    "completed verdict state must be satisfied, refuted, or unresolved",
                )),
            }
        }
        Some("faulted") => {
            let fault = require_map(result, "fault", "faulted execution")?;
            require_map(fault, "error", "operational fault")?;
            require_array_in_map(fault, "trace", "operational fault")
        }
        _ => Err(Diagnostic::plain(
            "BHCP5002",
            "execution state must be completed or faulted",
        )),
    }
}

fn validate_extension_descriptor(document: &Value) -> Result<()> {
    match document.get("extension_kind") {
        Some(Value::Text(kind)) if kind == "derived" => {
            require_bool(document, "must_understand", false)?;
            match document.get("lowering") {
                Some(Value::Text(lowering)) if is_symbol(lowering) => {}
                _ => {
                    return Err(Diagnostic::plain(
                        "BHCP5002",
                        "derived extension requires a BHCP lowering function symbol",
                    ));
                }
            }
            if document.get("payload_schema").is_some() {
                return Err(Diagnostic::plain(
                    "BHCP5002",
                    "derived extension must not declare a native payload schema",
                ));
            }
            Ok(())
        }
        Some(Value::Text(kind)) if kind == "native" => {
            require_bool(document, "must_understand", true)?;
            match document.get("payload_schema") {
                Some(Value::Map(_)) => {}
                _ => {
                    return Err(Diagnostic::plain(
                        "BHCP5002",
                        "native extension requires a payload schema",
                    ));
                }
            }
            if document.get("lowering").is_some() {
                return Err(Diagnostic::plain(
                    "BHCP5002",
                    "native extension must not masquerade as a derived lowering",
                ));
            }
            Ok(())
        }
        _ => Err(Diagnostic::plain(
            "BHCP5002",
            "extension kind must be derived or native",
        )),
    }
}

fn require_bool(value: &Value, field: &str, expected: bool) -> Result<()> {
    match value.get(field) {
        Some(Value::Bool(actual)) if *actual == expected => Ok(()),
        _ => Err(Diagnostic::plain(
            "BHCP5002",
            format!("field {field} must equal {expected}"),
        )),
    }
}

fn map_get<'a>(entries: &'a [(String, Value)], field: &str) -> Option<&'a Value> {
    entries
        .iter()
        .find_map(|(key, value)| (key == field).then_some(value))
}

fn map_text<'a>(entries: &'a [(String, Value)], field: &str) -> Option<&'a str> {
    match map_get(entries, field) {
        Some(Value::Text(value)) => Some(value),
        _ => None,
    }
}

fn require_present(entries: &[(String, Value)], field: &str, context: &str) -> Result<()> {
    if map_get(entries, field).is_some() {
        Ok(())
    } else {
        Err(Diagnostic::plain(
            "BHCP5002",
            format!("{context} requires {field}"),
        ))
    }
}

fn require_map<'a>(
    entries: &'a [(String, Value)],
    field: &str,
    context: &str,
) -> Result<&'a [(String, Value)]> {
    match map_get(entries, field) {
        Some(Value::Map(value)) => Ok(value),
        _ => Err(Diagnostic::plain(
            "BHCP5002",
            format!("{context} requires map field {field}"),
        )),
    }
}

fn require_array_in_map(entries: &[(String, Value)], field: &str, context: &str) -> Result<()> {
    match map_get(entries, field) {
        Some(Value::Array(_)) => Ok(()),
        _ => Err(Diagnostic::plain(
            "BHCP5002",
            format!("{context} requires array field {field}"),
        )),
    }
}

fn require_nonempty_array(entries: &[(String, Value)], field: &str, context: &str) -> Result<()> {
    match map_get(entries, field) {
        Some(Value::Array(values)) if !values.is_empty() => Ok(()),
        _ => Err(Diagnostic::plain(
            "BHCP5002",
            format!("{context} requires non-empty array field {field}"),
        )),
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl Parser<'_> {
    fn value(&mut self) -> Result<Value> {
        self.whitespace();
        match self.peek() {
            Some(b'{') => self.map(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Value::Text(self.string()?)),
            Some(b'h') if self.bytes.get(self.cursor + 1) == Some(&b'\'') => self.bytes_value(),
            Some(b't') => {
                self.word(b"true")?;
                Ok(Value::Bool(true))
            }
            Some(b'f') => {
                self.word(b"false")?;
                Ok(Value::Bool(false))
            }
            Some(b'n') => {
                self.word(b"null")?;
                Ok(Value::Null)
            }
            Some(b'-' | b'0'..=b'9') => self.number_or_tag(),
            _ => Err(self.error("expected diagnostic value")),
        }
    }

    fn map(&mut self) -> Result<Value> {
        self.expect(b'{')?;
        let mut entries = Vec::new();
        self.whitespace();
        while self.peek() != Some(b'}') {
            let key = self.string()?;
            self.whitespace();
            self.expect(b':')?;
            let value = self.value()?;
            entries.push((key, value));
            self.whitespace();
            if self.peek() != Some(b',') {
                break;
            }
            self.cursor += 1;
            self.whitespace();
        }
        self.expect(b'}')?;
        let mut seen = HashSet::new();
        if entries.iter().any(|(key, _)| !seen.insert(key.clone())) {
            return Err(self.error("duplicate diagnostic map key"));
        }
        Ok(Value::owned_map(entries))
    }

    fn array(&mut self) -> Result<Value> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.whitespace();
        while self.peek() != Some(b']') {
            items.push(self.value()?);
            self.whitespace();
            if self.peek() != Some(b',') {
                break;
            }
            self.cursor += 1;
            self.whitespace();
        }
        self.expect(b']')?;
        Ok(Value::Array(items))
    }

    fn string(&mut self) -> Result<String> {
        self.whitespace();
        self.expect(b'"')?;
        let mut output = String::new();
        while let Some(byte) = self.peek() {
            self.cursor += 1;
            match byte {
                b'"' => return Ok(output),
                b'\\' => {
                    let escaped = self
                        .peek()
                        .ok_or_else(|| self.error("unterminated string escape"))?;
                    self.cursor += 1;
                    output.push(match escaped {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{0008}',
                        b'f' => '\u{000c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        _ => return Err(self.error("unsupported diagnostic string escape")),
                    });
                }
                0..=31 => return Err(self.error("control byte in diagnostic string")),
                byte if byte.is_ascii() => output.push(char::from(byte)),
                _ => return Err(self.error(
                    "non-ASCII diagnostic string is outside this dependency-free fixture subset",
                )),
            }
        }
        Err(self.error("unterminated diagnostic string"))
    }

    fn bytes_value(&mut self) -> Result<Value> {
        self.expect(b'h')?;
        self.expect(b'\'')?;
        let start = self.cursor;
        while self.peek().is_some_and(|byte| byte != b'\'') {
            self.cursor += 1;
        }
        let hex = self
            .bytes
            .get(start..self.cursor)
            .ok_or_else(|| self.error("invalid byte string"))?;
        self.expect(b'\'')?;
        if hex.len() % 2 != 0 {
            return Err(self.error("byte string needs an even number of hex digits"));
        }
        let mut bytes = Vec::with_capacity(hex.len() / 2);
        for pair in hex.chunks_exact(2) {
            bytes.push(
                (nibble(pair[0]).ok_or_else(|| self.error("invalid hex digit"))? << 4)
                    | nibble(pair[1]).ok_or_else(|| self.error("invalid hex digit"))?,
            );
        }
        Ok(Value::Bytes(bytes))
    }

    fn number_or_tag(&mut self) -> Result<Value> {
        let start = self.cursor;
        if self.peek() == Some(b'-') {
            self.cursor += 1;
        }
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.cursor += 1;
        }
        let number: i64 = std::str::from_utf8(&self.bytes[start..self.cursor])
            .unwrap()
            .parse()
            .map_err(|_| self.error("diagnostic integer exceeds i64"))?;
        self.whitespace();
        if number >= 0 && self.peek() == Some(b'(') {
            self.cursor += 1;
            let value = self.value()?;
            self.whitespace();
            self.expect(b')')?;
            Ok(Value::Tag(number as u64, Box::new(value)))
        } else {
            Ok(Value::Integer(number))
        }
    }

    fn whitespace(&mut self) {
        while self.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
            self.cursor += 1;
        }
    }
    fn word(&mut self, expected: &[u8]) -> Result<()> {
        if self.bytes.get(self.cursor..self.cursor + expected.len()) == Some(expected) {
            self.cursor += expected.len();
            Ok(())
        } else {
            Err(self.error("unexpected diagnostic token"))
        }
    }
    fn expect(&mut self, expected: u8) -> Result<()> {
        self.whitespace();
        if self.peek() == Some(expected) {
            self.cursor += 1;
            Ok(())
        } else {
            Err(self.error(format!("expected {:?}", char::from(expected))))
        }
    }
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.cursor).copied()
    }
    fn error(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new("BHCP5001", message, "<diagnostic>", 1, self.cursor + 1)
    }
}

fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
