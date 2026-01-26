use crate::helpers::{BLUE, DIM, ESC};
use crate::lux::ext::list_help;

pub fn main() {
    // header
    println!("{BLUE}[onyx]{ESC} v0.1 {DIM}(26w04a){ESC}");
    println!("{BLUE}usage:{ESC} onyx <module> <command> [options]\n");

    // Core modules
    println!("{BLUE}Modules:{ESC}");

    // Core module list
    let core_modules = [
        ("box", "Manage Onyx boxes"),
        ("update", "Update Onyx and its components to the latest version"),
        ("profile", "Set performance profiles for Onyx boxes"),
        ("doctor", "Diagnose Onyx installation"),
        ("help", "Show this help message"),
        ("lux", "Manage Onyx extensions and plugins"),
        ("it", "Build and create Onyx boxes from rootfs images"),
    ];

    // find longest name for padding
    let max_len = core_modules.iter().map(|(name, _)| name.len()).max().unwrap_or(0);

    for (name, desc) in core_modules.iter() {
        println!("  {name:<width$}  {DIM}{desc}{ESC}", width = max_len + 2);
    }
    println!();

    // User modules (lux extensions)
    let ext_cmds = list_help();
    if !ext_cmds.is_empty() {
        println!("{BLUE}User Modules:{ESC}");
        let max_len_ext = ext_cmds.iter().map(|(name, _)| name.len()).max().unwrap_or(0);

        for (name, desc) in ext_cmds.iter() {
            println!("  {name:<width$}  {DIM}{desc}{ESC}", width = max_len_ext + 2);
        }
        println!();
    }

    println!("{BLUE}Notes:{ESC}");
    println!("  Use 'onyx <module> help' for more info on commands.");
}
