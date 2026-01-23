use crate::helpers::{errln, time_get, BLUE, DIM, ESC, RED};

use std::process;

pub fn cmd(args: Vec<String>) {
    let command = &args[2];

    match command.as_str() {
        
        _ => {
            errln("onyx", &format!("unknown command: {}", command));
            process::exit(1);
        }
    }
}