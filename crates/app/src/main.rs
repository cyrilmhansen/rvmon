#![forbid(unsafe_code)]

use std::io::{self, BufRead, Write};

fn main() {
    if std::env::args().any(|argument| argument == "--interactive") {
        interactive();
    } else {
        demo();
    }
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
