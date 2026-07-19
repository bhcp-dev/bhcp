use crate::diagnostic::{Diagnostic, Result};
use crate::model::{AstNode, ContentReference, Point, TokenSpan};
use crate::policy::{PolicyDocument, PolicyHeader, PolicyLayer, PolicyRule, SourcePolicyDocument};
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
    Operator,
    Punctuation,
    Eof,
}

#[derive(Clone, Debug)]
enum TokenValue {
    Integer(i64),
    Text(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
}

#[derive(Clone, Debug)]
pub struct SurfaceGoal {
    pub symbol: String,
    pub clauses: Vec<SurfaceClause>,
    pub body: Option<SurfaceComposition>,
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
    Compose {
        reducer: String,
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
            | Self::Compose { branches, .. } => branches,
        }
    }

    pub fn at(&self) -> &Point {
        match self {
            Self::DerivedAll { at, .. }
            | Self::DerivedAny { at, .. }
            | Self::DerivedNone { at, .. }
            | Self::Compose { at, .. } => at,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SurfaceBranch {
    pub tag: String,
    pub goal: String,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Debug)]
pub struct SurfaceFunction {
    pub symbol: String,
    pub type_parameters: Vec<String>,
    pub parameters: Vec<SurfaceParameter>,
    pub result: SurfaceType,
    pub definition: SurfaceExpression,
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
pub struct ParsedProgram {
    pub functions: Vec<SurfaceFunction>,
    pub goals: Vec<SurfaceGoal>,
    pub policies: Vec<SurfacePolicy>,
    pub ast: AstNode,
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

fn lex(source: &str, source_name: &str) -> Result<Vec<Token>> {
    if source
        .chars()
        .any(|character| !character.is_ascii() && character != '§')
    {
        return Err(Diagnostic::new(
            "BHCP0002",
            "dependency-free canonical source currently accepts ASCII plus the precomposed § sigil; unsupported Unicode is rejected",
            source_name,
            1,
            1,
        ));
    }
    let characters: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let (mut index, mut byte, mut line, mut column) = (0usize, 0usize, 1usize, 1usize);
    let point = |byte, line, column| Point { byte, line, column };
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
        if matches!(pair.as_str(), "<=" | ">=" | "==" | "!=" | "&&" | "||") {
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
        if "+-*/%!=<>".contains(current) || "{}[]();:,.@".contains(current) {
            let kind = if "+-*/%!=<>".contains(current) {
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
        let mut functions = Vec::new();
        let mut goals = Vec::new();
        let mut policies = Vec::new();
        let mut definitions = Vec::new();
        while self.current().kind != TokenKind::Eof {
            match self.current().text.as_str() {
                "§function" => {
                    let function = self.function()?;
                    definitions.push(function.ast.clone());
                    functions.push(function);
                }
                "§goal" => {
                    let goal = self.goal()?;
                    definitions.push(goal.ast.clone());
                    goals.push(goal);
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
                    definitions.push(policy.ast.clone());
                    policies.push(policy);
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
            }
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
            functions,
            goals,
            policies,
            ast,
        })
    }

    fn function(&mut self) -> Result<SurfaceFunction> {
        let keyword = self.expect("§function")?;
        let (symbol, _) = self.qualified_name()?;
        let mut type_parameters = Vec::new();
        if self.matches("<") {
            self.consume();
            loop {
                let parameter = self.identifier("type parameter")?;
                if type_parameters.contains(&parameter.text) {
                    return Err(at(
                        "BHCP1003",
                        "duplicate type parameter",
                        self.source_name,
                        &parameter.start,
                    ));
                }
                type_parameters.push(parameter.text);
                if !self.matches(",") {
                    break;
                }
                self.consume();
            }
            self.expect(">")?;
        }
        self.expect("(")?;
        let mut parameters = Vec::new();
        if !self.matches(")") {
            loop {
                let name = self.identifier("parameter name")?;
                self.expect(":")?;
                let value_type = self.value_type(&type_parameters)?;
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
            vec![("symbol".to_owned(), Value::Text(symbol.clone()))],
            vec![],
        );
        Ok(SurfaceFunction {
            symbol,
            type_parameters,
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

    fn meta_value(&mut self) -> Result<Value> {
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
                    values.push(self.meta_value()?);
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
            self.consume();
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
                    entries.push((key.text, self.meta_value()?));
                    if !self.matches(",") {
                        break;
                    }
                    self.consume();
                }
            }
            self.expect("}")?;
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
            .is_some_and(|token| token.text == "/" || token.text == ".")
            && self
                .tokens
                .get(cursor + 1)
                .is_some_and(|token| token.kind == TokenKind::Identifier)
        {
            if self.tokens[cursor].text == "/" {
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
        self.expect("{")?;
        let mut clauses = Vec::new();
        let mut body = None;
        let mut children = Vec::new();
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
                || self.matches("§compose")
            {
                if body.is_some() {
                    return self.fail(
                        "BHCP1004",
                        "multiple composition bodies are outside the implemented vertical slice",
                    );
                }
                let (composition, ast) = self.composition()?;
                children.push(ast);
                body = Some(composition);
            } else {
                let clause = self.clause()?;
                children.push(clause.ast.clone());
                clauses.push(clause);
            }
        }
        let end = self.consume().end;
        let ast = self.ast(
            "goal",
            Some("§goal"),
            keyword.start.clone(),
            end,
            vec![("symbol".to_owned(), Value::Text(symbol.clone()))],
            children,
        );
        Ok(SurfaceGoal {
            symbol,
            clauses,
            body,
            at: keyword.start,
            ast,
        })
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
        self.expect("{")?;
        let mut branches = Vec::new();
        while !self.matches("}") {
            if self.current().kind == TokenKind::Eof {
                return Err(at(
                    "BHCP1001",
                    "unterminated composition block",
                    self.source_name,
                    &keyword.start,
                ));
            }
            let tag = self.identifier("branch tag")?;
            self.expect("=")?;
            if self.current().kind == TokenKind::Keyword {
                return self.fail(
                    "BHCP1004",
                    "nested composition is outside the implemented vertical slice",
                );
            }
            let (goal, _) = self.qualified_name()?;
            self.expect("(")?;
            if !self.matches(")") {
                return self.fail(
                    "BHCP1004",
                    "goal-call arguments are outside the implemented composition slice",
                );
            }
            self.consume();
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
                vec![],
            );
            branches.push(SurfaceBranch {
                tag: tag.text,
                goal,
                at: tag.start,
                ast: branch_ast,
            });
        }
        self.consume();
        let end = self.expect(";")?.end;
        let ast = self.ast(
            if reducer.is_some() {
                "compose"
            } else if derived == "§any" {
                "any"
            } else if derived == "§none" {
                "none"
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
        } else {
            SurfaceComposition::DerivedAll {
                branches,
                at: keyword.start,
            }
        };
        Ok((composition, ast))
    }

    fn clause(&mut self) -> Result<SurfaceClause> {
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
            "§input" | "§output" => {
                let name = self.identifier("binding name")?.text;
                self.expect(":")?;
                let value_type = self.value_type(&[])?;
                let kind = if keyword.text == "§input" {
                    "input"
                } else {
                    "output"
                };
                (
                    SurfaceClauseKind::Fact {
                        kind,
                        name: name.clone(),
                        value_type: value_type.clone(),
                    },
                    vec![
                        ("name".to_owned(), Value::Text(name)),
                        ("type".to_owned(), Value::Text(type_name(&value_type))),
                    ],
                )
            }
            "§requires" | "§ensures" | "§limit" => {
                let kind = match keyword.text.as_str() {
                    "§requires" => "requires",
                    "§ensures" => "ensures",
                    _ => "limit",
                };
                (
                    SurfaceClauseKind::Contract {
                        kind,
                        condition: self.expression(0)?,
                    },
                    vec![],
                )
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
                (SurfaceClauseKind::Authority { kind, effects }, vec![])
            }
            "§prefer" => {
                let mut priority = 0;
                if self.current().kind == TokenKind::Number && self.peek().text == ":" {
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
                self.expect("with")?;
                let (verifier, _) = self.qualified_name()?;
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
                (
                    SurfaceClauseKind::Verify {
                        verifier: verifier.clone(),
                        obligation_labels,
                    },
                    attributes,
                )
            }
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
                    while self.matches("/") || self.matches(".") {
                        name.push_str(&self.consume().text);
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
            Some("/") | Some(".")
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
        if self.matches("{") {
            self.consume();
            let mut fields = Vec::new();
            while !self.matches("}") {
                let name = self.identifier("field name")?;
                self.expect(":")?;
                fields.push(SurfaceFieldType {
                    name: name.text,
                    value_type: self.value_type(parameters)?,
                });
                if !self.matches(",") {
                    break;
                }
                self.consume();
            }
            self.expect("}")?;
            return Ok(SurfaceType::Record(fields));
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
            name if parameters.iter().any(|parameter| parameter == name) => {
                SurfaceType::Parameter(name.to_owned())
            }
            _ => {
                return Err(at(
                    "BHCP1004",
                    format!(
                        "type syntax {:?} is outside the implemented vertical slice",
                        token.text
                    ),
                    self.source_name,
                    &token.start,
                ));
            }
        };
        Ok(value)
    }

    fn qualified_name(&mut self) -> Result<(String, Point)> {
        let first = self.identifier("qualified name")?;
        let at = first.start.clone();
        let mut segments = vec![first.text];
        while self.matches("/") || self.matches(".") {
            let separator = self.consume().text;
            let component = self.identifier("name segment")?.text;
            if separator == "/" {
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
        SurfaceType::Record(_) => "record".to_owned(),
        SurfaceType::Parameter(name) => name.clone(),
        SurfaceType::Dynamic => "Dynamic".to_owned(),
        SurfaceType::Reduction(_) => "Reduction".to_owned(),
        SurfaceType::Meta { kind, .. } => format!("Meta<{kind}>"),
    }
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
