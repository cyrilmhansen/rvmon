#![forbid(unsafe_code)]

use luna_asm_lexer::{Span, Token, TokenKind, tokenize};
use luna_diag::{Diagnostic, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedLine {
    pub labels: Vec<String>,
    pub mnemonic: Option<String>,
    pub operands: Vec<Operand>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operand {
    pub kind: OperandKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperandKind {
    Register(String),
    Integer(String),
    Symbol(String),
    Expression(String),
    String(String),
    BitPattern { width: usize, value: String },
    Memory { offset: String, base: String },
}

impl Operand {
    fn atom(token: &Token) -> Result<Self> {
        let kind = match &token.kind {
            TokenKind::Register(value) => OperandKind::Register(value.clone()),
            TokenKind::Identifier(value) if is_register_name(value) => {
                OperandKind::Register(value.clone())
            }
            TokenKind::Identifier(value) => OperandKind::Symbol(value.clone()),
            TokenKind::Number(value) => OperandKind::Integer(value.clone()),
            TokenKind::String(value) => OperandKind::String(value.clone()),
            _ => {
                return Err(Diagnostic::error("ASM-SYNTAX-002", "expected an operand")
                    .at(token.span.line, token.span.column));
            }
        };
        Ok(Self {
            kind,
            span: token.span,
        })
    }
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

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn new(source: &str) -> Result<Self> {
        Ok(Self {
            tokens: tokenize(source)?,
            cursor: 0,
        })
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    fn take(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.cursor).cloned();
        self.cursor += usize::from(token.is_some());
        token
    }

    fn error_at(&self, code: &'static str, message: impl Into<String>) -> Diagnostic {
        let message = message.into();
        match self.peek() {
            None => Diagnostic::error(code, message),
            Some(token) => Diagnostic::error(code, message).at(token.span.line, token.span.column),
        }
    }

    fn parse_operand(&mut self) -> Result<Operand> {
        let first = self
            .take()
            .ok_or_else(|| self.error_at("ASM-SYNTAX-003", "missing operand"))?;
        let mut pieces = Vec::new();
        let mut has_operator = false;
        let first_atom = if let TokenKind::Operator(operator) = first.kind {
            if !matches!(operator, '+' | '-' | '~') {
                return Err(Diagnostic::error("ASM-SYNTAX-005", "expected operand")
                    .at(first.span.line, first.span.column));
            }
            has_operator = true;
            pieces.push(operator.to_string());
            self.take()
                .ok_or_else(|| self.error_at("ASM-SYNTAX-003", "missing operand"))?
        } else {
            first
        };
        let mut operand = Operand::atom(&first_atom)?;
        pieces.push(match &first_atom.kind {
            TokenKind::Identifier(value)
            | TokenKind::Register(value)
            | TokenKind::Number(value) => value.clone(),
            TokenKind::String(_) => String::new(),
            _ => unreachable!("operand atom has a scalar token"),
        });
        if let TokenKind::Identifier(name) = &first_atom.kind {
            if let Some(width) = bit_pattern_width(name)
                && matches!(
                    self.peek().map(|token| &token.kind),
                    Some(TokenKind::LParen)
                )
            {
                let open = self.take().expect("peeked opening parenthesis");
                let value = self.take().ok_or_else(|| {
                    Diagnostic::error("ASM-BITS-001", "missing bit-pattern value")
                        .at(open.span.line, open.span.column)
                })?;
                let value_text = match value.kind {
                    TokenKind::Number(value) | TokenKind::Identifier(value) => value,
                    _ => {
                        return Err(Diagnostic::error(
                            "ASM-BITS-001",
                            "bit-pattern value must be numeric",
                        )
                        .at(value.span.line, value.span.column));
                    }
                };
                let close = self.take().ok_or_else(|| {
                    Diagnostic::error("ASM-BITS-001", "missing closing parenthesis")
                        .at(open.span.line, open.span.column)
                })?;
                if !matches!(close.kind, TokenKind::RParen) {
                    return Err(
                        Diagnostic::error("ASM-BITS-001", "missing closing parenthesis")
                            .at(close.span.line, close.span.column),
                    );
                }
                operand.kind = OperandKind::BitPattern {
                    width,
                    value: value_text,
                };
                operand.span.length = close.span.column + close.span.length - operand.span.column;
                return Ok(operand);
            }
        }
        while let Some(token) = self.peek() {
            let operator = match &token.kind {
                TokenKind::Operator(operator) => operator.to_string(),
                TokenKind::ShiftLeft => "<<".to_owned(),
                TokenKind::ShiftRight => ">>".to_owned(),
                _ => break,
            };
            has_operator = true;
            pieces.push(operator);
            self.take();
            let next = self
                .take()
                .ok_or_else(|| self.error_at("ASM-SYNTAX-003", "missing operand"))?;
            let next_value = match &next.kind {
                TokenKind::Identifier(value)
                | TokenKind::Register(value)
                | TokenKind::Number(value) => value.clone(),
                _ => {
                    return Err(
                        Diagnostic::error("ASM-SYNTAX-005", "expected expression atom")
                            .at(next.span.line, next.span.column),
                    );
                }
            };
            pieces.push(next_value);
        }
        if has_operator {
            if matches!(operand.kind, OperandKind::String(_)) {
                return Err(Diagnostic::error(
                    "ASM-EXPR-006",
                    "strings cannot be used in expressions",
                )
                .at(operand.span.line, operand.span.column));
            }
            let expression = pieces.concat();
            operand.kind = if matches!(operand.kind, OperandKind::Integer(_)) && pieces.len() == 2 {
                OperandKind::Integer(expression)
            } else {
                OperandKind::Expression(expression)
            };
        }
        if !matches!(self.peek().map(|item| &item.kind), Some(TokenKind::LParen)) {
            return Ok(operand);
        }

        let open = self.take().expect("peeked opening parenthesis");
        let base = self.take().ok_or_else(|| {
            Diagnostic::error("ASM-MEMORY-002", "missing base register")
                .at(open.span.line, open.span.column)
        })?;
        let base_name = match base.kind {
            TokenKind::Register(value) => value,
            TokenKind::Identifier(value) if is_register_name(&value) => value,
            _ => {
                return Err(
                    Diagnostic::error("ASM-MEMORY-003", "base must be a register")
                        .at(base.span.line, base.span.column),
                );
            }
        };
        let close = self.take().ok_or_else(|| {
            Diagnostic::error("ASM-MEMORY-004", "missing closing parenthesis")
                .at(open.span.line, open.span.column)
        })?;
        if !matches!(close.kind, TokenKind::RParen) {
            return Err(
                Diagnostic::error("ASM-MEMORY-004", "missing closing parenthesis")
                    .at(close.span.line, close.span.column),
            );
        }
        let offset = match operand.kind {
            OperandKind::Integer(value)
            | OperandKind::Symbol(value)
            | OperandKind::Expression(value) => value,
            _ => {
                return Err(Diagnostic::error(
                    "ASM-MEMORY-005",
                    "memory offset must be an integer",
                )
                .at(operand.span.line, operand.span.column));
            }
        };
        operand.kind = OperandKind::Memory {
            offset,
            base: base_name,
        };
        operand.span.length = close.span.column + close.span.length - operand.span.column;
        Ok(operand)
    }

    fn parse(mut self) -> Result<ParsedLine> {
        let mut labels = Vec::new();
        loop {
            let is_label = matches!(
                (
                    self.tokens.get(self.cursor),
                    self.tokens.get(self.cursor + 1)
                ),
                (
                    Some(Token {
                        kind: TokenKind::Identifier(_),
                        ..
                    }),
                    Some(Token {
                        kind: TokenKind::Colon,
                        ..
                    })
                )
            );
            if !is_label {
                break;
            }
            let label = self.take().expect("label token");
            let name = match label.kind {
                TokenKind::Identifier(value) => value,
                _ => unreachable!("label lookahead guarantees identifier"),
            };
            self.take().expect("label colon");
            labels.push(name);
        }

        let mnemonic = match self.take() {
            None => None,
            Some(token) => match token.kind {
                TokenKind::Identifier(value) => Some(value.to_ascii_lowercase()),
                _ => {
                    return Err(Diagnostic::error(
                        "ASM-SYNTAX-001",
                        "expected instruction mnemonic",
                    )
                    .at(token.span.line, token.span.column));
                }
            },
        };
        let Some(mnemonic) = mnemonic else {
            return Ok(ParsedLine {
                labels,
                mnemonic: None,
                operands: Vec::new(),
            });
        };

        let mut operands = Vec::new();
        if mnemonic.starts_with('.') && self.peek().is_none() {
            return Ok(ParsedLine {
                labels,
                mnemonic: Some(mnemonic),
                operands,
            });
        }
        operands.push(self.parse_operand()?);
        while self.peek().is_some() {
            let comma = self.take().expect("operand separator");
            if !matches!(comma.kind, TokenKind::Comma) {
                return Err(
                    Diagnostic::error("ASM-SYNTAX-004", "expected comma between operands")
                        .at(comma.span.line, comma.span.column),
                );
            }
            operands.push(self.parse_operand()?);
        }
        Ok(ParsedLine {
            labels,
            mnemonic: Some(mnemonic),
            operands,
        })
    }
}

fn bit_pattern_width(name: &str) -> Option<usize> {
    match name.to_ascii_lowercase().as_str() {
        "bits16" => Some(16),
        "bits32" => Some(32),
        "bits64" => Some(64),
        "bits128" => Some(128),
        _ => None,
    }
}

pub fn parse_line(source: &str) -> Result<ParsedLine> {
    Parser::new(source)?.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_labels_registers_and_memory_operands() {
        let line = parse_line("loop: lw a0, -8(sp)").unwrap();
        assert_eq!(line.labels, ["loop"]);
        assert_eq!(line.mnemonic.as_deref(), Some("lw"));
        assert_eq!(
            line.operands[1].kind,
            OperandKind::Memory {
                offset: "-8".into(),
                base: "sp".into()
            }
        );
    }

    #[test]
    fn rejects_missing_operand_separator() {
        let error = parse_line("add x1 x2,x3").unwrap_err();
        assert_eq!(error.code, "ASM-SYNTAX-004");
    }

    #[test]
    fn accepts_instructionless_label() {
        let line = parse_line("done:").unwrap();
        assert_eq!(line.labels, ["done"]);
        assert!(line.mnemonic.is_none());
    }

    #[test]
    fn preserves_shift_operators_in_expressions() {
        let line = parse_line(".word 1 << 3").unwrap();
        assert_eq!(
            line.operands[0].kind,
            OperandKind::Expression("1<<3".into())
        );
    }

    #[test]
    fn parses_exact_float_bit_patterns() {
        let line = parse_line(".binary128 bits128(0x1)").unwrap();
        assert_eq!(
            line.operands[0].kind,
            OperandKind::BitPattern {
                width: 128,
                value: "0x1".into()
            }
        );
    }
}
