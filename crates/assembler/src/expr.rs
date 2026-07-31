#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use luna_diag::{Diagnostic, Result};

pub fn evaluate(source: &str, symbols: &BTreeMap<String, u64>) -> Result<i128> {
    let mut parser = Parser {
        source,
        symbols,
        cursor: 0,
    };
    let value = parser.expression(0)?;
    parser.skip_space();
    if parser.cursor != source.len() {
        return Err(Diagnostic::error(
            "ASM-EXPR-001",
            "unexpected token in expression",
        ));
    }
    Ok(value)
}

pub fn references_symbol(source: &str) -> bool {
    let mut start = None;
    for (index, character) in source
        .char_indices()
        .chain(std::iter::once((source.len(), ' ')))
    {
        let is_atom = character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '$');
        if is_atom && start.is_none() {
            start = Some(index);
        } else if !is_atom {
            if let Some(atom_start) = start.take() {
                if parse_integer(&source[atom_start..index]).is_none() {
                    return true;
                }
            }
        }
    }
    false
}

struct Parser<'a> {
    source: &'a str,
    symbols: &'a BTreeMap<String, u64>,
    cursor: usize,
}

impl<'a> Parser<'a> {
    fn skip_space(&mut self) {
        while self.source[self.cursor..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            self.cursor += 1;
        }
    }

    fn expression(&mut self, minimum_binding_power: u8) -> Result<i128> {
        let mut left = self.prefix()?;
        loop {
            self.skip_space();
            let Some((operator, width)) = self.binary_operator() else {
                break;
            };
            let Some((left_bp, right_bp)) = binding_power(operator) else {
                break;
            };
            if left_bp < minimum_binding_power {
                break;
            }
            self.cursor += width;
            left = apply(operator, left, self.expression(right_bp)?)?;
        }
        Ok(left)
    }

    fn prefix(&mut self) -> Result<i128> {
        self.skip_space();
        let Some(character) = self.source[self.cursor..].chars().next() else {
            return Err(Diagnostic::error("ASM-EXPR-002", "missing expression"));
        };
        if matches!(character, '+' | '-' | '~') {
            self.cursor += character.len_utf8();
            let value = self.prefix()?;
            return match character {
                '+' => Ok(value),
                '-' => value
                    .checked_neg()
                    .ok_or_else(|| Diagnostic::error("ASM-EXPR-003", "expression overflow")),
                '~' => Ok(!value),
                _ => unreachable!(),
            };
        }
        if character == '(' {
            self.cursor += 1;
            let value = self.expression(0)?;
            self.skip_space();
            if self.source[self.cursor..].starts_with(')') {
                self.cursor += 1;
                return Ok(value);
            }
            return Err(Diagnostic::error(
                "ASM-EXPR-004",
                "missing closing parenthesis",
            ));
        }
        let start = self.cursor;
        while let Some(next) = self.source[self.cursor..].chars().next() {
            if next.is_ascii_alphanumeric() || matches!(next, '_' | '.' | '$') {
                self.cursor += next.len_utf8();
            } else {
                break;
            }
        }
        if self.cursor == start {
            return Err(Diagnostic::error(
                "ASM-EXPR-005",
                "expected integer or symbol",
            ));
        }
        let atom = &self.source[start..self.cursor];
        if let Some(value) = parse_integer(atom) {
            return Ok(value);
        }
        self.symbols
            .get(atom)
            .copied()
            .map(i128::from)
            .ok_or_else(|| Diagnostic::error("ASM-SYMBOL-001", format!("unknown symbol: {atom}")))
    }

    fn binary_operator(&self) -> Option<(&'static str, usize)> {
        let remaining = &self.source[self.cursor..];
        for (text, width) in [("<<", 2), (">>", 2)] {
            if remaining.starts_with(text) {
                return Some((text, width));
            }
        }
        remaining.chars().next().and_then(|character| {
            Some((
                match character {
                    '+' => "+",
                    '-' => "-",
                    '*' => "*",
                    '/' => "/",
                    '%' => "%",
                    '&' => "&",
                    '^' => "^",
                    '|' => "|",
                    _ => return None,
                },
                1,
            ))
        })
    }
}

fn parse_integer(atom: &str) -> Option<i128> {
    let (radix, digits) =
        if let Some(value) = atom.strip_prefix("0x").or_else(|| atom.strip_prefix("0X")) {
            (16, value)
        } else if let Some(value) = atom.strip_prefix("0b").or_else(|| atom.strip_prefix("0B")) {
            (2, value)
        } else if let Some(value) = atom.strip_prefix("0o").or_else(|| atom.strip_prefix("0O")) {
            (8, value)
        } else {
            return atom.parse().ok();
        };
    u128::from_str_radix(digits, radix)
        .ok()
        .and_then(|value| i128::try_from(value).ok())
}

fn binding_power(operator: &str) -> Option<(u8, u8)> {
    Some(match operator {
        "|" => (1, 2),
        "^" => (3, 4),
        "&" => (5, 6),
        "<<" | ">>" => (7, 8),
        "+" | "-" => (9, 10),
        "*" | "/" | "%" => (11, 12),
        _ => return None,
    })
}

fn apply(operator: &str, left: i128, right: i128) -> Result<i128> {
    let value = match operator {
        "+" => left.checked_add(right),
        "-" => left.checked_sub(right),
        "*" => left.checked_mul(right),
        "/" => left.checked_div(right),
        "%" => left.checked_rem(right),
        "&" => Some(left & right),
        "^" => Some(left ^ right),
        "|" => Some(left | right),
        "<<" => u32::try_from(right)
            .ok()
            .and_then(|shift| left.checked_shl(shift)),
        ">>" => u32::try_from(right)
            .ok()
            .and_then(|shift| left.checked_shr(shift)),
        _ => None,
    };
    value.ok_or_else(|| {
        Diagnostic::error("ASM-EXPR-003", "expression overflow or invalid operation")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn respects_precedence_and_bases() {
        let symbols = BTreeMap::from([(String::from("label"), 20)]);
        assert_eq!(evaluate("1 + 2 * 3", &symbols).unwrap(), 7);
        assert_eq!(evaluate("0x10 + 0b10 + label", &symbols).unwrap(), 38);
    }
    #[test]
    fn supports_unary_and_shifts() {
        let symbols = BTreeMap::new();
        assert_eq!(evaluate("-(1 << 3)", &symbols).unwrap(), -8);
        assert_eq!(evaluate("~0xff", &symbols).unwrap(), -256);
    }
    #[test]
    fn reports_unknown_symbols_and_division_by_zero() {
        let symbols = BTreeMap::new();
        assert_eq!(
            evaluate("missing", &symbols).unwrap_err().code,
            "ASM-SYMBOL-001"
        );
        assert_eq!(
            evaluate("1 / 0", &symbols).unwrap_err().code,
            "ASM-EXPR-003"
        );
    }
}
