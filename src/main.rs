mod lux;
mod r#box;
mod helpers;
mod doctor;
mod help;
mod update;

use crate::helpers::{errln};

use std::env;
use std::process;

//=== cli ===//
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        errln("onyx", "no command provided");
        help::main();
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "box" => {
            r#box::cmd(args);
        }
        "doctor" => {
            let _ = doctor::cmd();
        }
        "update" => {
            update::cmd(args);
        }
        _ => {
            errln("onyx", &format!("unknown command: {}", command));
            help::main();
            process::exit(1);
        }
    }

    process::exit(0);
}