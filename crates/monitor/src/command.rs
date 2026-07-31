use luna_diag::{Diagnostic, Result};

const MAX_COMMAND_LINE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommandLine {
    pub(crate) name: String,
    pub(crate) raw_arguments: String,
    pub(crate) tokens: Vec<String>,
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
}
