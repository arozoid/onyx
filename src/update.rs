use crate::doctor::{self, VERSION};
use crate::helpers::{infoln, download, file_exists, ONYX_DIR};
use std::process::Command;
use std::os::unix::fs::PermissionsExt;

pub fn cmd(args: Vec<String>) {
    infoln("onyx", "checking onyx health...");
    let (
        _kv, 
        _mv, 
        _root, 
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
        Command::new("chmod")
                .args(["+x", ONYX_DIR.join("onyx").to_str().unwrap()])
                .status()
                .unwrap();
    }

    if (!proot
        && !args.contains(&"--ignore-proot".to_string()))
        || args.contains(&"--force".to_string())
    {
        infoln("onyx", "installing proot...");
        if arch == "x86_64" {
            download("https://proot.gitlab.io/proot/bin/proot", ONYX_DIR.join("bin/proot").to_str().unwrap()).unwrap();
        } else {
            download("https://skirsten.github.io/proot-portable-android-binaries/aarch64/proot", ONYX_DIR.join("bin/proot").to_str().unwrap()).unwrap();
            // make executable
            let mut perms = std::fs::metadata(ONYX_DIR.join("bin/proot")).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(ONYX_DIR.join("bin/proot"), perms).unwrap();
        }
    }

    if arch == "aarch64" && ((!box64 || !glibc) 
        && !args.contains(&"--ignore-proot".to_string()))
        || args.contains(&"--force".to_string())
    {
        infoln("onyx", "installing box64/glibc...");
    }
}