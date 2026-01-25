use crate::doctor::{self, VERSION};
use crate::helpers::{errln, infoln, BLUE, DIM, ESC, RED, GREEN, YELLOW, download, file_exists, ONYX_DIR};
use std::path::Path;

pub fn cmd(args: Vec<String>) {
    infoln("onyx", "checking onyx health...");
    let (
        kv, 
        mv, 
        root, 
        box64, 
        proot, 
        glibc, 
        arch, 
        latest_version
    ) = doctor::cmd();

    if file_exists("/home/onyx") {

    }

    if (VERSION != latest_version && !latest_version.is_empty())
        && !args.contains(&"--ignore-onyx".to_string())
        && args.contains(&"--force".to_string())
    {
        infoln("onyx", "updating onyx...");
        download("https://raw.githubusercontent.com/arozoid/onyx/refs/heads/main/onyx", ONYX_DIR.join("onyx").to_str().unwrap()).unwrap();
    }

    download("https://www.gutenberg.org/files/135/135-0.txt", ONYX_DIR.join("version.txt").to_str().unwrap()).unwrap();
}