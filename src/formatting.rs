use crate::diagnostic::{Diagnostic, Result};
use crate::hash::HashAlgorithm;
use crate::model::{AstNode, is_symbol};
use crate::parser::{CANONICAL_PROFILE, normalize_source_for_formatting, scan_profile_preamble};
use crate::pipeline::parse_source_bytes_with_profile_registry_and_algorithm;
use crate::profile::{FormattingRules, ProfileRegistry, SyntaxDocument, SyntaxMappingCategory};

pub const CANONICAL_FORMATTING_RULES: FormattingRules = FormattingRules {
    indent_width: 4,
    line_width: 100,
    final_newline: true,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Lexeme {
    Token(String),
    LineComment(String),
    BlockComment(String),
}

/// Formats valid canonical or profile-selected source through its resolved presentation rules.
pub fn format_source_bytes_with_profile_registry(
    source: &[u8],
    source_name: &str,
    registry: &ProfileRegistry,
) -> Result<String> {
    format_source_bytes_with_profile_registry_and_algorithm(
        source,
        source_name,
        registry,
        HashAlgorithm::default(),
    )
}

pub fn format_source_bytes_with_profile_registry_and_algorithm(
    source: &[u8],
    source_name: &str,
    registry: &ProfileRegistry,
    algorithm: HashAlgorithm,
) -> Result<String> {
    let before = parse_source_bytes_with_profile_registry_and_algorithm(
        source,
        source_name,
        registry,
        algorithm,
    )?;
    let had_bom = source.starts_with(&[0xef, 0xbb, 0xbf]);
    let selected = scan_profile_preamble(source, source_name)?;
    let (canonical, syntax, rules) = if selected.profile == CANONICAL_PROFILE {
        (selected.canonical_source, None, CANONICAL_FORMATTING_RULES)
    } else {
        let resolved = registry.resolve(&selected.profile, algorithm)?;
        let canonical = normalize_source_for_formatting(
            &selected.canonical_source,
            source_name,
            &resolved.syntax,
        )?;
        let rules = resolved.syntax.formatting;
        (canonical, Some(resolved.syntax), rules)
    };

    let lexemes = layout_lexemes(&canonical, source_name)?;
    let canonical_tokens = token_texts(&lexemes);
    let mut formatted = String::new();
    if had_bom {
        formatted.push('\u{feff}');
    }
    if selected.had_preamble {
        formatted.push_str("#!bhcp-profile ");
        formatted.push_str(&selected.profile);
        formatted.push('\n');
    }
    formatted.push_str(&render_lexemes(&lexemes, syntax.as_ref(), rules));

    let after = parse_source_bytes_with_profile_registry_and_algorithm(
        formatted.as_bytes(),
        "<formatted-source>",
        registry,
        algorithm,
    )?;
    let formatted_selection = scan_profile_preamble(formatted.as_bytes(), "<formatted-source>")?;
    let formatted_canonical = if let Some(syntax) = syntax.as_ref() {
        normalize_source_for_formatting(
            &formatted_selection.canonical_source,
            "<formatted-source>",
            syntax,
        )?
    } else {
        formatted_selection.canonical_source
    };
    let formatted_lexemes = layout_lexemes(&formatted_canonical, "<formatted-source>")?;
    if canonical_tokens != token_texts(&formatted_lexemes)
        || !equivalent_ast(&before.root, &after.root)
    {
        return Err(format_error(
            "formatting-changed-canonical-token-stream",
            source_name,
        ));
    }
    Ok(formatted)
}

fn token_texts(lexemes: &[Lexeme]) -> Vec<&str> {
    lexemes
        .iter()
        .filter_map(|lexeme| match lexeme {
            Lexeme::Token(token) => Some(token.as_str()),
            Lexeme::LineComment(_) | Lexeme::BlockComment(_) => None,
        })
        .collect()
}

fn equivalent_ast(left: &AstNode, right: &AstNode) -> bool {
    left.kind == right.kind
        && left.token == right.token
        && left.attributes == right.attributes
        && left.children.len() == right.children.len()
        && left
            .children
            .iter()
            .zip(&right.children)
            .all(|(left, right)| equivalent_ast(left, right))
}

fn layout_lexemes(source: &str, source_name: &str) -> Result<Vec<Lexeme>> {
    let mut lexemes = Vec::new();
    let mut index = 0;
    while index < source.len() {
        let tail = &source[index..];
        let character = tail.chars().next().expect("non-empty formatter tail");
        if character.is_whitespace() || character == '\u{feff}' {
            index += character.len_utf8();
            continue;
        }
        if tail.starts_with("//") {
            let end = tail
                .find('\n')
                .map_or(source.len(), |offset| index + offset);
            lexemes.push(Lexeme::LineComment(
                source[index..end].trim_end().to_owned(),
            ));
            index = end;
            continue;
        }
        if tail.starts_with("/*") {
            let Some(relative_end) = tail.find("*/") else {
                return Err(format_error("unterminated-block-comment", source_name));
            };
            let end = index + relative_end + 2;
            lexemes.push(Lexeme::BlockComment(source[index..end].to_owned()));
            index = end;
            continue;
        }
        if character == '"' {
            let end = string_end(source, index, source_name)?;
            lexemes.push(Lexeme::Token(source[index..end].to_owned()));
            index = end;
            continue;
        }
        if character == '§' {
            let mut end = index + character.len_utf8();
            while let Some(next) = source[end..].chars().next() {
                if !identifier_continue(next) {
                    break;
                }
                end += next.len_utf8();
            }
            lexemes.push(Lexeme::Token(source[index..end].to_owned()));
            index = end;
            continue;
        }
        if identifier_start(character) {
            let identifier_end = take_while(source, index, identifier_continue);
            let candidate_end = take_while(source, index, symbol_character);
            let candidate = &source[index..candidate_end];
            let end = if is_symbol(candidate) {
                candidate_end
            } else {
                identifier_end
            };
            lexemes.push(Lexeme::Token(source[index..end].to_owned()));
            index = end;
            continue;
        }
        if character.is_ascii_digit() {
            let end = take_while(source, index, |next| next.is_ascii_digit());
            lexemes.push(Lexeme::Token(source[index..end].to_owned()));
            index = end;
            continue;
        }
        if ["<=", ">=", "==", "!=", "&&", "||"]
            .iter()
            .any(|operator| tail.starts_with(operator))
        {
            let operator = &source[index..index + 2];
            lexemes.push(Lexeme::Token(operator.to_owned()));
            index += 2;
            continue;
        }
        if "+-*/%!=<>{}[]();:,.@".contains(character) {
            lexemes.push(Lexeme::Token(character.to_string()));
            index += character.len_utf8();
            continue;
        }
        return Err(format_error(
            format!("unsupported-layout-character {character:?}"),
            source_name,
        ));
    }
    Ok(lexemes)
}

fn string_end(source: &str, start: usize, source_name: &str) -> Result<usize> {
    let mut escaped = false;
    for (offset, character) in source[start + 1..].char_indices() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Ok(start + 1 + offset + character.len_utf8());
        }
    }
    Err(format_error("unterminated-string", source_name))
}

fn take_while(source: &str, start: usize, predicate: impl Fn(char) -> bool) -> usize {
    let mut end = start;
    while let Some(character) = source[end..].chars().next() {
        if !predicate(character) {
            break;
        }
        end += character.len_utf8();
    }
    end
}

fn identifier_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

fn identifier_continue(character: char) -> bool {
    identifier_start(character) || character.is_ascii_digit() || character == '-'
}

fn symbol_character(character: char) -> bool {
    identifier_continue(character) || matches!(character, '/' | '.' | '@')
}

fn render_lexemes(
    lexemes: &[Lexeme],
    syntax: Option<&SyntaxDocument>,
    rules: FormattingRules,
) -> String {
    let mut writer = LayoutWriter::new(rules);
    let mut index = 0;
    while index < lexemes.len() {
        match &lexemes[index] {
            Lexeme::LineComment(comment) => writer.line_comment(comment),
            Lexeme::BlockComment(comment) => writer.block_comment(comment),
            Lexeme::Token(canonical) if canonical == "{" => {
                writer.open_block(&surface_token(canonical, syntax));
            }
            Lexeme::Token(canonical) if canonical == "}" => {
                writer.close_block(&surface_token(canonical, syntax));
            }
            Lexeme::Token(canonical) if canonical == ";" => {
                writer.token(&surface_token(canonical, syntax), TokenLayout::Attached);
                if let Some(Lexeme::LineComment(comment)) = lexemes.get(index + 1) {
                    writer.line_comment(comment);
                    index += 1;
                } else {
                    writer.newline();
                }
            }
            Lexeme::Token(canonical) => {
                writer.layout_token(canonical, &surface_token(canonical, syntax));
            }
        }
        index += 1;
    }
    writer.finish()
}

fn surface_token(canonical: &str, syntax: Option<&SyntaxDocument>) -> String {
    let Some(syntax) = syntax else {
        return canonical.to_owned();
    };
    let mapped = |category, coordinate: &str| {
        syntax
            .mappings
            .iter()
            .find(|mapping| mapping.category == category && mapping.canonical == coordinate)
            .map(|mapping| mapping.surface.as_str())
    };
    if let Some(keyword) = canonical.strip_prefix('§') {
        return format!(
            "{}{}",
            mapped(SyntaxMappingCategory::Sigil, "§").unwrap_or("§"),
            mapped(SyntaxMappingCategory::Keyword, keyword).unwrap_or(keyword)
        );
    }
    let category = match canonical {
        "{" | "(" | "[" => Some(SyntaxMappingCategory::OpenDelimiter),
        "}" | ")" | "]" => Some(SyntaxMappingCategory::CloseDelimiter),
        ";" => Some(SyntaxMappingCategory::Terminator),
        _ if is_symbol(canonical) => Some(SyntaxMappingCategory::Alias),
        _ => None,
    };
    category
        .and_then(|category| mapped(category, canonical))
        .unwrap_or(canonical)
        .to_owned()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TokenLayout {
    Word,
    Attached,
    Prefix,
    Operator,
}

struct LayoutWriter {
    output: String,
    rules: FormattingRules,
    indent: usize,
    line_width: usize,
    at_line_start: bool,
    after_close: bool,
}

impl LayoutWriter {
    fn new(rules: FormattingRules) -> Self {
        Self {
            output: String::new(),
            rules,
            indent: 0,
            line_width: 0,
            at_line_start: true,
            after_close: false,
        }
    }

    fn layout_token(&mut self, canonical: &str, surface: &str) {
        let layout = match canonical {
            ")" | "]" | "," | ":" | "." | "@" => TokenLayout::Attached,
            "(" | "[" => TokenLayout::Prefix,
            "+" | "-" | "*" | "/" | "%" | "!" | "=" | "<" | ">" | "<=" | ">=" | "==" | "!="
            | "&&" | "||" => TokenLayout::Operator,
            _ => TokenLayout::Word,
        };
        if self.after_close && !matches!(layout, TokenLayout::Attached) {
            self.newline();
        }
        match canonical {
            "," => {
                self.token(surface, TokenLayout::Attached);
                self.space();
            }
            ":" => {
                self.token(surface, TokenLayout::Attached);
                self.space();
            }
            "." | "/" | "@" => self.token(surface, TokenLayout::Attached),
            _ => self.token(surface, layout),
        }
        self.after_close = false;
    }

    fn open_block(&mut self, surface: &str) {
        self.token(surface, TokenLayout::Word);
        self.indent += 1;
        self.newline();
    }

    fn close_block(&mut self, surface: &str) {
        self.indent = self.indent.saturating_sub(1);
        self.newline();
        self.token(surface, TokenLayout::Attached);
        self.after_close = true;
    }

    fn line_comment(&mut self, comment: &str) {
        self.token(comment, TokenLayout::Word);
        self.newline();
    }

    fn block_comment(&mut self, comment: &str) {
        if !self.at_line_start {
            self.newline();
        }
        self.token(comment, TokenLayout::Attached);
        self.newline();
    }

    fn token(&mut self, token: &str, layout: TokenLayout) {
        let existing_space = self.output.ends_with(' ');
        let wants_space = !self.at_line_start
            && matches!(layout, TokenLayout::Word | TokenLayout::Operator)
            && !self.output.ends_with([' ', '\n', '(', '[', '.', '/', '@']);
        let can_break = !self.at_line_start
            && matches!(layout, TokenLayout::Word | TokenLayout::Operator)
            && (wants_space || existing_space);
        let token_width = token.chars().count();
        if can_break
            && self.line_width + usize::from(wants_space) + token_width
                > usize::from(self.rules.line_width)
        {
            self.newline();
        } else if wants_space {
            self.space();
        }
        self.start_line();
        self.output.push_str(token);
        self.line_width += token_width;
        if layout == TokenLayout::Operator {
            self.space();
        }
    }

    fn start_line(&mut self) {
        if !self.at_line_start {
            return;
        }
        let spaces = self.indent * usize::from(self.rules.indent_width);
        self.output.extend(std::iter::repeat_n(' ', spaces));
        self.line_width = spaces;
        self.at_line_start = false;
    }

    fn space(&mut self) {
        if self.at_line_start || self.output.ends_with([' ', '\n']) {
            return;
        }
        self.output.push(' ');
        self.line_width += 1;
    }

    fn newline(&mut self) {
        while self.output.ends_with(' ') {
            self.output.pop();
        }
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.line_width = 0;
        self.at_line_start = true;
        self.after_close = false;
    }

    fn finish(mut self) -> String {
        while self.output.ends_with([' ', '\n']) {
            self.output.pop();
        }
        if self.rules.final_newline {
            self.output.push('\n');
        }
        self.output
    }
}

fn format_error(message: impl Into<String>, source_name: &str) -> Diagnostic {
    Diagnostic::new("BHCP9004", message, source_name, 1, 1)
}
