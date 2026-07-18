use crate::diagnostic::{Diagnostic, Result};
use crate::model::{AstNode, ContentReference, Point, TokenSpan};
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
}

impl SurfaceExpression {
    pub fn at(&self) -> &Point {
        match self {
            Self::Literal { at, .. }
            | Self::Reference { at, .. }
            | Self::Unary { at, .. }
            | Self::Binary { at, .. } => at,
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
    },
}

#[derive(Clone, Debug)]
pub struct SurfaceGoal {
    pub symbol: String,
    pub clauses: Vec<SurfaceClause>,
    pub at: Point,
    pub ast: AstNode,
}

#[derive(Clone, Debug)]
pub struct ParsedProgram {
    pub goals: Vec<SurfaceGoal>,
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
        if "+-*/%!=<>".contains(current) || "{}();:,.@".contains(current) {
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
        let mut goals = Vec::new();
        while self.current().kind != TokenKind::Eof {
            if self.current().text != "§goal" {
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
            goals.push(self.goal()?);
        }
        if goals.is_empty() {
            return self.fail(
                "BHCP1001",
                "a canonical source file must contain at least one goal",
            );
        }
        let start = goals[0].ast.span.start.clone();
        let end = goals.last().unwrap().ast.span.end.clone();
        let children = goals.iter().map(|goal| goal.ast.clone()).collect();
        let ast = self.ast("program", None, start, end, vec![], children);
        Ok(ParsedProgram { goals, ast })
    }

    fn goal(&mut self) -> Result<SurfaceGoal> {
        let keyword = self.expect("§goal")?;
        let (symbol, _) = self.qualified_name()?;
        self.expect("{")?;
        let mut clauses = Vec::new();
        while !self.matches("}") {
            if self.current().kind == TokenKind::Eof {
                return Err(at(
                    "BHCP1001",
                    "unterminated goal block",
                    self.source_name,
                    &keyword.start,
                ));
            }
            clauses.push(self.clause()?);
        }
        let end = self.consume().end;
        let children = clauses.iter().map(|clause| clause.ast.clone()).collect();
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
            at: keyword.start,
            ast,
        })
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
                let value_type = self.value_type()?;
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
                        (
                            "type".to_owned(),
                            Value::Text(type_name(&value_type).to_owned()),
                        ),
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
                (
                    SurfaceClauseKind::Verify {
                        verifier: verifier.clone(),
                    },
                    vec![("verifier".to_owned(), Value::Text(verifier))],
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
        match (token.kind, token.value) {
            (TokenKind::Number, Some(TokenValue::Integer(value))) => {
                Ok(SurfaceExpression::Literal {
                    value: SurfaceLiteral::Integer(value),
                    at: token.start,
                })
            }
            (TokenKind::String, Some(TokenValue::Text(value))) => Ok(SurfaceExpression::Literal {
                value: SurfaceLiteral::Text(value),
                at: token.start,
            }),
            (TokenKind::Identifier, _) if token.text == "true" || token.text == "false" => {
                Ok(SurfaceExpression::Literal {
                    value: SurfaceLiteral::Bool(token.text == "true"),
                    at: token.start,
                })
            }
            (TokenKind::Identifier, _) => Ok(SurfaceExpression::Reference {
                name: token.text,
                at: token.start,
            }),
            (_, _) if token.text == "(" => {
                let value = self.expression(0)?;
                self.expect(")")?;
                Ok(value)
            }
            _ => Err(at(
                "BHCP1001",
                format!("expected expression, found {:?}", token.text),
                self.source_name,
                &token.start,
            )),
        }
    }

    fn value_type(&mut self) -> Result<SurfaceType> {
        let token = self.identifier("type")?;
        match token.text.as_str() {
            "Bool" => Ok(SurfaceType::Primitive("Bool")),
            "Text" => Ok(SurfaceType::Primitive("Text")),
            "Bytes" => Ok(SurfaceType::Primitive("Bytes")),
            "Unit" => Ok(SurfaceType::Primitive("Unit")),
            "Timestamp" => Ok(SurfaceType::Primitive("Timestamp")),
            "Duration" => Ok(SurfaceType::Primitive("Duration")),
            "Integer" => Ok(SurfaceType::Exact("Integer")),
            "Rational" => Ok(SurfaceType::Exact("Rational")),
            "Decimal" => Ok(SurfaceType::Exact("Decimal")),
            _ => Err(at(
                "BHCP1004",
                format!(
                    "type syntax {:?} is outside the implemented vertical slice",
                    token.text
                ),
                self.source_name,
                &token.start,
            )),
        }
    }

    fn qualified_name(&mut self) -> Result<(String, Point)> {
        let first = self.identifier("qualified name")?;
        let at = first.start.clone();
        let mut segments = vec![first.text];
        while self.matches("/") {
            self.consume();
            segments.push(self.identifier("name segment")?.text);
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
fn type_name(value: &SurfaceType) -> &str {
    match value {
        SurfaceType::Primitive(name) | SurfaceType::Exact(name) => name,
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
