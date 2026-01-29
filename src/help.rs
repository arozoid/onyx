use crate::helpers::{BLUE, BLUEB, DIM, ESC, errln, time_get};
use crate::lux::ext::list_help;

pub fn make_help(header: &str, options: Vec<(String, String)>) {
    // header
    println!("{BLUEB}{header}{ESC}");

    // find longest option for padding
    let max_len = options.iter().map(|(opt, _)| opt.len()).max().unwrap_or(0);

    for (opt, desc) in options.iter() {
        // split opt into first word and rest
        let mut parts = opt.splitn(2, ' ');
        let first = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("");

        if rest.is_empty() {
            // no space in opt
            println!("  {first:<width$}  {DIM}{desc}{ESC}", width = max_len + 2);
        } else {
            // space exists, print first normally, rest in blue
            let colored_opt = format!("{first} {BLUE}{rest}{ESC}");
            println!("  {colored_opt:<width$}  {DIM}{desc}{ESC}", width = max_len + 2);
        }
    }
}


fn sub_cmd(args: Vec<String>) {
    match args[2].as_str() {
        "box" => {
            let r#box = vec![
                ("create <name> <rootfs-folder>".to_string(), "Create a new Onyx box".to_string()),
                ("delete <name>".to_string(), "Delete an existing Onyx box".to_string()),
                ("open <name>".to_string(), "Open an Onyx box in the terminal".to_string()),
                ("list".to_string(), "List all existing Onyx boxes".to_string()),
            ];
            make_help("Box Modules:", r#box);
        }
        "update" => {
            
        }
        "profile" => {
            
        }
        "doctor" => {
            
        }
        "help" => {

        }
        _ => {
            errln("onyx", format!("unknown module: {} {DIM}[{}]{ESC}", args[1], time_get()).as_str());
        }
    }
}

pub fn cmd(args: Vec<String>) {
    // header 1
    println!("{BLUE}[onyx]{ESC} v0.1.0 {DIM}(RC 2){ESC}");

    if args.len() > 2 {
        sub_cmd(args);
        return;
    }

    // header 2
    println!("{BLUE}usage:{ESC} onyx <module> <command> [options]\n");

    // Core module list
    let core_modules = vec![
        ("box".to_string(), "Manage Onyx boxes".to_string()),
        ("update".to_string(), "Update Onyx and its components to the latest version".to_string()),
        ("profile".to_string(), "Set performance profiles for Onyx boxes".to_string()),
        ("doctor".to_string(), "Diagnose Onyx installation".to_string()),
        ("help".to_string(), "Show this help message".to_string()),
        // ("lux", "Manage Onyx extensions and plugins"),
    ];

    make_help("Core Modules:", core_modules);
    println!();

    // User modules (lux extensions)
    let ext_cmds = list_help();
    if !ext_cmds.is_empty() {
        make_help("User Modules:", ext_cmds);
        println!();
    }

    println!("{BLUE}Notes:{ESC}");
    println!("  Use 'onyx <module> help' for more info on commands.");
}
