use luna_diag::{Diagnostic, Result};

const MAX_COMMAND_LINE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommandLine {
    pub(crate) name: String,
    pub(crate) raw_arguments: String,
    pub(crate) tokens: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Expression {
    Literal(i128),
    Symbol(String),
    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnaryOperator {
    Plus,
    Minus,
    BitNot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    ShiftLeft,
    ShiftRight,
    BitAnd,
    BitXor,
    BitOr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExpressionRange {
    pub(crate) start: Expression,
    pub(crate) end: Option<Expression>,
}

impl Expression {
    pub(crate) fn evaluate(&self, resolve: &dyn Fn(&str) -> Option<i128>) -> Result<i128> {
        match self {
            Self::Literal(value) => Ok(*value),
            Self::Symbol(name) => resolve(name).ok_or_else(|| {
                Diagnostic::error("CMD-003", format!("unknown expression symbol: {name}"))
            }),
            Self::Unary { operator, operand } => {
                let value = operand.evaluate(resolve)?;
                match operator {
                    UnaryOperator::Plus => Ok(value),
                    UnaryOperator::Minus => value
                        .checked_neg()
                        .ok_or_else(|| Diagnostic::error("CMD-003", "expression signed overflow")),
                    UnaryOperator::BitNot => Ok(!value),
                }
            }
            Self::Binary {
                operator,
                left,
                right,
            } => {
                let left = left.evaluate(resolve)?;
                let right = right.evaluate(resolve)?;
                let result = match operator {
                    BinaryOperator::Add => left.checked_add(right),
                    BinaryOperator::Subtract => left.checked_sub(right),
                    BinaryOperator::Multiply => left.checked_mul(right),
                    BinaryOperator::Divide => left.checked_div(right),
                    BinaryOperator::Remainder => left.checked_rem(right),
                    BinaryOperator::ShiftLeft => shift_left(left, right),
                    BinaryOperator::ShiftRight => shift_right(left, right),
                    BinaryOperator::BitAnd => Some(left & right),
                    BinaryOperator::BitXor => Some(left ^ right),
                    BinaryOperator::BitOr => Some(left | right),
                };
                result.ok_or_else(|| {
                    Diagnostic::error("CMD-003", "expression overflow or invalid operation")
                })
            }
        }
    }
}

pub(crate) fn parse_expression(input: &str) -> Result<Expression> {
    let mut parser = ExpressionParser::new(input)?;
    let expression = parser.expression(0)?;
    if !matches!(parser.peek(), ExpressionToken::End) {
        return Err(Diagnostic::error(
            "CMD-003",
            "unexpected token after expression",
        ));
    }
    Ok(expression)
}

pub(crate) fn parse_range(input: &str) -> Result<ExpressionRange> {
    let mut parser = ExpressionParser::new(input)?;
    let start = parser.expression(0)?;
    let end = if matches!(parser.peek(), ExpressionToken::Range) {
        parser.next();
        Some(parser.expression(0)?)
    } else {
        None
    };
    if !matches!(parser.peek(), ExpressionToken::End) {
        return Err(Diagnostic::error(
            "CMD-003",
            "range expects one expression or start..end",
        ));
    }
    Ok(ExpressionRange { start, end })
}

pub(crate) fn register_index(name: &str) -> Option<usize> {
    let name = name.to_ascii_lowercase();
    if let Some(index) = name.strip_prefix('x').and_then(|value| value.parse().ok()) {
        return (index < 32).then_some(index);
    }
    match name.as_str() {
        "zero" => Some(0),
        "ra" => Some(1),
        "sp" => Some(2),
        "gp" => Some(3),
        "tp" => Some(4),
        "t0" => Some(5),
        "t1" => Some(6),
        "t2" => Some(7),
        "s0" | "fp" => Some(8),
        "s1" => Some(9),
        "a0" => Some(10),
        "a1" => Some(11),
        "a2" => Some(12),
        "a3" => Some(13),
        "a4" => Some(14),
        "a5" => Some(15),
        "a6" => Some(16),
        "a7" => Some(17),
        "s2" => Some(18),
        "s3" => Some(19),
        "s4" => Some(20),
        "s5" => Some(21),
        "s6" => Some(22),
        "s7" => Some(23),
        "s8" => Some(24),
        "s9" => Some(25),
        "s10" => Some(26),
        "s11" => Some(27),
        "t3" => Some(28),
        "t4" => Some(29),
        "t5" => Some(30),
        "t6" => Some(31),
        _ => None,
    }
}

fn shift_left(left: i128, right: i128) -> Option<i128> {
    u32::try_from(right).ok().filter(|shift| *shift < 128)?;
    left.checked_shl(right as u32)
}

fn shift_right(left: i128, right: i128) -> Option<i128> {
    u32::try_from(right).ok().filter(|shift| *shift < 128)?;
    Some(left >> right as u32)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExpressionToken<'a> {
    Literal(i128),
    Symbol(&'a str),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    ShiftLeft,
    ShiftRight,
    Ampersand,
    Caret,
    Pipe,
    Tilde,
    LeftParen,
    RightParen,
    Range,
    End,
}

struct ExpressionParser<'a> {
    tokens: Vec<ExpressionToken<'a>>,
    position: usize,
}

impl<'a> ExpressionParser<'a> {
    fn new(input: &'a str) -> Result<Self> {
        Ok(Self {
            tokens: tokenize_expression(input)?,
            position: 0,
        })
    }

    fn peek(&self) -> ExpressionToken<'a> {
        self.tokens
            .get(self.position)
            .copied()
            .unwrap_or(ExpressionToken::End)
    }

    fn next(&mut self) -> ExpressionToken<'a> {
        let token = self.peek();
        if !matches!(token, ExpressionToken::End) {
            self.position += 1;
        }
        token
    }

    fn expression(&mut self, minimum_precedence: u8) -> Result<Expression> {
        let mut left = self.unary()?;
        while let Some((operator, precedence)) = binary_operator(self.peek()) {
            if precedence < minimum_precedence {
                break;
            }
            self.next();
            let right = self.expression(precedence + 1)?;
            left = Expression::Binary {
                operator,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expression> {
        let operator = match self.peek() {
            ExpressionToken::Plus => Some(UnaryOperator::Plus),
            ExpressionToken::Minus => Some(UnaryOperator::Minus),
            ExpressionToken::Tilde => Some(UnaryOperator::BitNot),
            _ => None,
        };
        if let Some(operator) = operator {
            self.next();
            return Ok(Expression::Unary {
                operator,
                operand: Box::new(self.unary()?),
            });
        }
        match self.next() {
            ExpressionToken::Literal(value) => Ok(Expression::Literal(value)),
            ExpressionToken::Symbol(name) => Ok(Expression::Symbol(name.to_string())),
            ExpressionToken::LeftParen => {
                let expression = self.expression(0)?;
                if !matches!(self.next(), ExpressionToken::RightParen) {
                    return Err(Diagnostic::error("CMD-003", "missing closing parenthesis"));
                }
                Ok(expression)
            }
            _ => Err(Diagnostic::error("CMD-003", "expected an expression atom")),
        }
    }
}

fn binary_operator(token: ExpressionToken<'_>) -> Option<(BinaryOperator, u8)> {
    Some(match token {
        ExpressionToken::Pipe => (BinaryOperator::BitOr, 1),
        ExpressionToken::Caret => (BinaryOperator::BitXor, 2),
        ExpressionToken::Ampersand => (BinaryOperator::BitAnd, 3),
        ExpressionToken::ShiftLeft => (BinaryOperator::ShiftLeft, 4),
        ExpressionToken::ShiftRight => (BinaryOperator::ShiftRight, 4),
        ExpressionToken::Plus => (BinaryOperator::Add, 5),
        ExpressionToken::Minus => (BinaryOperator::Subtract, 5),
        ExpressionToken::Star => (BinaryOperator::Multiply, 6),
        ExpressionToken::Slash => (BinaryOperator::Divide, 6),
        ExpressionToken::Percent => (BinaryOperator::Remainder, 6),
        _ => return None,
    })
}

fn tokenize_expression(input: &str) -> Result<Vec<ExpressionToken<'_>>> {
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < input.len() {
        let character = input[index..]
            .chars()
            .next()
            .expect("expression index is on a character boundary");
        if character.is_ascii_whitespace() {
            index += character.len_utf8();
            continue;
        }
        let rest = &input[index..];
        let token = match character {
            '+' => {
                index += 1;
                ExpressionToken::Plus
            }
            '-' => {
                index += 1;
                ExpressionToken::Minus
            }
            '*' => {
                index += 1;
                ExpressionToken::Star
            }
            '/' => {
                index += 1;
                ExpressionToken::Slash
            }
            '%' => {
                index += 1;
                ExpressionToken::Percent
            }
            '&' => {
                index += 1;
                ExpressionToken::Ampersand
            }
            '^' => {
                index += 1;
                ExpressionToken::Caret
            }
            '|' => {
                index += 1;
                ExpressionToken::Pipe
            }
            '~' => {
                index += 1;
                ExpressionToken::Tilde
            }
            '(' => {
                index += 1;
                ExpressionToken::LeftParen
            }
            ')' => {
                index += 1;
                ExpressionToken::RightParen
            }
            '<' if rest.starts_with("<<") => {
                index += 2;
                ExpressionToken::ShiftLeft
            }
            '>' if rest.starts_with(">>") => {
                index += 2;
                ExpressionToken::ShiftRight
            }
            '.' if rest.starts_with("..") => {
                index += 2;
                ExpressionToken::Range
            }
            character if character.is_ascii_digit() => {
                let start = index;
                index += character.len_utf8();
                while input[index..].chars().next().is_some_and(|next| {
                    next.is_ascii_hexdigit() || next == '_' || next == 'x' || next == 'b'
                }) {
                    index += input[index..]
                        .chars()
                        .next()
                        .expect("digit continuation exists")
                        .len_utf8();
                }
                let text = &input[start..index];
                let normalized = text.replace('_', "");
                let value = if let Some(hex) = normalized.strip_prefix("0x") {
                    i128::from_str_radix(hex, 16)
                } else if let Some(binary) = normalized.strip_prefix("0b") {
                    i128::from_str_radix(binary, 2)
                } else {
                    normalized.parse::<i128>()
                }
                .map_err(|_| Diagnostic::error("CMD-003", "invalid expression number"))?;
                ExpressionToken::Literal(value)
            }
            character
                if character.is_ascii_alphabetic()
                    || matches!(character, '_' | '.' | '$' | '@') =>
            {
                let start = index;
                index += character.len_utf8();
                while let Some(next) = input[index..].chars().next() {
                    if next == '.' && input[index..].starts_with("..") {
                        break;
                    }
                    if !(next.is_ascii_alphanumeric() || matches!(next, '_' | '.' | '$' | '@')) {
                        break;
                    }
                    index += next.len_utf8();
                }
                ExpressionToken::Symbol(&input[start..index])
            }
            _ => {
                return Err(Diagnostic::error(
                    "CMD-003",
                    format!("invalid expression character: {character}"),
                ));
            }
        };
        tokens.push(token);
    }
    tokens.push(ExpressionToken::End);
    Ok(tokens)
}

/// Parse the command boundary shared by the host simulator and target backend
/// consoles. The command implementation deliberately receives the original
/// argument tail so assembler source and multiline programs retain their
/// spelling; `tokens` is available to command handlers that need structured
/// arguments.
pub(crate) fn parse(input: &str) -> Result<Option<CommandLine>> {
    if input.len() > MAX_COMMAND_LINE_BYTES {
        return Err(Diagnostic::error(
            "CMD-006",
            "command line exceeds the 64 KiB limit",
        ));
    }
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let name_end = trimmed
        .char_indices()
        .find(|(_, character)| character.is_ascii_whitespace())
        .map_or(trimmed.len(), |(index, _)| index);
    let name = &trimmed[..name_end];
    validate_name(name)?;

    let raw_arguments = trimmed[name_end..].trim_start().to_string();
    let tokens = tokenize(&raw_arguments)?;
    Ok(Some(CommandLine {
        name: name.to_string(),
        raw_arguments,
        tokens,
    }))
}

/// Validate only the command-level arity shared by both consoles. Operation-
/// specific syntax remains owned by the operation so its diagnostics retain
/// their memory, debugger, or persistence context.
pub(crate) fn validate_required_arguments(name: &str, count: usize) -> Result<()> {
    let required = match name.to_ascii_lowercase().as_str() {
        "assemble" | "a" | "assemble-program" | "load" | "view" | "jump" | "edit" | "e"
        | "mark" | "unmark" | "break" | "b" | "watch" | "rwatch" | "awatch" | "delete" | "del"
        | "info" | "snapshot" | "restore" | "project-save" | "project-load" | "session-save"
        | "session-load" | "find" | "fill" | "copy" => 1,
        _ => 0,
    };
    if count < required {
        return Err(Diagnostic::error(
            "CMD-002",
            format!("command {name} expects at least {required} argument(s)"),
        ));
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if name == "?" {
        return Ok(());
    }
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return Err(Diagnostic::error("CMD-003", "command name is empty"));
    };
    if !first.is_ascii_alphabetic()
        || !characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '?')
        })
    {
        return Err(Diagnostic::error(
            "CMD-003",
            "command name must be an ASCII identifier",
        ));
    }
    Ok(())
}

fn tokenize(input: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut token_started = false;

    for character in input.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            token_started = true;
            continue;
        }
        if character == '\\' {
            escaped = true;
            token_started = true;
            continue;
        }
        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            } else {
                current.push(character);
            }
            token_started = true;
            continue;
        }
        match character {
            '\'' | '"' => {
                quote = Some(character);
                token_started = true;
            }
            character if character.is_ascii_whitespace() => {
                if token_started {
                    tokens.push(std::mem::take(&mut current));
                    token_started = false;
                }
            }
            character => {
                current.push(character);
                token_started = true;
            }
        }
    }

    if escaped {
        return Err(Diagnostic::error(
            "CMD-003",
            "command argument ends with an escape character",
        ));
    }
    if quote.is_some() {
        return Err(Diagnostic::error(
            "CMD-003",
            "unterminated quoted command argument",
        ));
    }
    if token_started {
        tokens.push(current);
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_empty_lines_without_an_operation() {
        assert_eq!(parse(" \t\n ").unwrap(), None);
    }

    #[test]
    fn preserves_assembler_tail_and_exposes_tokens() {
        let line = parse("assemble addi x1,x0,1").unwrap().unwrap();
        assert_eq!(line.name, "assemble");
        assert_eq!(line.raw_arguments, "addi x1,x0,1");
        assert_eq!(line.tokens, ["addi", "x1,x0,1"]);
    }

    #[test]
    fn supports_quotes_and_escaped_delimiters() {
        let line = parse(r##"snapshot "session copy.rv" note\"1"##)
            .unwrap()
            .unwrap();
        assert_eq!(line.tokens, ["session copy.rv", "note\"1"]);
    }

    #[test]
    fn rejects_unterminated_quotes_and_escapes() {
        assert_eq!(parse("snapshot 'broken").unwrap_err().code, "CMD-003");
        assert_eq!(parse("snapshot broken\\").unwrap_err().code, "CMD-003");
    }

    #[test]
    fn rejects_invalid_command_names() {
        assert_eq!(parse("1run").unwrap_err().code, "CMD-003");
        assert_eq!(parse("run! now").unwrap_err().code, "CMD-003");
    }

    #[test]
    fn rejects_command_lines_over_the_safety_limit() {
        let input = format!("run {}", "x".repeat(MAX_COMMAND_LINE_BYTES));
        assert_eq!(parse(&input).unwrap_err().code, "CMD-006");
    }

    #[test]
    fn accepts_question_mark_alias() {
        assert_eq!(parse("?").unwrap().unwrap().name, "?");
    }

    #[test]
    fn validates_required_arguments_without_restricting_optional_commands() {
        assert_eq!(
            validate_required_arguments("view", 0).unwrap_err().code,
            "CMD-002"
        );
        assert!(validate_required_arguments("run", 0).is_ok());
        assert!(validate_required_arguments("assemble", 2).is_ok());
    }

    #[test]
    fn parses_and_evaluates_signed_precedence() {
        let expression = parse_expression("-(1 + 2) * 4 + 0x10").unwrap();
        assert_eq!(expression.evaluate(&|_| None).unwrap(), 4);
        let expression = parse_expression("1 << 3 | 2").unwrap();
        assert_eq!(expression.evaluate(&|_| None).unwrap(), 10);
    }

    #[test]
    fn resolves_symbols_and_ranges_without_host_integer_wrap() {
        let expression = parse_expression("base + 4").unwrap();
        assert_eq!(
            expression
                .evaluate(&|name| (name == "base").then_some(0x100i128))
                .unwrap(),
            0x104
        );
        let range = parse_range("0x10 .. base + 0x20").unwrap();
        assert_eq!(
            range
                .end
                .unwrap()
                .evaluate(&|name| (name == "base").then_some(0x100))
                .unwrap(),
            0x120
        );
    }

    #[test]
    fn rejects_expression_overflow_division_by_zero_and_bad_ranges() {
        let overflow = parse_expression("1 / 0").unwrap().evaluate(&|_| None);
        assert_eq!(overflow.unwrap_err().code, "CMD-003");
        let overflow = parse_expression("170141183460469231731687303715884105727 + 1")
            .unwrap()
            .evaluate(&|_| None);
        assert_eq!(overflow.unwrap_err().code, "CMD-003");
        assert_eq!(parse_range("1..2..3").unwrap_err().code, "CMD-003");
    }
}
