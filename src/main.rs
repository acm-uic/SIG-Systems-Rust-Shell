mod lexer;
use lexer::Scanner;
use std::io::stdin;

fn main() {
    println!("Hello, world!");
    loop {
        let mut command = String::new();
        if let Err(e) = std::io::Stdin::read_line(&stdin(), &mut command) {
            eprintln!("{}", e);
        }
        let scan = Scanner::new(command);
        println!("Got: {:?}", scan.get_tokens());
    }
}
