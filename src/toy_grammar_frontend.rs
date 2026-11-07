//! Tiny toolchain-free grammar tokenizer probe.
//!
//! This module is intentionally narrow. It proves the first non-JSON grammar
//! boundary with a hand-written toy language without generalizing the JSON
//! Schema compiler or depending on Tree-sitter, ANTLR, Go, TypeScript, or
//! JavaScript tooling.

use std::error::Error;
use std::fmt;

use crate::grammar_ir::{
    GrammarField, GrammarFieldAlternative, GrammarIr, GrammarIrProvenance, GrammarRule,
    GrammarTerminalKind,
};

pub const TOY_GRAMMAR_LANGUAGE_ID: &str = "toy_assignment_v1";
pub const TOY_GRAMMAR_VERSION: u32 = 1;
pub const PROGRAM_START_TOKEN: u32 = 0;
pub const PROGRAM_END_TOKEN: u32 = 1;
pub const STATEMENT_TOKEN: u32 = 2;
pub const IDENTIFIER_END_TOKEN: u32 = 3;
pub const STRING_VALUE_TOKEN: u32 = 4;
pub const STRING_END_TOKEN: u32 = 5;
pub const INTEGER_VALUE_TOKEN: u32 = 6;
pub const TRUE_VALUE_TOKEN: u32 = 7;
pub const FALSE_VALUE_TOKEN: u32 = 8;
pub const STRUCTURAL_TOKEN_END: u32 = 32;
pub const BYTE_TOKEN_BASE: u32 = STRUCTURAL_TOKEN_END;
pub const BYTE_TOKEN_END: u32 = BYTE_TOKEN_BASE + 256;
pub const INTEGER_TOKEN_BASE: u32 = BYTE_TOKEN_END;
pub const INTEGER_TOKEN_END: u32 = INTEGER_TOKEN_BASE + 256;
pub const PAD_TOKEN: u32 = 1023;
pub const VOCAB_SIZE: u32 = PAD_TOKEN + 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToyProgram {
    pub statements: Vec<ToyStatement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToyStatement {
    pub identifier: String,
    pub value: ToyValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToyValue {
    Integer(u8),
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToyGrammarSpec {
    pub language_id: &'static str,
    pub version: u32,
    pub layout: ToyTokenLayout,
}

impl ToyGrammarSpec {
    pub fn assignment_language_v1() -> Self {
        Self {
            language_id: TOY_GRAMMAR_LANGUAGE_ID,
            version: TOY_GRAMMAR_VERSION,
            layout: ToyTokenLayout::default(),
        }
    }

    pub fn compile_tokenizer(&self) -> ToyGrammarTokenizer {
        ToyGrammarTokenizer { spec: self.clone() }
    }

    pub fn to_grammar_ir(&self) -> GrammarIr {
        let mut grammar = GrammarIr::new(
            self.language_id,
            GrammarIrProvenance {
                source_kind: "hand_written_toy_grammar".to_string(),
                source_path: Some("src/toy_grammar_frontend.rs".to_string()),
                source_sha256: None,
                metadata_path: None,
                metadata_sha256: None,
                parser: None,
            },
        );
        grammar.rules.push(GrammarRule {
            name: "program".to_string(),
            node_kind: "program".to_string(),
            fields: vec![GrammarField {
                name: "statements".to_string(),
                required: true,
                multiple: true,
                alternatives: vec![GrammarFieldAlternative {
                    node_kind: "assignment".to_string(),
                    terminal: GrammarTerminalKind::Nonterminal,
                }],
            }],
        });
        grammar.rules.push(GrammarRule {
            name: "assignment".to_string(),
            node_kind: "assignment".to_string(),
            fields: vec![
                GrammarField {
                    name: "name".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![GrammarFieldAlternative {
                        node_kind: "identifier".to_string(),
                        terminal: GrammarTerminalKind::IdentifierPayload,
                    }],
                },
                GrammarField {
                    name: "value".to_string(),
                    required: true,
                    multiple: false,
                    alternatives: vec![
                        GrammarFieldAlternative {
                            node_kind: "integer".to_string(),
                            terminal: GrammarTerminalKind::IntegerPayload,
                        },
                        GrammarFieldAlternative {
                            node_kind: "boolean".to_string(),
                            terminal: GrammarTerminalKind::BooleanPayload,
                        },
                        GrammarFieldAlternative {
                            node_kind: "string".to_string(),
                            terminal: GrammarTerminalKind::StringPayload,
                        },
                    ],
                },
            ],
        });
        grammar
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToyTokenLayout {
    pub vocab_size: u32,
    pub pad_token: u32,
    pub structural_tokens: TokenRange,
    pub utf8_byte_tokens: TokenRange,
    pub u8_integer_tokens: TokenRange,
}

impl Default for ToyTokenLayout {
    fn default() -> Self {
        Self {
            vocab_size: VOCAB_SIZE,
            pad_token: PAD_TOKEN,
            structural_tokens: TokenRange {
                start: 0,
                end_exclusive: STRUCTURAL_TOKEN_END,
            },
            utf8_byte_tokens: TokenRange {
                start: BYTE_TOKEN_BASE,
                end_exclusive: BYTE_TOKEN_END,
            },
            u8_integer_tokens: TokenRange {
                start: INTEGER_TOKEN_BASE,
                end_exclusive: INTEGER_TOKEN_END,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenRange {
    pub start: u32,
    pub end_exclusive: u32,
}

impl TokenRange {
    pub fn contains(&self, token: u32) -> bool {
        self.start <= token && token < self.end_exclusive
    }
}

#[derive(Debug, Clone)]
pub struct ToyGrammarTokenizer {
    spec: ToyGrammarSpec,
}

impl ToyGrammarTokenizer {
    pub fn spec(&self) -> &ToyGrammarSpec {
        &self.spec
    }

    pub fn parse_source(&self, source: &str) -> Result<ToyProgram, ToyGrammarError> {
        Parser::new(source).parse_program()
    }

    pub fn encode_source(&self, source: &str) -> Result<Vec<u32>, ToyGrammarError> {
        let program = self.parse_source(source)?;
        self.encode_program(&program)
    }

    pub fn encode_program(&self, program: &ToyProgram) -> Result<Vec<u32>, ToyGrammarError> {
        let mut tokens = vec![PROGRAM_START_TOKEN];
        for statement in &program.statements {
            tokens.push(STATEMENT_TOKEN);
            encode_bytes(
                statement.identifier.as_bytes(),
                IDENTIFIER_END_TOKEN,
                &mut tokens,
            )?;
            match &statement.value {
                ToyValue::Integer(value) => {
                    tokens.push(INTEGER_VALUE_TOKEN);
                    tokens.push(INTEGER_TOKEN_BASE + u32::from(*value));
                }
                ToyValue::Bool(true) => tokens.push(TRUE_VALUE_TOKEN),
                ToyValue::Bool(false) => tokens.push(FALSE_VALUE_TOKEN),
                ToyValue::String(value) => {
                    tokens.push(STRING_VALUE_TOKEN);
                    encode_bytes(value.as_bytes(), STRING_END_TOKEN, &mut tokens)?;
                }
            }
        }
        tokens.push(PROGRAM_END_TOKEN);
        if let Some(token) = tokens
            .iter()
            .find(|token| **token >= self.spec.layout.pad_token)
        {
            return Err(ToyGrammarError::new(
                "encode",
                "token_range",
                None,
                format!("token {token} collides with PAD or exceeds vocabulary"),
            ));
        }
        Ok(tokens)
    }

    pub fn decode_tokens(&self, tokens: &[u32]) -> Result<ToyProgram, ToyGrammarError> {
        let mut cursor = TokenCursor::new(tokens);
        cursor.expect(PROGRAM_START_TOKEN, "program_start")?;
        let mut statements = Vec::new();
        while cursor.peek() != Some(PROGRAM_END_TOKEN) {
            cursor.expect(STATEMENT_TOKEN, "statement")?;
            let identifier = decode_string_until(&mut cursor, IDENTIFIER_END_TOKEN, "identifier")?;
            if !is_identifier(&identifier) {
                return Err(ToyGrammarError::new(
                    "decode",
                    "identifier",
                    Some(cursor.position()),
                    format!("decoded identifier {identifier:?} is invalid"),
                ));
            }
            let value = match cursor.next("value_tag")? {
                INTEGER_VALUE_TOKEN => {
                    let token = cursor.next("integer_payload")?;
                    if !self.spec.layout.u8_integer_tokens.contains(token) {
                        return Err(ToyGrammarError::new(
                            "decode",
                            "integer_payload",
                            Some(cursor.position().saturating_sub(1)),
                            format!("integer token {token} is outside the u8 range"),
                        ));
                    }
                    ToyValue::Integer((token - INTEGER_TOKEN_BASE) as u8)
                }
                TRUE_VALUE_TOKEN => ToyValue::Bool(true),
                FALSE_VALUE_TOKEN => ToyValue::Bool(false),
                STRING_VALUE_TOKEN => ToyValue::String(decode_string_until(
                    &mut cursor,
                    STRING_END_TOKEN,
                    "string",
                )?),
                other => {
                    return Err(ToyGrammarError::new(
                        "decode",
                        "value_tag",
                        Some(cursor.position().saturating_sub(1)),
                        format!("unsupported value tag {other}"),
                    ));
                }
            };
            statements.push(ToyStatement { identifier, value });
        }
        cursor.expect(PROGRAM_END_TOKEN, "program_end")?;
        cursor.expect_done()?;
        Ok(ToyProgram { statements })
    }

    pub fn render_program(&self, program: &ToyProgram) -> String {
        let mut output = String::new();
        for statement in &program.statements {
            output.push_str("let ");
            output.push_str(&statement.identifier);
            output.push_str(" = ");
            match &statement.value {
                ToyValue::Integer(value) => output.push_str(&value.to_string()),
                ToyValue::Bool(true) => output.push_str("true"),
                ToyValue::Bool(false) => output.push_str("false"),
                ToyValue::String(value) => {
                    output.push('"');
                    output.push_str(&escape_string(value));
                    output.push('"');
                }
            }
            output.push_str(";\n");
        }
        output
    }
}

fn encode_bytes(
    bytes: &[u8],
    end_token: u32,
    tokens: &mut Vec<u32>,
) -> Result<(), ToyGrammarError> {
    for byte in bytes {
        tokens.push(BYTE_TOKEN_BASE + u32::from(*byte));
    }
    tokens.push(end_token);
    Ok(())
}

fn decode_string_until(
    cursor: &mut TokenCursor<'_>,
    end_token: u32,
    rule: &'static str,
) -> Result<String, ToyGrammarError> {
    let mut bytes = Vec::new();
    loop {
        let token = cursor.next(rule)?;
        if token == end_token {
            break;
        }
        if !(BYTE_TOKEN_BASE..BYTE_TOKEN_END).contains(&token) {
            return Err(ToyGrammarError::new(
                "decode",
                rule,
                Some(cursor.position().saturating_sub(1)),
                format!("byte token {token} is outside the UTF-8 byte range"),
            ));
        }
        bytes.push((token - BYTE_TOKEN_BASE) as u8);
    }
    String::from_utf8(bytes).map_err(|error| {
        ToyGrammarError::new(
            "decode",
            rule,
            Some(cursor.position()),
            format!("payload is not valid UTF-8: {error}"),
        )
    })
}

fn escape_string(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

struct TokenCursor<'a> {
    tokens: &'a [u32],
    position: usize,
}

impl<'a> TokenCursor<'a> {
    fn new(tokens: &'a [u32]) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn position(&self) -> usize {
        self.position
    }

    fn peek(&self) -> Option<u32> {
        self.tokens.get(self.position).copied()
    }

    fn next(&mut self, rule: &'static str) -> Result<u32, ToyGrammarError> {
        let token = self.tokens.get(self.position).copied().ok_or_else(|| {
            ToyGrammarError::new(
                "decode",
                rule,
                Some(self.position),
                "unexpected end of token stream",
            )
        })?;
        self.position += 1;
        Ok(token)
    }

    fn expect(&mut self, expected: u32, rule: &'static str) -> Result<(), ToyGrammarError> {
        let actual = self.next(rule)?;
        if actual != expected {
            return Err(ToyGrammarError::new(
                "decode",
                rule,
                Some(self.position.saturating_sub(1)),
                format!("expected token {expected}, got {actual}"),
            ));
        }
        Ok(())
    }

    fn expect_done(&self) -> Result<(), ToyGrammarError> {
        if self.position != self.tokens.len() {
            return Err(ToyGrammarError::new(
                "decode",
                "trailing_tokens",
                Some(self.position),
                format!(
                    "{} trailing token(s) remain",
                    self.tokens.len() - self.position
                ),
            ));
        }
        Ok(())
    }
}

struct Parser<'a> {
    source: &'a str,
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            position: 0,
        }
    }

    fn parse_program(&mut self) -> Result<ToyProgram, ToyGrammarError> {
        let mut statements = Vec::new();
        self.skip_whitespace();
        while !self.is_eof() {
            statements.push(self.parse_statement()?);
            self.skip_whitespace();
        }
        Ok(ToyProgram { statements })
    }

    fn parse_statement(&mut self) -> Result<ToyStatement, ToyGrammarError> {
        self.expect_keyword("let")?;
        self.require_whitespace("let_statement")?;
        let identifier = self.parse_identifier()?;
        self.skip_whitespace();
        self.expect_char('=', "assignment")?;
        self.skip_whitespace();
        let value = self.parse_value()?;
        self.skip_whitespace();
        self.expect_char(';', "statement_end")?;
        Ok(ToyStatement { identifier, value })
    }

    fn parse_identifier(&mut self) -> Result<String, ToyGrammarError> {
        let start = self.position;
        let Some(first) = self.peek_char() else {
            return Err(self.error("identifier", "expected identifier"));
        };
        if !(first == '_' || first.is_ascii_alphabetic()) {
            return Err(self.error("identifier", "identifier must start with ASCII letter or _"));
        }
        self.advance_char();
        while self
            .peek_char()
            .is_some_and(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        {
            self.advance_char();
        }
        Ok(self.source[start..self.position].to_string())
    }

    fn parse_value(&mut self) -> Result<ToyValue, ToyGrammarError> {
        if self.starts_with("true") {
            self.position += "true".len();
            return Ok(ToyValue::Bool(true));
        }
        if self.starts_with("false") {
            self.position += "false".len();
            return Ok(ToyValue::Bool(false));
        }
        if self.peek_char() == Some('"') {
            return self.parse_string().map(ToyValue::String);
        }
        if self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            return self.parse_integer().map(ToyValue::Integer);
        }
        Err(self.error("value", "expected u8 integer, bool, or string literal"))
    }

    fn parse_integer(&mut self) -> Result<u8, ToyGrammarError> {
        let start = self.position;
        while self.peek_char().is_some_and(|ch| ch.is_ascii_digit()) {
            self.advance_char();
        }
        let text = &self.source[start..self.position];
        let value = text
            .parse::<u16>()
            .map_err(|error| self.error("integer", format!("invalid integer: {error}")))?;
        u8::try_from(value).map_err(|_| self.error("integer", "integer must be in 0..=255"))
    }

    fn parse_string(&mut self) -> Result<String, ToyGrammarError> {
        self.expect_char('"', "string")?;
        let mut value = String::new();
        while let Some(ch) = self.peek_char() {
            self.advance_char();
            match ch {
                '"' => return Ok(value),
                '\\' => {
                    let escaped = self
                        .peek_char()
                        .ok_or_else(|| self.error("string", "unterminated escape"))?;
                    self.advance_char();
                    match escaped {
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        'n' => value.push('\n'),
                        _ => return Err(self.error("string", "unsupported escape sequence")),
                    }
                }
                '\n' | '\r' => return Err(self.error("string", "unescaped newline in string")),
                _ => value.push(ch),
            }
        }
        Err(self.error("string", "unterminated string literal"))
    }

    fn expect_keyword(&mut self, keyword: &'static str) -> Result<(), ToyGrammarError> {
        if !self.starts_with(keyword) {
            return Err(self.error("let_statement", format!("expected keyword {keyword:?}")));
        }
        let end = self.position + keyword.len();
        if self.source[end..]
            .chars()
            .next()
            .is_some_and(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        {
            return Err(self.error("let_statement", "keyword must be followed by whitespace"));
        }
        self.position = end;
        Ok(())
    }

    fn require_whitespace(&mut self, rule: &'static str) -> Result<(), ToyGrammarError> {
        if !self.peek_char().is_some_and(char::is_whitespace) {
            return Err(self.error(rule, "expected whitespace"));
        }
        self.skip_whitespace();
        Ok(())
    }

    fn skip_whitespace(&mut self) {
        while self.peek_char().is_some_and(char::is_whitespace) {
            self.advance_char();
        }
    }

    fn expect_char(&mut self, expected: char, rule: &'static str) -> Result<(), ToyGrammarError> {
        if self.peek_char() != Some(expected) {
            return Err(self.error(rule, format!("expected {expected:?}")));
        }
        self.advance_char();
        Ok(())
    }

    fn starts_with(&self, text: &str) -> bool {
        self.source[self.position..].starts_with(text)
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.position..].chars().next()
    }

    fn advance_char(&mut self) {
        if let Some(ch) = self.peek_char() {
            self.position += ch.len_utf8();
        }
    }

    fn is_eof(&self) -> bool {
        self.position >= self.source.len()
    }

    fn error(&self, rule: &'static str, message: impl Into<String>) -> ToyGrammarError {
        ToyGrammarError::new("parse", rule, Some(self.position), message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToyGrammarError {
    pub phase: &'static str,
    pub rule: &'static str,
    pub position: Option<usize>,
    pub message: String,
}

impl ToyGrammarError {
    fn new(
        phase: &'static str,
        rule: &'static str,
        position: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            phase,
            rule,
            position,
            message: message.into(),
        }
    }
}

impl fmt::Display for ToyGrammarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.position {
            Some(position) => write!(
                formatter,
                "{} error in {} at {}: {}",
                self.phase, self.rule, position, self.message
            ),
            None => write!(
                formatter,
                "{} error in {}: {}",
                self.phase, self.rule, self.message
            ),
        }
    }
}

impl Error for ToyGrammarError {}
