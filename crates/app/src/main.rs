#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{self, BufRead, BufReader, IsTerminal, Write};

use crossterm::cursor::MoveToColumn;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{self, Clear, ClearType};
use luna_snapshot_format::GuestCommandTransport;

const MAX_SHELL_HISTORY: usize = 256;

fn main() {
    if let Some(operation) = guest_snapshot_options() {
        match operation {
            GuestSnapshotOperation::Export { port, output } => export_guest_snapshot(port, &output),
            GuestSnapshotOperation::ExportProject { port, output } => {
                export_guest_project(port, &output)
            }
            GuestSnapshotOperation::Import { port, input } => import_guest_snapshot(port, &input),
            GuestSnapshotOperation::ImportProject { port, input } => {
                import_guest_project(port, &input)
            }
        }
        return;
    }
    let script = script_path();
    if let Some(port) = qemu_port() {
        qemu_interactive(port, script.as_deref());
    } else if script.is_some() || std::env::args().any(|argument| argument == "--interactive") {
        interactive(script.as_deref());
    } else {
        demo();
    }
}

enum GuestSnapshotOperation {
    Export { port: u16, output: String },
    ExportProject { port: u16, output: String },
    Import { port: u16, input: String },
    ImportProject { port: u16, input: String },
}

fn guest_snapshot_options() -> Option<GuestSnapshotOperation> {
    let mut arguments = std::env::args().skip(1);
    let mut port = None;
    let mut output = None;
    let mut input = None;
    let mut project = None;
    let mut project_input = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--guest-uart-port" => {
                let value = arguments
                    .next()
                    .unwrap_or_else(|| panic!("--guest-uart-port expects a TCP port"));
                port = Some(
                    value
                        .parse()
                        .unwrap_or_else(|_| panic!("invalid guest UART TCP port: {value}")),
                );
            }
            "--snapshot-out" => {
                output = Some(
                    arguments
                        .next()
                        .unwrap_or_else(|| panic!("--snapshot-out expects a file path")),
                );
            }
            "--snapshot-in" => {
                input = Some(
                    arguments
                        .next()
                        .unwrap_or_else(|| panic!("--snapshot-in expects a file path")),
                );
            }
            "--project-out" => {
                project = Some(
                    arguments
                        .next()
                        .unwrap_or_else(|| panic!("--project-out expects a file path")),
                );
            }
            "--project-in" => {
                project_input = Some(
                    arguments
                        .next()
                        .unwrap_or_else(|| panic!("--project-in expects a file path")),
                );
            }
            _ => {}
        }
    }
    match (port, output, input, project, project_input) {
        (Some(port), Some(output), None, None, None) => {
            Some(GuestSnapshotOperation::Export { port, output })
        }
        (Some(port), None, None, Some(project), None) => {
            Some(GuestSnapshotOperation::ExportProject {
                port,
                output: project,
            })
        }
        (Some(port), None, Some(input), None, None) => {
            Some(GuestSnapshotOperation::Import { port, input })
        }
        (Some(port), None, None, None, Some(input)) => {
            Some(GuestSnapshotOperation::ImportProject { port, input })
        }
        (None, None, None, None, None) => None,
        _ => panic!(
            "--guest-uart-port must be combined with exactly one snapshot/project input or output"
        ),
    }
}

fn export_guest_snapshot(port: u16, output: &str) {
    let mut transport =
        luna_snapshot_format::TcpGuestCommandTransport::connect(("127.0.0.1", port))
            .unwrap_or_else(|error| panic!("cannot connect to guest UART: {error}"));
    let save_response = transport
        .command("snapshot save")
        .unwrap_or_else(|error| panic!("cannot save guest snapshot: {error}"));
    if save_response.contains("error [") {
        panic!("guest snapshot save failed: {save_response}");
    }
    let image = luna_snapshot_format::fetch_guest_snapshot(&mut transport)
        .unwrap_or_else(|error| panic!("cannot fetch guest snapshot: {error:?}"));
    let encoded = image
        .encode()
        .unwrap_or_else(|error| panic!("cannot encode guest snapshot: {error}"));
    std::fs::write(output, encoded)
        .unwrap_or_else(|error| panic!("cannot write snapshot {output}: {error}"));
    println!(
        "guest snapshot exported to {output} (workspace={} data={} source-lines={})",
        image.workspace.len(),
        image.data.len(),
        image.source_lines
    );
}

fn export_guest_project(port: u16, output: &str) {
    let mut transport =
        luna_snapshot_format::TcpGuestCommandTransport::connect(("127.0.0.1", port))
            .unwrap_or_else(|error| panic!("cannot connect to guest UART: {error}"));
    let save_response = transport
        .command("snapshot save")
        .unwrap_or_else(|error| panic!("cannot save guest snapshot: {error}"));
    if save_response.contains("error [") {
        panic!("guest snapshot save failed: {save_response}");
    }
    let image = luna_snapshot_format::fetch_guest_snapshot(&mut transport)
        .unwrap_or_else(|error| panic!("cannot fetch guest snapshot: {error:?}"));
    let metadata = luna_snapshot_format::fetch_guest_metadata(&mut transport)
        .unwrap_or_else(|error| panic!("cannot fetch guest metadata: {error:?}"));
    let project = luna_snapshot_format::SnapshotProject { image, metadata };
    let encoded = project
        .encode()
        .unwrap_or_else(|error| panic!("cannot encode guest project: {error:?}"));
    std::fs::write(output, encoded)
        .unwrap_or_else(|error| panic!("cannot write project {output}: {error}"));
    println!(
        "guest project exported to {output} (metadata source={} symbols={})",
        project.metadata.source.len(),
        project.metadata.symbols.len()
    );
}

fn import_guest_snapshot(port: u16, input: &str) {
    let encoded = std::fs::read(input)
        .unwrap_or_else(|error| panic!("cannot read snapshot {input}: {error}"));
    let image = luna_snapshot_format::SnapshotImage::decode(&encoded)
        .unwrap_or_else(|error| panic!("cannot decode snapshot {input}: {error}"));
    let mut transport =
        luna_snapshot_format::TcpGuestCommandTransport::connect(("127.0.0.1", port))
            .unwrap_or_else(|error| panic!("cannot connect to guest UART: {error}"));
    let save_response = transport
        .command("snapshot save")
        .unwrap_or_else(|error| panic!("cannot initialize guest snapshot slot: {error}"));
    if save_response.contains("error [") {
        panic!("guest snapshot slot initialization failed: {save_response}");
    }
    luna_snapshot_format::apply_guest_snapshot(&mut transport, &image)
        .unwrap_or_else(|error| panic!("cannot apply guest snapshot: {error:?}"));
    println!(
        "guest snapshot imported from {input} (workspace={} data={} source-lines={})",
        image.workspace.len(),
        image.data.len(),
        image.source_lines
    );
}

fn import_guest_project(port: u16, input: &str) {
    let encoded =
        std::fs::read(input).unwrap_or_else(|error| panic!("cannot read project {input}: {error}"));
    let project = luna_snapshot_format::SnapshotProject::decode(&encoded)
        .unwrap_or_else(|error| panic!("cannot decode project {input}: {error:?}"));
    let mut transport =
        luna_snapshot_format::TcpGuestCommandTransport::connect(("127.0.0.1", port))
            .unwrap_or_else(|error| panic!("cannot connect to guest UART: {error}"));
    let save_response = transport
        .command("snapshot save")
        .unwrap_or_else(|error| panic!("cannot initialize guest snapshot slot: {error}"));
    if save_response.contains("error [") {
        panic!("guest snapshot slot initialization failed: {save_response}");
    }
    luna_snapshot_format::apply_guest_project(&mut transport, &project)
        .unwrap_or_else(|error| panic!("cannot apply guest project: {error:?}"));
    println!("guest project imported from {input}");
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
    let mut monitor = luna_monitor::Monitor::new(64 * 1024);
    let mut history = Vec::new();
    if script.is_none() && io::stdin().is_terminal() {
        interactive_tty("rvmonitor> ", &mut history, |line| {
            monitor.execute(line).map_err(|error| {
                let detail = monitor
                    .execute("diagnostic")
                    .unwrap_or_else(|_| "diagnostic unavailable".into());
                format!("{}: {}\n{detail}", error.code, error.message)
            })
        });
        return;
    }
    let stdin = io::stdin();
    let input: Box<dyn BufRead> = match script {
        Some(path) => {
            Box::new(BufReader::new(File::open(path).unwrap_or_else(|error| {
                panic!("cannot open script {path}: {error}")
            })))
        }
        None => Box::new(stdin.lock()),
    };
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
            Err(error) => {
                eprintln!("{}: {}", error.code, error.message);
                if let Ok(detail) = monitor.execute("diagnostic") {
                    eprintln!("{detail}");
                }
            }
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
    let mut history = Vec::new();
    if script.is_none() && io::stdin().is_terminal() {
        interactive_tty("rvmonitor-qemu> ", &mut history, |line| {
            console.execute(line).map_err(|error| {
                let detail = console
                    .execute("diagnostic")
                    .unwrap_or_else(|_| "diagnostic unavailable".into());
                format!("{}: {}\n{detail}", error.code, error.message)
            })
        });
        return;
    }
    let stdin = io::stdin();
    let input: Box<dyn BufRead> = match script {
        Some(path) => {
            Box::new(BufReader::new(File::open(path).unwrap_or_else(|error| {
                panic!("cannot open script {path}: {error}")
            })))
        }
        None => Box::new(stdin.lock()),
    };
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
            Err(error) => {
                eprintln!("{}: {}", error.code, error.message);
                if let Ok(detail) = console.execute("diagnostic") {
                    eprintln!("{detail}");
                }
            }
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

fn interactive_tty<F>(prompt: &str, history: &mut Vec<String>, mut execute_command: F)
where
    F: FnMut(&str) -> Result<String, String>,
{
    if let Err(error) = terminal::enable_raw_mode() {
        eprintln!("APP-TTY-001: cannot enable raw terminal mode: {error}");
        return;
    }
    let _raw_mode = RawModeGuard;
    println!("interactive keyboard: arrows edit/history, Ctrl-D exits");
    loop {
        let line = match read_tty_line(prompt, history) {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(error) => {
                eprintln!("APP-TTY-002: terminal input failed: {error}");
                break;
            }
        };
        let line = match expand_history_line(&line, history) {
            Ok(line) => line,
            Err(error) => {
                println!("\r\x1b[K{error}");
                continue;
            }
        };
        let leave = matches!(line.trim(), "quit" | "exit");
        match execute_command(&line) {
            Ok(output) if !output.is_empty() => println!("\r\x1b[K{output}"),
            Ok(_) => {}
            Err(error) => println!("\r\x1b[K{error}"),
        }
        if leave {
            break;
        }
    }
}

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        println!();
    }
}

fn read_tty_line(prompt: &str, history: &[String]) -> io::Result<Option<String>> {
    let mut stdout = io::stdout();
    let mut line = String::new();
    let mut cursor = 0usize;
    let mut history_index = None;
    let mut draft = String::new();
    render_tty_line(&mut stdout, prompt, &line, cursor)?;
    loop {
        match event::read()? {
            Event::Key(KeyEvent {
                code,
                modifiers,
                kind: _,
                state: _,
            }) => match (code, modifiers) {
                _ if shortcut_command(code, modifiers).is_some() => {
                    println!();
                    return Ok(shortcut_command(code, modifiers).map(str::to_string));
                }
                (KeyCode::Enter, _) => {
                    println!();
                    return Ok(Some(line));
                }
                (KeyCode::Char('d'), KeyModifiers::CONTROL) if line.is_empty() => {
                    println!();
                    return Ok(None);
                }
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    line.clear();
                    cursor = 0;
                    history_index = None;
                    render_tty_line(&mut stdout, prompt, &line, cursor)?;
                }
                (KeyCode::Char('g'), KeyModifiers::CONTROL) => {
                    line.insert_str(cursor, "view ");
                    cursor += "view ".len();
                    history_index = None;
                    render_tty_line(&mut stdout, prompt, &line, cursor)?;
                }
                (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                    line.insert_str(cursor, "find ");
                    cursor += "find ".len();
                    history_index = None;
                    render_tty_line(&mut stdout, prompt, &line, cursor)?;
                }
                (KeyCode::Backspace, _) if cursor > 0 => {
                    let start = line[..cursor]
                        .char_indices()
                        .last()
                        .map_or(0, |(index, _)| index);
                    line.replace_range(start..cursor, "");
                    cursor = start;
                    history_index = None;
                    render_tty_line(&mut stdout, prompt, &line, cursor)?;
                }
                (KeyCode::Delete, _) if cursor < line.len() => {
                    let end = line[cursor..]
                        .char_indices()
                        .nth(1)
                        .map_or(line.len(), |(index, _)| cursor + index);
                    line.replace_range(cursor..end, "");
                    history_index = None;
                    render_tty_line(&mut stdout, prompt, &line, cursor)?;
                }
                (KeyCode::Left, _) if cursor > 0 => {
                    cursor = line[..cursor]
                        .char_indices()
                        .last()
                        .map_or(0, |(index, _)| index);
                    render_tty_line(&mut stdout, prompt, &line, cursor)?;
                }
                (KeyCode::Right, _) if cursor < line.len() => {
                    cursor += line[cursor..].chars().next().map_or(0, char::len_utf8);
                    render_tty_line(&mut stdout, prompt, &line, cursor)?;
                }
                (KeyCode::Home, _) => {
                    cursor = 0;
                    render_tty_line(&mut stdout, prompt, &line, cursor)?;
                }
                (KeyCode::End, _) => {
                    cursor = line.len();
                    render_tty_line(&mut stdout, prompt, &line, cursor)?;
                }
                (KeyCode::Up, _) if !history.is_empty() => {
                    if history_index.is_none() {
                        draft = line.clone();
                        history_index = Some(history.len() - 1);
                    } else if history_index.unwrap() > 0 {
                        history_index = Some(history_index.unwrap() - 1);
                    }
                    line = history[history_index.unwrap()].clone();
                    cursor = line.len();
                    render_tty_line(&mut stdout, prompt, &line, cursor)?;
                }
                (KeyCode::Down, _) => {
                    if let Some(index) = history_index {
                        if index + 1 < history.len() {
                            history_index = Some(index + 1);
                            line = history[index + 1].clone();
                        } else {
                            history_index = None;
                            line = draft.clone();
                        }
                        cursor = line.len();
                        render_tty_line(&mut stdout, prompt, &line, cursor)?;
                    }
                }
                (KeyCode::Char(character), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    line.insert(cursor, character);
                    cursor += character.len_utf8();
                    history_index = None;
                    render_tty_line(&mut stdout, prompt, &line, cursor)?;
                }
                _ => {}
            },
            Event::Resize(_, _) => render_tty_line(&mut stdout, prompt, &line, cursor)?,
            _ => {}
        }
    }
}

fn shortcut_command(code: KeyCode, modifiers: KeyModifiers) -> Option<&'static str> {
    match (code, modifiers) {
        (KeyCode::F(5), _) => Some("run"),
        (KeyCode::F(10), _) => Some("step-over"),
        (KeyCode::F(11), _) => Some("step-out"),
        (KeyCode::Char('1'), KeyModifiers::CONTROL) => Some("regs"),
        (KeyCode::Char('2'), KeyModifiers::CONTROL) => Some("memory"),
        (KeyCode::Char('3'), KeyModifiers::CONTROL) => Some("dashboard"),
        _ => None,
    }
}

fn render_tty_line(
    stdout: &mut io::Stdout,
    prompt: &str,
    line: &str,
    cursor: usize,
) -> io::Result<()> {
    execute!(
        stdout,
        MoveToColumn(0),
        Print(prompt),
        Print(line),
        Clear(ClearType::UntilNewLine),
        MoveToColumn((prompt.len() + cursor).min(u16::MAX as usize) as u16)
    )?;
    stdout.flush()
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

    #[test]
    fn maps_tty_shortcuts_to_existing_commands() {
        assert_eq!(
            shortcut_command(KeyCode::F(5), KeyModifiers::NONE),
            Some("run")
        );
        assert_eq!(
            shortcut_command(KeyCode::F(10), KeyModifiers::NONE),
            Some("step-over")
        );
        assert_eq!(
            shortcut_command(KeyCode::F(11), KeyModifiers::NONE),
            Some("step-out")
        );
        assert_eq!(
            shortcut_command(KeyCode::Char('1'), KeyModifiers::CONTROL),
            Some("regs")
        );
        assert_eq!(
            shortcut_command(KeyCode::Char('2'), KeyModifiers::CONTROL),
            Some("memory")
        );
        assert_eq!(
            shortcut_command(KeyCode::Char('3'), KeyModifiers::CONTROL),
            Some("dashboard")
        );
        assert_eq!(
            shortcut_command(KeyCode::Char('x'), KeyModifiers::NONE),
            None
        );
    }
}
