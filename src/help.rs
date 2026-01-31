use crate::helpers::{BLUE, YELLOW, DIM, ESC, errln, time_get, infoln};
use crate::lux::ext::list_help;

fn visible_len(s: &str) -> usize {
    // remove ANSI escape codes like \x1b[...m
    let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").len()
}

pub fn make_help(header: &str, options: Vec<(String, String)>) {
    println!("{BLUE}{header}{ESC}");

    // max visible width of first lines
    let max_len = options.iter().map(|(opt, _)| {
        opt.lines().next().map(|s| s.len()).unwrap_or(0)
    }).max().unwrap_or(0);

    for (opt, desc) in options.iter() {
        let mut lines = opt.lines();

        if let Some(first_line) = lines.next() {
            // split first/rest for coloring
            let mut parts = first_line.splitn(2, ' ');
            let first = parts.next().unwrap_or("");
            let rest = parts.next().unwrap_or("");

            let colored = if rest.is_empty() {
                first.to_string()
            } else {
                format!("{} {BLUE}{}{}", first, rest, ESC)
            };

            // calculate real padding ignoring ANSI
            let pad = max_len.saturating_sub(visible_len(&colored));

            println!("  {}{:pad$}  {DIM}{}{ESC}", colored, "", desc, pad=pad);
        }

        // remaining lines: just yellow, aligned under option column
        for line in lines {
            println!("    {YELLOW}{:<width$}{ESC}", line, width = max_len);
        }
    }
}

fn sub_cmd(args: Vec<String>) {
    match args[2].as_str() {
        "box" => {
            let r#box = vec![
                ("delete <name>".to_string(), "Delete an existing Onyx box".to_string()),

                ("open <name>\n --profile=PROFILE".to_string(), 
                "Open an Onyx box in the terminal".to_string()),

                ("exec <name>\n --profile=PROFILE".to_string(), 
                "Execute a single command within the Onyx box".to_string()),
                
                ("create <name> <rootfs-folder>\n <move: TRUE/FALSE>".to_string(), 
                "Create a new Onyx box from an existing rootfs".to_string()),

                ("list".to_string(), "List all existing Onyx boxes".to_string()),
            ];
            make_help("Box Modules:", r#box);
            println!();
            infoln("help", "--profile=PROFILE is optional, see 'onyx help profile' for info");
        }
        "update" => {
            let update = vec![
                ("--force".to_string(), "Force update everything even if no updates are available".to_string()),
                ("--force-aarch64".to_string(), "Force ARM/AARCH64 software".to_string()),
                ("--force-x86_64".to_string(), "Force x86_64 software".to_string()),
                ("--ignore-onyx".to_string(), "Ignore Onyx updates".to_string()),
                ("--ignore-proot".to_string(), "Ignore PRoot updates".to_string()),
                ("--ignore-box64".to_string(), "Ignore Box64 updates".to_string()),
            ];
            make_help("Update Modules:", update);
        }
        "profile" => {
            let profile = vec![
                ("list".to_string(), "List all available performance profiles".to_string()),
                ("use <profile>".to_string(), "Use a specific performance profile".to_string()),

                ("edit <profile> \n--name=NAME --description=DESCRIPTION --mem=TYPE:VALUE --nice=NICENESS --cores=CPU_CORES".to_string(), 
                "Edit an existing performance profile".to_string()),

                ("create <profile>\n--name=NAME --description=DESCRIPTION --mem=TYPE:VALUE --nice=NICENESS --cores=CPU_CORES".to_string(), 
                "Create your own performance profile".to_string()),

                ("delete <profile>".to_string(), "Delete a performance profile".to_string()),
            ];
            make_help("Profile Modules:", profile);
            println!();
            infoln("help", "the flags provided by 'edit' and 'create' are completely optional;");
            infoln("help", "not using them either doesn't change the selected profile, or uses the default settings");
            println!();
            infoln("help", "examples of --mem=TYPE:VALUE usage:");
            println!();
            println!("{YELLOW}--mem=unlimited");
            println!("--mem=percent:75");
            println!("--mem=fixed:1024{ESC}");
            println!();
            infoln("help", "(fixed is in megabytes)");
        }
        "doctor" => {
            println!("{BLUE}usage:{ESC}");
            println!("  onyx doctor\n");
        }
        "help" => {
            println!("{BLUE}usage:{ESC}");
            println!("  onyx help <module>");
            println!("  onyx help");
        }
        _ => {
            errln("onyx", format!("unknown module: {} {DIM}[{}]{ESC}", args[1], time_get()).as_str());
        }
    }
}

pub fn cmd(args: Vec<String>) {
    // header 1
    println!("{BLUE}[onyx]{ESC} v0.1.2 {DIM}{ESC}");

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
