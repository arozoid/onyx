mod lux;
mod r#box;
mod helpers;
mod doctor;
mod help;

use crate::helpers::{errln, time_get, BLUE, DIM, ESC, RED};

use std::env;
use std::process;

//=== cli ===//
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        errln("core.onyx", "no command provided");
        help::main();
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "box" => {
            r#box::cmd(args);
        }
        "doctor" => {
            doctor::cmd(args);
        }
        _ => {
            errln("onyx", &format!("unknown command: {}", command));
            help::main();
            process::exit(1);
        }
    }

    process::exit(0);
}