#![forbid(unsafe_code)]

use std::iter::Peekable;
use std::str::Chars;

use luna_diag::{Diagnostic, Result};

/// One-based source position. Columns and lengths count Unicode scalar values,
/// not UTF-8 bytes; this keeps diagnostics stable for human-readable strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub line: u32,
    pub column: u32,
    pub length: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Identifier(String),
    Register(String),
    Number(String),
    String(String),
    Comma,
    Colon,
    LParen,
    RParen,
    Operator(char),
    ShiftLeft,
    ShiftRight,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

fn is_register_name(value: &str) -> bool {
    value
        .strip_prefix('x')
        .and_then(|number| number.parse::<u8>().ok())
        .is_some_and(|number| number < 32)
        || value
            .strip_prefix('f')
            .and_then(|number| number.parse::<u8>().ok())
            .is_some_and(|number| number < 32)
        || matches!(
            value,
            "zero"
                | "ra"
                | "sp"
                | "gp"
                | "tp"
                | "t0"
                | "t1"
                | "t2"
                | "s0"
                | "fp"
                | "s1"
                | "a0"
                | "a1"
                | "a2"
                | "a3"
                | "a4"
                | "a5"
                | "a6"
                | "a7"
                | "s2"
                | "s3"
                | "s4"
                | "s5"
                | "s6"
                | "s7"
                | "s8"
                | "s9"
                | "s10"
                | "s11"
                | "t3"
                | "t4"
                | "t5"
                | "t6"
        )
}

fn is_number_literal(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if lower.starts_with("0x") || lower.starts_with("0b") || lower.starts_with("0o") {
        return true;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || byte == b'_')
        && value.bytes().any(|byte| byte.is_ascii_digit())
}

fn classify_word(value: String) -> TokenKind {
    let lower = value.to_ascii_lowercase();
    if is_register_name(&lower) {
        TokenKind::Register(lower)
    } else if is_number_literal(&value) {
        TokenKind::Number(value)
    } else {
        TokenKind::Identifier(value)
    }
}

fn skip_comment(chars: &mut Peekable<Chars<'_>>, line: &mut u32, column: &mut u32) {
    while let Some(next) = chars.next() {
        if next == '\n' {
            *line += 1;
            *column = 1;
            return;
        }
        *column += 1;
    }
}

pub fn tokenize(source: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut line = 1u32;
    let mut column = 1u32;
    let mut chars = source.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\n' {
            line += 1;
            column = 1;
            continue;
        }
        if character.is_whitespace() {
            column += 1;
            continue;
        }
        if matches!(character, '#' | ';') {
            column += 1;
            skip_comment(&mut chars, &mut line, &mut column);
            continue;
        }
        if character == '/' && chars.peek() == Some(&'/') {
            chars.next();
            column += 2;
            skip_comment(&mut chars, &mut line, &mut column);
            continue;
        }

        let start = column;
        let span = |length| Span {
            line,
            column: start,
            length,
        };
        let token = match character {
            ',' => Token {
                kind: TokenKind::Comma,
                span: span(1),
            },
            ':' => Token {
                kind: TokenKind::Colon,
                span: span(1),
            },
            '(' => Token {
                kind: TokenKind::LParen,
                span: span(1),
            },
            ')' => Token {
                kind: TokenKind::RParen,
                span: span(1),
            },
            '<' if chars.peek() == Some(&'<') => {
                chars.next();
                Token {
                    kind: TokenKind::ShiftLeft,
                    span: span(2),
                }
            }
            '>' if chars.peek() == Some(&'>') => {
                chars.next();
                Token {
                    kind: TokenKind::ShiftRight,
                    span: span(2),
                }
            }
            '+' | '-' | '*' | '/' | '%' | '&' | '|' | '^' | '~' | '<' | '>' => Token {
                kind: TokenKind::Operator(character),
                span: span(1),
            },
            '"' => {
                let mut value = String::new();
                let mut length = 1;
                loop {
                    let next = chars.next().ok_or_else(|| {
                        Diagnostic::error("PARSE-STRING-001", "unterminated string")
                            .at_len(line, start, length)
                    })?;
                    length += 1;
                    if next == '"' {
                        break;
                    }
                    if next == '\n' {
                        return Err(Diagnostic::error(
                            "PARSE-STRING-004",
                            "newline in string literal",
                        )
                        .at_len(line, start, length));
                    }
                    if next == '\\' {
                        let escaped = chars.next().ok_or_else(|| {
                            Diagnostic::error("PARSE-STRING-002", "unterminated escape")
                                .at_len(line, start, length)
                        })?;
                        length += 1;
                        value.push(match escaped {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '\\' => '\\',
                            '"' => '"',
                            _ => {
                                return Err(Diagnostic::error(
                                    "PARSE-STRING-003",
                                    "unknown string escape",
                                )
                                .at_len(line, start, length));
                            }
                        });
                    } else {
                        value.push(next);
                    }
                }
                Token {
                    kind: TokenKind::String(value),
                    span: span(length),
                }
            }
            character if character.is_ascii_alphanumeric() || "_.$".contains(character) => {
                let mut value = String::from(character);
                while let Some(next) = chars.peek().copied() {
                    if next.is_ascii_alphanumeric() || "_.$".contains(next) {
                        value.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let length = value.chars().count() as u32;
                Token {
                    kind: classify_word(value),
                    span: span(length),
                }
            }
            _ => {
                return Err(Diagnostic::error("PARSE-CHAR-001", "unexpected character")
                    .at_len(line, column, 1));
            }
        };
        column += token.span.length;
        tokens.push(token);
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_instruction_registers_and_multiline_comments() {
        let tokens =
            tokenize("START: ADDI X1,A0,1 # comment\nnext: sd x2,8(sp) // comment\n").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Identifier("START".into()));
        assert_eq!(tokens[2].kind, TokenKind::Identifier("ADDI".into()));
        assert_eq!(tokens[3].kind, TokenKind::Register("x1".into()));
        assert_eq!(tokens[5].kind, TokenKind::Register("a0".into()));
        assert_eq!(tokens[8].span.line, 2);
        assert_eq!(tokens[8].kind, TokenKind::Identifier("next".into()));
    }

    #[test]
    fn tokenizes_shift_operators_and_numbers() {
        let tokens = tokenize(".word 0x1f + (1 << 3)").unwrap();
        assert_eq!(tokens[1].kind, TokenKind::Number("0x1f".into()));
        assert_eq!(tokens[5].kind, TokenKind::ShiftLeft);
        assert_eq!(tokens[7].kind, TokenKind::RParen);
        assert_eq!(tokens[5].span.length, 2);
    }

    #[test]
    fn counts_unicode_string_spans_in_scalar_columns() {
        let tokens = tokenize(".ascii \"é\"\nnext:").unwrap();
        assert_eq!(tokens[1].kind, TokenKind::String("é".into()));
        assert_eq!(tokens[1].span.length, 3);
        assert_eq!(tokens[2].span.line, 2);
        assert_eq!(tokens[2].span.column, 1);
    }

    #[test]
    fn reports_position_and_length_for_bad_character() {
        let error = tokenize("addi x1, @").unwrap_err();
        assert_eq!(error.code, "PARSE-CHAR-001");
        assert_eq!(error.line, Some(1));
        assert_eq!(error.column, Some(10));
        assert_eq!(error.length, Some(1));
    }

    #[test]
    fn reports_unterminated_and_multiline_strings() {
        let error = tokenize(".ascii \"unterminated").unwrap_err();
        assert_eq!(error.code, "PARSE-STRING-001");
        assert_eq!(error.length, Some(13));
        let error = tokenize(".ascii \"a\nb\"").unwrap_err();
        assert_eq!(error.code, "PARSE-STRING-004");
    }
}
