#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

const MAX_SHELL_HISTORY: usize = 256;

fn main() {
    let script = script_path();
    if let Some(port) = qemu_port() {
        qemu_interactive(port, script.as_deref());
    } else if script.is_some() || std::env::args().any(|argument| argument == "--interactive") {
        interactive(script.as_deref());
    } else {
        demo();
    }
}

fn qemu_port() -> Option<u16> {
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--qemu-port" {
            let value = arguments
                .next()
                .unwrap_or_else(|| panic!("--qemu-port expects a TCP port"));
            return Some(
                value
                    .parse()
                    .unwrap_or_else(|_| panic!("invalid QEMU TCP port: {value}")),
            );
        }
    }
    None
}

fn script_path() -> Option<String> {
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--script" {
            return Some(
                arguments
                    .next()
                    .unwrap_or_else(|| panic!("--script expects a file path")),
            );
        }
    }
    None
}

fn demo() {
    let mut monitor = luna_monitor::Monitor::new(4096);
    println!(
        "{}",
        monitor
            .execute("assemble addi x1,x0,1")
            .expect("bootstrap source must assemble")
    );
    println!("{}", monitor.execute("step").expect("addi must execute"));
    assert_eq!(monitor.machine.x[1], 1);
}

fn interactive(script: Option<&str>) {
    let stdin = io::stdin();
    let input: Box<dyn BufRead> = match script {
        Some(path) => {
            Box::new(BufReader::new(File::open(path).unwrap_or_else(|error| {
                panic!("cannot open script {path}: {error}")
            })))
        }
        None => Box::new(stdin.lock()),
    };
    let mut monitor = luna_monitor::Monitor::new(64 * 1024);
    let mut history = Vec::new();
    println!("RVMonitor interactive; type 'help' for commands (!! and !N replay history)");
    if script.is_none() {
        print!("rvmonitor> ");
        io::stdout().flush().unwrap();
    }
    for line in input.lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("input error: {error}");
                break;
            }
        };
        let line = match expand_history_line(&line, &mut history) {
            Ok(line) => line,
            Err(error) => {
                eprintln!("{error}");
                continue;
            }
        };
        let leave = matches!(line.trim(), "quit" | "exit");
        match monitor.execute(&line) {
            Ok(output) if !output.is_empty() => println!("{output}"),
            Ok(_) => {}
            Err(error) => eprintln!("{}: {}", error.code, error.message),
        }
        if leave {
            break;
        }
        if script.is_none() {
            print!("rvmonitor> ");
            io::stdout().flush().unwrap();
        }
    }
}

fn qemu_interactive(port: u16, script: Option<&str>) {
    let address = ("127.0.0.1", port);
    let backend = luna_qemu_backend::GdbRemote::connect(address)
        .unwrap_or_else(|error| panic!("cannot connect to QEMU GDB RSP: {error}"));
    let mut console = luna_monitor::BackendConsole::new(backend);
    let stdin = io::stdin();
    let input: Box<dyn BufRead> = match script {
        Some(path) => {
            Box::new(BufReader::new(File::open(path).unwrap_or_else(|error| {
                panic!("cannot open script {path}: {error}")
            })))
        }
        None => Box::new(stdin.lock()),
    };
    let mut history = Vec::new();
    println!("RVMonitor QEMU backend on 127.0.0.1:{port}; type 'help' for commands");
    if script.is_none() {
        print!("rvmonitor-qemu> ");
        io::stdout().flush().unwrap();
    }
    for line in input.lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("input error: {error}");
                break;
            }
        };
        let line = match expand_history_line(&line, &mut history) {
            Ok(line) => line,
            Err(error) => {
                eprintln!("{error}");
                continue;
            }
        };
        let leave = matches!(line.trim(), "quit" | "exit");
        match console.execute(&line) {
            Ok(output) if !output.is_empty() => println!("{output}"),
            Ok(_) => {}
            Err(error) => eprintln!("{}: {}", error.code, error.message),
        }
        if leave {
            break;
        }
        if script.is_none() {
            print!("rvmonitor-qemu> ");
            io::stdout().flush().unwrap();
        }
    }
}

fn expand_history_line(line: &str, history: &mut Vec<String>) -> Result<String, String> {
    let trimmed = line.trim();
    let expanded = if trimmed == "!!" {
        history
            .last()
            .cloned()
            .ok_or_else(|| "APP-SHELL-001: command history is empty".to_string())?
    } else if let Some(index) = trimmed.strip_prefix('!') {
        if index.is_empty() || !index.chars().all(|character| character.is_ascii_digit()) {
            return Err("APP-SHELL-002: history reference must be !! or !N".into());
        }
        let number = index
            .parse::<usize>()
            .map_err(|_| "APP-SHELL-003: invalid history number".to_string())?;
        if number == 0 || number > history.len() {
            return Err(format!(
                "APP-SHELL-004: history entry !{number} does not exist"
            ));
        }
        history[number - 1].clone()
    } else {
        line.to_string()
    };

    if !expanded.trim().is_empty() && (history.last() != Some(&expanded)) {
        if history.len() == MAX_SHELL_HISTORY {
            history.remove(0);
        }
        history.push(expanded.clone());
    }
    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_last_and_numbered_commands_with_bounded_history() {
        let mut history = Vec::new();
        assert_eq!(expand_history_line("regs", &mut history).unwrap(), "regs");
        assert_eq!(
            expand_history_line("set a0 1", &mut history).unwrap(),
            "set a0 1"
        );
        assert_eq!(expand_history_line("!!", &mut history).unwrap(), "set a0 1");
        assert_eq!(expand_history_line("!1", &mut history).unwrap(), "regs");
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn rejects_invalid_or_empty_history_references() {
        let mut history = Vec::new();
        assert_eq!(
            expand_history_line("!!", &mut history).unwrap_err(),
            "APP-SHELL-001: command history is empty"
        );
        assert_eq!(
            expand_history_line("!foo", &mut history).unwrap_err(),
            "APP-SHELL-002: history reference must be !! or !N"
        );
        assert_eq!(
            expand_history_line("!1", &mut history).unwrap_err(),
            "APP-SHELL-004: history entry !1 does not exist"
        );
    }
}
