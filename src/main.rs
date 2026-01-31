mod lux;
mod normalize;
mod r#box;
mod helpers;
mod doctor;
mod help;
mod update;
mod cpu;
mod profile;

use crate::helpers::{ONYX_DIR, check_file_authority};
use crate::helpers::{errln, infoln};

use std::env;
use std::process;

//=== cli ===//
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        errln("onyx", "no command provided");
        help::cmd(args);
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
            help::cmd(args);
        }
        "profile" => {
            profile::cmd(args);
        }
        "normalize" => {
            let perms = check_file_authority(&ONYX_DIR).unwrap();
            if perms.0 == true || perms.1 == true {
                infoln("doctor", "normalizing onyx file permissions...");
                normalize::normalize_onyx_dir().unwrap();
            } else {
                errln("doctor", "this user cannot normalize onyx.");
            }
        }
        _ => {
            errln("onyx", &format!("unknown command: {}", command));
            help::cmd(args);
            process::exit(1);
        }
    }

    process::exit(0);
}