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
    String(String),
    Memory { offset: String, base: String },
}

impl Operand {
    fn atom(token: &Token) -> Result<Self> {
        let kind = match &token.kind {
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
        let token = self
            .take()
            .ok_or_else(|| self.error_at("ASM-SYNTAX-003", "missing operand"))?;
        let mut operand = Operand::atom(&token)?;
        if !matches!(self.peek().map(|item| &item.kind), Some(TokenKind::LParen)) {
            return Ok(operand);
        }

        let open = self.take().expect("peeked opening parenthesis");
        let base = self.take().ok_or_else(|| {
            Diagnostic::error("ASM-MEMORY-002", "missing base register")
                .at(open.span.line, open.span.column)
        })?;
        let base_name = match base.kind {
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
            OperandKind::Integer(value) | OperandKind::Symbol(value) => value,
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
}
