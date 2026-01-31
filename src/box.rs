use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{UNIX_EPOCH};

use nix::sched::{self, CloneFlags};
use dir_size;

use crate::profile::{read_current_profile, load_profiles, Profile, MemoryConfig::{self, Unlimited, Percent, Fixed}, apply_profile_cpu};
use crate::helpers::{errln, BLUE, ESC, infoln, rooted, ONYX_DIR, BLUEB, set_nice, set_memory_limit, RED};

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

fn limit_box(profile: String) {
    let backup = Profile {
        name: "backup".to_string(),
        description: Some("Temporary backup profile".to_string()),
        nice: 0,
        memory: MemoryConfig::Unlimited,
        cpu: None,
    };
    let binding;
    let prof;
    if profile == String::new() {
        binding = load_profiles(ONYX_DIR.join("profiles").as_path()).expect("failed to fetch profiles");
        prof = binding.get(read_current_profile().unwrap().as_str()).unwrap_or(&backup);
    } else if profile == "__backup__".to_string() {
        prof = &backup;
    } else {
        binding = load_profiles(ONYX_DIR.join("profiles").as_path()).expect("failed to fetch profiles");
        prof = binding.get(&profile).unwrap_or(&backup);
    }

    apply_profile_cpu(prof);

    let _ = set_nice(prof.nice);
    match prof.memory {
        Unlimited => {}
        Percent{value} => {
            let limit = crate::doctor::get_mem().1 * value as u64 / 100;
            let _ = set_memory_limit(limit * 1024 * 1024);
        }
        Fixed{mb} => {
            let _ = set_memory_limit(mb * 1024 * 1024);
        }
    }
}

//=== cli ===//
pub fn cmd(args: Vec<String>) {
    if args.len() < 3 {
        errln("box", "not enough arguments were provided");
        errln("box", "see 'onyx help box' for usage");
    }

    match args[2].as_str() {
        "delete" => {
            if args.len() < 4 {
                errln("box", "usage: onyx box delete <name>");
                std::process::exit(1);
            }
            let name = &args[3];
            if let Err(e) = delete_box(name) {
                errln("box", &format!("failed to nuke box '{}': {}", name, e));
                std::process::exit(1);
            }
        }

        "create" => {
            if args.len() < 5 {
                errln("box", "usage: onyx box create <name> <rootfs-folder> [--move=true]");
                std::process::exit(1);
            }
            let name = &args[3];
            let source = Path::new(&args[4]);
            
            let move_flag = args.iter().any(|arg| arg.to_lowercase() == "--move=true");

            if let Err(e) = create_box(name, source, move_flag) {
                errln("box", &format!("creation failed for '{}': {}", name, e));
                std::process::exit(1);
            }
        }
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

    let mut prof = String::new();
    for arg in args.clone() {
        if let Some(profile) = arg.strip_prefix("--profile=") {
            prof = profile.to_string();
        }
    }

    if prof.len() > 0 {
        limit_box(prof);
    } else {
        prof = read_current_profile().expect("failed to obtain 'current-profile'");
        if prof.len() > 0 {
            limit_box(prof);
        } else {
            limit_box("__backup__".to_string());
        }
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

        let mut cmd = Command::new("env");
        cmd.arg("-u").arg("LD_PRELOAD")
        .arg(proot_bin)
        .arg("-r").arg(&sys_path)
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

    let mut prof = String::new();
    for arg in args.clone() {
        if let Some(profile) = arg.strip_prefix("--profile=") {
            prof = profile.to_string();
        }
    }

    if prof.len() > 0 {
        limit_box(prof);
    } else {
        prof = read_current_profile().expect("failed to obtain 'current-profile'");
        if prof.len() > 0 {
            limit_box(prof);
        } else {
            limit_box("__backup__".to_string());
        }
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
        cmd
            .arg("-r").arg(&sys_path)
            .arg("-0")
            .arg("-b").arg("/dev")
            .arg("-b").arg("/proc")
            .arg("-b").arg("/sys")
            .arg("-w").arg("/")
            .arg(shell)
            // env handling
            .env("PATH", "/usr/bin")
            .env_remove("LD_PRELOAD")
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

    #[cfg(target_os = "android")]
    match Command::new("chroot")
        .arg(&sys_path)
        .arg(shell)
        // env isolation
        .env("PATH", "/usr/bin")
        .env_remove("LD_PRELOAD")
        .status()
    {
        Ok(_) => {} // shell finished, ignore exit code
        Err(e) => errln("box", &format!("chroot failed: {}", e)),
    }

    #[cfg(not(target_os = "android"))]
    match Command::new("chroot")
        .arg(&sys_path)
        .arg(shell)
        // env isolation
        .env_remove("LD_PRELOAD")
        .status()
    {
        Ok(_) => {} // shell finished, ignore exit code
        Err(e) => errln("box", &format!("chroot failed: {}", e)),
    }

    // when function exits, MountGuard is dropped and unmounts occur inside
    infoln("box", "exited box");
    infoln("box", "unmounting...");
}

/// creates a new box by either copying or moving a rootfs
fn create_box(name: &str, source_path: &Path, move_mode: bool) -> std::io::Result<()> {
    let onyx_dir = std::env::var("ONYX_DIR").unwrap_or_else(|_| "/home/onyx".to_string());
    let target_dir = PathBuf::from(onyx_dir).join("sys").join(name);

    if target_dir.exists() {
        eprintln!("{RED}[box] err:{ESC} box '{}' already exists.", name);
        std::process::exit(1);
    }

    fs::create_dir_all(&target_dir)?;

    if move_mode {
        // brute move: fast, but only works on same mount point
        fs::rename(source_path, &target_dir)?;
    } else {
        // brute copy: slow, works everywhere
        // note: in real life, use a crate like `fs_extra` for recursive copy
        copy_recursive(source_path, &target_dir)?;
    }

    println!("{BLUE}[box]{ESC} box '{}' created successfully at {:?}", name, target_dir);
    Ok(())
}

/// deletes the system entirely
fn delete_box(name: &str) -> std::io::Result<()> {
    let onyx_dir = std::env::var("ONYX_DIR").unwrap_or_else(|_| "/home/onyx".to_string());
    let target_dir = PathBuf::from(onyx_dir).join("sys").join(name);

    if !target_dir.exists() {
        eprintln!("{RED}[box] err:{ESC} box '{}' does not exist.", name);
        std::process::exit(1);
    }

    fs::remove_dir_all(target_dir)?;
    println!("{BLUE}[box]{ESC} box '{}' nuked.", name);
    Ok(())
}

fn copy_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}