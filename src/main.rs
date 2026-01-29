mod lux;
mod r#box;
mod helpers;
mod doctor;
mod help;
mod update;
mod cpu;

use crate::helpers::{errln};

use std::env;
use std::process;
use std::path::Path;

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
        "help" => {
            help::main();
        }
        // "it" => {
        //     if args.len() < 4 {
        //         errln("onyxit", "usage: onyx it <source> <name>");
        //         errln("onyxit", "see 'onyx help it' for more info");
        //         process::exit(1);
        //     }

        //     let src = &args[2];
        //     let name = &args[3];
        //     if let Err(e) = it::cmd(Path::new(&src), &name) {
        //         errln("onyxit", &format!("failed to import system: {}", e));
        //         process::exit(1);
        //     }
        // }
        _ => {
            errln("onyx", &format!("unknown command: {}", command));
            help::main();
            process::exit(1);
        }
    }

    process::exit(0);
}