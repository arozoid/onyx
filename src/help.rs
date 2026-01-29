use crate::helpers::{BLUE, BLUEB, DIM, ESC, errln, time_get};
use crate::lux::ext::list_help;

fn make_help(header: &str, options: Vec<(&str, &str)>) {
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
                ("create <name> <rootfs-folder>", "Create a new Onyx box"),
                ("delete <name>", "Delete an existing Onyx box"),
                ("open <name>", "Open an Onyx box in the terminal"),
                ("list", "List all existing Onyx boxes"),
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
        ("box", "Manage Onyx boxes"),
        ("update", "Update Onyx and its components to the latest version"),
        ("profile", "Set performance profiles for Onyx boxes"),
        ("doctor", "Diagnose Onyx installation"),
        ("help", "Show this help message"),
        // ("lux", "Manage Onyx extensions and plugins"),
    ];

    make_help("Core Modules:", core_modules);
    println!();

    // User modules (lux extensions)
    let ext_cmds = list_help();
    if !ext_cmds.is_empty() {
        make_help("User Modules:", ext_cmds.iter().map(|(n, d)| (n.as_str(), d.as_str())).collect());
        println!();
    }

    println!("{BLUE}Notes:{ESC}");
    println!("  Use 'onyx <module> help' for more info on commands.");
}
