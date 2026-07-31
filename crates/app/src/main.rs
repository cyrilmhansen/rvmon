#![forbid(unsafe_code)]

use std::io::{self, BufRead, Write};

fn main() {
    if let Some(port) = qemu_port() {
        qemu_interactive(port);
    } else if std::env::args().any(|argument| argument == "--interactive") {
        interactive();
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

fn interactive() {
    let stdin = io::stdin();
    let mut monitor = luna_monitor::Monitor::new(64 * 1024);
    println!("RVMonitor interactive; type 'help' for commands");
    print!("rvmonitor> ");
    io::stdout().flush().unwrap();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("input error: {error}");
                break;
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
        print!("rvmonitor> ");
        io::stdout().flush().unwrap();
    }
}

fn qemu_interactive(port: u16) {
    let address = ("127.0.0.1", port);
    let backend = luna_qemu_backend::GdbRemote::connect(address)
        .unwrap_or_else(|error| panic!("cannot connect to QEMU GDB RSP: {error}"));
    let mut console = luna_monitor::BackendConsole::new(backend);
    let stdin = io::stdin();
    println!("RVMonitor QEMU backend on 127.0.0.1:{port}; type 'help' for commands");
    print!("rvmonitor-qemu> ");
    io::stdout().flush().unwrap();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("input error: {error}");
                break;
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
        print!("rvmonitor-qemu> ");
        io::stdout().flush().unwrap();
    }
}
