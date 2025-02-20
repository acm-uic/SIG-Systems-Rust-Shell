mod lexer;
mod parser;
mod safe_wrappers;

use safe_wrappers::{exec, fork, wait, ForkReturn};

#[cfg(test)]
mod tests;

use std::io::{self, Write};

use parser::{Arg, Command};

fn main() {
    // Input REPL
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("> ");
        stdout.flush().unwrap();

        let mut input = String::new();
        stdin.read_line(&mut input).unwrap();
        let input = input.trim();

        if input == "exit" {
            break;
        }

        let command = Command::parse(input).unwrap();
        match run_command(&command) {
            Ok(_) => (),
            Err(e) => eprintln!("{}", e),
        }
    }
}

use crate::safe_wrappers::WaitStatus;
fn run_command(cmd: &Command) -> io::Result<WaitStatus> {
    match fork() {
        ForkReturn::Child => {
            let args = cmd
                .argv
                .iter()
                .filter_map(|arg| {
                    if let Arg::Word(w) = arg {
                        Some(w)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            if args.len() == 0 {
                return Ok(WaitStatus::Exited(0));
            }

            if let Err(e) = exec(&args[0], &args.as_slice()) {
                Err(io::Error::from(e))
            } else {
                unsafe { std::hint::unreachable_unchecked() };
            }
        }
        ForkReturn::Parent(_) => Ok(wait()?.into()),
    }
}
