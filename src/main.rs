use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Error: No command provided to extension.");
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        _ => {
            println!("{}", "hello from .xeo extension!");
        }
    }

    process::exit(0);
}