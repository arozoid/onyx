use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{UNIX_EPOCH};

use nix::sched::{self, CloneFlags};
use dir_size;

use crate::helpers::{errln, BLUE, ESC, infoln, rooted, ONYX_DIR, BLUEB};

//=== mount guard ===//
struct MountGuard {
    mounts: Vec<PathBuf>,
}

impl MountGuard {
    fn new(root: &Path) -> Result<Self, String> {
        let mut mounts = Vec::new();

        // make / private to avoid propagation back to host (best-effort)
        run("mount", &["--make-rprivate", "/"])?;

        let proc = root.join("proc");
        let sys  = root.join("sys");
        let dev  = root.join("dev");
        let pts  = root.join("dev/pts");

        // proc
        run("mount", &["-t", "proc", "proc", proc.to_str().unwrap()])?;
        mounts.push(proc);

        // dev
        run("mount", &["--bind", "/dev", dev.to_str().unwrap()])?;
        // make it slave so it won't propagate to other mounts in this ns
        run("mount", &["--make-slave", dev.to_str().unwrap()])?;
        mounts.push(dev.clone());

        // dev/pts
        run("mount", &["--bind", "/dev/pts", pts.to_str().unwrap()])?;
        mounts.push(pts);

        // sys (bind then remount ro)
        run("mount", &["--bind", "/sys", sys.to_str().unwrap()])?;
        run("mount", &["-o", "remount,ro", sys.to_str().unwrap()])?;
        mounts.push(sys);

        Ok(Self { mounts })
    }
}

impl Drop for MountGuard {
    fn drop(&mut self) {
        for m in self.mounts.iter().rev() {
            let _ = Command::new("umount").arg(m).status();
        }
    }
}

//=== box cmds ===//
fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| format!("failed to run {}: {}", cmd, e))?;

    if !status.success() {
        return Err(format!("command failed: {} {:?}", cmd, args));
    }
    Ok(())
}

fn find_shell(root: &Path) -> String {
    let candidates = [
        "usr/bin/zsh",
        "bin/bash",
        "bin/ash",
        "bin/sh",
        "usr/bin/bash",
    ];

    for s in candidates {
        if root.join(s).exists() {
            return format!("/{}", s);
        }
    }

    "/bin/sh".to_string()
}

/// try to create a mount namespace. returns Ok(()) if success.
/// on failure, returns Err with explanation.
fn try_unshare_mount_ns() -> Result<(), String> {
    match sched::unshare(CloneFlags::CLONE_NEWNS) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("unshare(CLONE_NEWNS) failed: {}", e)),
    }
}

//=== cli ===//
pub fn cmd(args: Vec<String>) {
    match args[2].as_str() {
        "open" => {
            open(args);
        }
        "exec" => {
            exec(args);
        }
        "list" => {
            list();
        }
        _ => {
            errln("box", &format!("unknown box command: {}", args[2]));
        }
    }
}

fn list() {
    let sys_dir = ONYX_DIR.join("sys");
    infoln("box", "fetching info");

    if !sys_dir.exists() {
        errln("box", "no systems found");
        return;
    }

    println!("{BLUEB}[>== box list ==<]{ESC}");

    match fs::read_dir(&sys_dir) {
        Ok(entries) => {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                let name = entry.file_name().into_string().unwrap_or_else(|_| "<invalid>".into());

                if path.is_dir() {
                    // compute dir size in human-readable format
                    let size = dir_size::get_size_in_abbr_human_bytes(&path).unwrap_or("unknown".into());

                    // get last modified timestamp of folder
                    let modified = fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .ok()
                        .map(|t| {
                            // convert SystemTime to seconds since UNIX_EPOCH
                            let dur = t.duration_since(UNIX_EPOCH).unwrap_or_default();
                            let secs = dur.as_secs();
                            // format manually into Y-m-d H:M:S
                            let tm = time_format::strftime_local("%Y-%m-%d %H:%M:%S", secs.try_into().unwrap_or(0 as i64)).unwrap_or_else(|_| "unknown".into());
                            tm
                        })
                        .unwrap_or_else(|| "unknown".into());

                    println!("{BLUEB}{}:{ESC}", name);
                    println!("    {BLUE}[size]{ESC} {}", size);
                    println!("    {BLUE}[modified]{ESC} {}", modified);
                }
            }
        }
        Err(e) => {
            errln("box", &format!("failed to read systems directory: {}", e));
        }
    }
}


fn exec(args: Vec<String>) {
    if args.len() < 4 {
        errln("box", "no system provided to exec");
        return;
    }

    let sys_path = ONYX_DIR.join("sys").join(&args[3]);

    if !sys_path.exists() {
        errln("box", "system not found");
        return;
    }

    let command = &args[4..];

    if command.len() <= 0 {
        errln("box", "no command provided to exec");
        return;
    }

    if !rooted() {
        infoln("box", "running as normal user via proot");

        let proot_bin = ONYX_DIR.join("bin/proot");
        if !proot_bin.exists() {
            errln("box", "proot binary not found!");
            return;
        }

        // turn args into string
        let strcommand = command.join(" ");

        infoln("box", format!("executing box command: {}", strcommand).as_str());

        let mut cmd = Command::new(proot_bin);
        cmd.arg("-r").arg(&sys_path)
        .arg("-0")
        .arg("-b").arg("/dev")
        .arg("-b").arg("/proc")
        .arg("-b").arg("/sys")
        .arg("-w").arg("/")
        .args(command)
        .status()
        .expect("failed to run proot");
        return;
    }

    infoln("box", "running as root user with chroot");

    // try to unshare mount namespace first
    match try_unshare_mount_ns() {
        Ok(_) => infoln("box", "entered new mount namespace (isolation enabled)"),
        Err(e) => {
            // fallback: make mounts private on the host (best-effort)
            errln("box", &format!("couldn't create mount namespace: {}. falling back to private mounts.", e));
            if let Err(e2) = run("mount", &["--make-rprivate", "/"]) {
                errln("box", &format!("failed to make / rprivate: {}", e2));
                errln("box", "refusing to proceed without isolation");
                return;
            }
        }
    }

    // RAII mount guard
    let _mounts = match MountGuard::new(&sys_path) {
        Ok(m) => m,
        Err(e) => {
            errln("box", &e);
            return;
        }
    };

    let strcommand = command.join(" ");

    infoln("box", &format!("executing box command: {}", strcommand));

    match Command::new("chroot").arg(&sys_path).args(command).status() {
        Ok(_) => {}, // command finished, ignore exit code
        Err(e) => errln("box", &format!("chroot failed: {}", e)),
    }

    // when function exits, MountGuard is dropped and unmounts occur inside
    infoln("box", "unmounting...");
}

fn open(args: Vec<String>) {
    if args.len() < 4 {
        errln("box", "no system provided to open");
        return;
    }

    let sys_path = ONYX_DIR.join("sys").join(&args[3]);

    if !sys_path.exists() {
        errln("box", "system not found");
        return;
    }

    if !rooted() {
        infoln("box", "running as normal user via proot");

        let proot_bin = ONYX_DIR.join("bin/proot");
        if !proot_bin.exists() {
            errln("box", "proot binary not found!");
            return;
        }

        let shell = find_shell(&sys_path);
        infoln("box", format!("entering box with {shell}").as_str());

        let mut cmd = Command::new(proot_bin);
        cmd.arg("-r").arg(&sys_path)
        .arg("-0")
        .arg("-b").arg("/dev")
        .arg("-b").arg("/proc")
        .arg("-b").arg("/sys")
        .arg("-w").arg("/")
        .arg(shell)
        .status()
        .expect("failed to run proot");
        infoln("box", "exited box");
        return;
    }

    infoln("box", "running as root user with chroot");

    // try to unshare mount namespace first
    match try_unshare_mount_ns() {
        Ok(_) => infoln("box", "entered new mount namespace (isolation enabled)"),
        Err(e) => {
            // fallback: make mounts private on the host (best-effort)
            errln("box", &format!("couldn't create mount namespace: {}. falling back to private mounts.", e));
            if let Err(e2) = run("mount", &["--make-rprivate", "/"]) {
                errln("box", &format!("failed to make / rprivate: {}", e2));
                errln("box", "refusing to proceed without isolation");
                return;
            }
        }
    }

    // RAII mount guard
    let _mounts = match MountGuard::new(&sys_path) {
        Ok(m) => m,
        Err(e) => {
            errln("box", &e);
            return;
        }
    };

    let shell = find_shell(&sys_path);

    infoln("box", &format!("entering box with {}", shell));

    match Command::new("chroot").arg(&sys_path).arg(shell).status() {
        Ok(_) => {}, // shell finished, ignore exit code
        Err(e) => errln("box", &format!("chroot failed: {}", e)),
    }

    // when function exits, MountGuard is dropped and unmounts occur inside
    infoln("box", "exited box");
    infoln("box", "unmounting...");
}
