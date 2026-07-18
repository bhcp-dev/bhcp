//! Dependency-free CBOR diagnostic-notation and v0 root-fixture validation.

use std::collections::HashSet;

use crate::diagnostic::{Diagnostic, Result};
use crate::model::is_symbol;
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
    Ok(())
}

pub fn validate_schema_inventory(schema: &str, expected_kinds: &[&str]) -> Result<()> {
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
    if !schema.contains("? \"body\": composition-node") {
        return Err(Diagnostic::plain(
            "BHCP5002",
            "CDDL declarative-goal body rule is missing",
        ));
    }
    Ok(())
}

fn required_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "canonical-ast" => &["profile", "root"],
        "semantic-ir" => &[
            "type_mode",
            "types",
            "predicates",
            "goals",
            "extensions",
            "entrypoints",
        ],
        "syntax" => &["symbol", "preamble", "mappings", "formatting"],
        "profile" => &["symbol", "syntax", "policy_overlays", "type_mode"],
        "policy" => &["symbol", "layer", "rules"],
        "waiver" => &[
            "symbol",
            "rules",
            "scope",
            "weakening",
            "justification",
            "issuer",
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
        "runtime-outcome" => &["goal", "outcome"],
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
