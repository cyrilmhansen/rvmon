#![forbid(unsafe_code)]

use luna_diag::{Diagnostic, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub line: u32,
    pub column: u32,
    pub length: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Identifier(String),
    Number(String),
    String(String),
    Comma,
    Colon,
    LParen,
    RParen,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
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
        if character == '#' {
            break;
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
            '"' => {
                let mut value = String::new();
                let mut length = 1;
                loop {
                    let next = chars.next().ok_or_else(|| {
                        Diagnostic::error("PARSE-STRING-001", "unterminated string").at(line, start)
                    })?;
                    length += 1;
                    if next == '"' {
                        break;
                    }
                    if next == '\\' {
                        let escaped = chars.next().ok_or_else(|| {
                            Diagnostic::error("PARSE-STRING-002", "unterminated escape")
                                .at(line, start)
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
                                .at(line, column));
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
            character if character.is_ascii_alphanumeric() || "_.$+-".contains(character) => {
                let mut value = String::from(character);
                while let Some(next) = chars.peek().copied() {
                    if next.is_ascii_alphanumeric() || "_.$+-xX".contains(next) {
                        value.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let kind = if value.starts_with("0x")
                    || value.starts_with("0X")
                    || value
                        .chars()
                        .all(|item| item.is_ascii_digit() || item == '-')
                {
                    TokenKind::Number(value)
                } else {
                    TokenKind::Identifier(value)
                };
                let length = match &kind {
                    TokenKind::Identifier(value) | TokenKind::Number(value) => value.len() as u32,
                    _ => unreachable!("identifier branch produces only identifier or number"),
                };
                Token {
                    kind,
                    span: span(length),
                }
            }
            _ => {
                return Err(
                    Diagnostic::error("PARSE-CHAR-001", "unexpected character").at(line, column)
                );
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
    fn tokenizes_instruction_and_label() {
        let tokens = tokenize("start: addi x1,x0,1 # comment").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Identifier("start".into()));
        assert_eq!(tokens[0].span.column, 1);
        assert_eq!(tokens[3].kind, TokenKind::Identifier("x1".into()));
    }
    #[test]
    fn reports_position_for_bad_character() {
        let error = tokenize("addi x1, @").unwrap_err();
        assert_eq!(error.code, "PARSE-CHAR-001");
        assert_eq!(error.line, Some(1));
        assert_eq!(error.column, Some(10));
    }
}
