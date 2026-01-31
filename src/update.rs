use crate::doctor::{self, VERSION};
use crate::helpers::{infoln, download, ONYX_DIR};
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
        latest_version,
        fuse_overlay
    ) = doctor::cmd();
    let _ = ONYX_DIR;

    let arch = if args.contains(&"--force-aarch64".to_string()) {
        "aarch64"
    } else if args.contains(&"--force-x86_64".to_string()) {
        "x86_64"
    } else {
        arch.as_str()
    };

    if (VERSION != latest_version && !latest_version.is_empty())
        && !args.contains(&"--ignore-onyx".to_string())
        || args.contains(&"--force".to_string())
    {
        infoln("onyx", "updating onyx...");
        if arch == "aarch64" {
            download("https://github.com/arozoid/onyx/releases/latest/download/onyx-aarch64", ONYX_DIR.join("bin/core/onyx").to_str().unwrap()).unwrap();
        } else {
            download("https://github.com/arozoid/onyx/releases/latest/download/onyx-x86_64", ONYX_DIR.join("bin/core/onyx").to_str().unwrap()).unwrap();
        }
        Command::new("chmod")
                .args(["+x", ONYX_DIR.join("onyx").to_str().unwrap()])
                .status()
                .unwrap();
    }

    if (!proot
        && !args.contains(&"--ignore-proot".to_string()))
        || args.contains(&"--force".to_string())
    {
        let proot_path = ONYX_DIR.join("bin/proot");
        infoln("onyx", "installing proot...");
        
        // download proot
        if arch == "x86_64" {
            download(
                "https://proot.gitlab.io/proot/bin/proot",
                proot_path.to_str().unwrap(),
            )
            .unwrap();
        } else {
            download(
                "https://skirsten.github.io/proot-portable-android-binaries/aarch64/proot",
                proot_path.to_str().unwrap(),
            )
            .unwrap();
        }

        // make executable (do this **after** download)
        let mut perms = std::fs::metadata(&proot_path)
            .expect("failed to get metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&proot_path, perms)
            .expect("failed to set executable permissions");
    }

    #[cfg(not(target_os = "android"))]
    {
        if (!fuse_overlay
            && !args.contains(&"--ignore-fuse-overlayfs".to_string()))
            || args.contains(&"--force".to_string())
        {
            let fuse_overlayfs_path = ONYX_DIR.join("bin/fuse-overlayfs");
            infoln("onyx", "installing fuse-overlayfs...");
            
            // download proot
            if arch == "x86_64" {
                download(
                    "https://github.com/containers/fuse-overlayfs/releases/latest/download/fuse-overlayfs-x86_64",
                    fuse_overlayfs_path.to_str().unwrap(),
                )
                .unwrap();
            } else {
                download(
                    "https://github.com/containers/fuse-overlayfs/releases/latest/download/fuse-overlayfs-aarch64",
                    fuse_overlayfs_path.to_str().unwrap(),
                )
                .unwrap();
            }

            // make executable (do this **after** download)
            let mut perms = std::fs::metadata(&fuse_overlayfs_path)
                .expect("failed to get metadata")
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&fuse_overlayfs_path, perms)
                .expect("failed to set executable permissions");
        }
    }   

    if arch == "aarch64" && ((!box64 || !glibc) 
        && !args.contains(&"--ignore-box64".to_string())
        || args.contains(&"--force".to_string()))
    {
        infoln("onyx", "installing box64/glibc...");
        infoln("onyx", format!("failed to install. x86_64 box support coming soon!").as_str());
    }
}