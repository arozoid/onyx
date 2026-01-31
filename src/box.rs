use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{UNIX_EPOCH};
use std::process::Stdio;

use nix::unistd::geteuid;
use nix::sched::{self, CloneFlags};
use dir_size;

use crate::profile::{read_current_profile, load_profiles, Profile, MemoryConfig::{self, Unlimited, Percent, Fixed}, apply_profile_cpu};
use crate::helpers::{errln, BLUE, ESC, infoln, rooted, ONYX_DIR, BLUEB, set_nice, set_memory_limit, RED};

//=== mount guard ===//
struct MountGuard {
    mounts: Vec<PathBuf>,
    merged: PathBuf, // this is the effective root (overlay or original root)
}

impl MountGuard {
    /// `root` = sys_path (ONYX_DIR/sys/<name>)
    /// `uid`  = Some(user_id) to enable per-user overlay, None = no overlay
    fn new(root: &Path, uid: Option<&str>) -> Result<Self, String> {
        let merged = if let Some(uid) = uid {
            // setup overlayfs dirs
            let base = root.parent().unwrap().parent().unwrap().join("delta").join(uid);
            let upper = base.join("upper");
            let work  = base.join("work");
            let merged = base.join("merged");

            for d in [&upper, &work, &merged] {
                std::fs::create_dir_all(d)
                    .map_err(|e| format!("failed to create {}: {}", d.display(), e))?;
            }

            // mount overlayfs
            let opts = format!(
                "lowerdir={},upperdir={},workdir={}",
                root.display(),
                upper.display(),
                work.display()
            );
            run("mount", &["-t", "overlay", "overlay", "-o", &opts, merged.to_str().unwrap()])?;

            merged
        } else {
            // no overlay, use root directly
            root.to_path_buf()
        };

        let mut mounts = Vec::new();

        // make / private
        run("mount", &["--make-rprivate", "/"])?;

        // bind mounts inside merged
        let proc = merged.join("proc");
        let sys  = merged.join("sys");
        let dev  = merged.join("dev");
        let pts  = merged.join("dev/pts");

        // proc
        std::fs::create_dir_all(&proc).map_err(|e| e.to_string())?;
        run("mount", &["-t", "proc", "proc", proc.to_str().unwrap()])?;
        mounts.push(proc);

        // dev
        std::fs::create_dir_all(&dev).map_err(|e| e.to_string())?;
        run("mount", &["--bind", "/dev", dev.to_str().unwrap()])?;
        run("mount", &["--make-slave", dev.to_str().unwrap()])?;
        mounts.push(dev.clone());

        // dev/pts
        std::fs::create_dir_all(&pts).map_err(|e| e.to_string())?;
        run("mount", &["--bind", "/dev/pts", pts.to_str().unwrap()])?;
        mounts.push(pts);

        // sys (bind + remount ro)
        std::fs::create_dir_all(&sys).map_err(|e| e.to_string())?;
        run("mount", &["--bind", "/sys", sys.to_str().unwrap()])?;
        run("mount", &["-o", "remount,ro", sys.to_str().unwrap()])?;
        mounts.push(sys);

        Ok(Self { mounts, merged })
    }

    /// the root to chroot into (merged overlay or original root)
    fn root(&self) -> &Path {
        &self.merged
    }
}

impl Drop for MountGuard {
    fn drop(&mut self) {
        // first, unmount overlay
        if self.merged.exists() {
            let _ = Command::new("umount")
                .arg("-l")
                .arg(&self.merged)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }

        // unmount bind mounts in reverse order
        for m in self.mounts.iter().rev() {
            let _ = Command::new("umount")
                .arg("-l")
                .arg(m)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }

        // now remove merged mountpoint only
        if self.merged.exists() {
            let _ = std::fs::remove_dir(&self.merged);
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
    let mut binding;
    let prof;
    if profile == String::new() {
        binding = load_profiles(ONYX_DIR.join("profiles").as_path()).expect("failed to fetch profiles");
        binding.insert("__backup__".to_string(), backup.clone());
        prof = binding
            .get(read_current_profile().unwrap_or_else(|| "__backup__".to_string()).as_str())
            .unwrap_or_else(|| &backup);
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
        prof = read_current_profile().unwrap_or("__backup__".to_string());
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

        let mut cmd = Command::new(proot_bin);
        cmd
            .env_clear() // kill everything termux gave us
            .env("HOME", "/root")
            .env("TERM", std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string()))
            .env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
            .env("PROOT_TMP_DIR", &format!("{}/tmp", ONYX_DIR.to_str().unwrap()))
            .arg("-r").arg(&sys_path)
            .arg("-0")
            .arg("-b").arg("/dev")
            .arg("-b").arg("/proc")
            .arg("-b").arg("/sys")
            .arg("-w").arg("/")
            .arg("--link2symlink")
            .arg(strcommand)
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
    let guard = match MountGuard::new(&sys_path, Some(&geteuid().to_string())) {
        Ok(m) => m,
        Err(e) => {
            errln("box", &e);
            return;
        }
    };

    let strcommand = command.join(" ");

    let shell = find_shell(&sys_path);
    infoln("box", &format!("executing box command: {}", strcommand));
    
    match Command::new("chroot")
    .env_clear() // kill everything termux gave us
    .env("HOME", "/root")
    .env("TERM", std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string()))
    .env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
    .arg(guard.root())       // chroot root = merged overlay
    .arg(shell)              // run the shell
    .arg("-c")               // execute a single command
    .arg(strcommand)         // the command to run
    .env_remove("LD_PRELOAD")
    .status()
    {
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
        prof = read_current_profile().unwrap_or("__backup__".to_string());
        if prof.len() > 0 {
            limit_box(prof);
        } else {
            limit_box("__backup__".to_string());
        }
    }

    if !rooted() {
        infoln("box", "running as normal user via proot + fuse-overlayfs");

        let proot_bin = ONYX_DIR.join("bin/proot");
        let fuse_bin = ONYX_DIR.join("bin/fuse-overlayfs"); // fresh from the roster!

        if !proot_bin.exists() || !fuse_bin.exists() {
            errln("box", "required binaries (proot/fuse-overlayfs) missing!");
            return;
        }

        let uid_str = geteuid().as_raw().to_string();
        let delta_dir = ONYX_DIR.join("delta").join(&uid_str);
        let upper = delta_dir.join("upper");
        let work = delta_dir.join("work");
        let merged = delta_dir.join("merged");

        for dir in [&upper, &work, &merged] {
            fs::create_dir_all(dir).unwrap();
        }

        // 1. mount the fuse overlay
        // we use -o lowerdir,upperdir,workdir and the crucial 'userxattr'
        let mut fuse_cmd = Command::new(&fuse_bin);
        let fuse_status = fuse_cmd
            .arg("-o")
            .arg(format!(
                "lowerdir={},upperdir={},workdir={}",
                sys_path.to_str().unwrap(),
                upper.to_str().unwrap(),
                work.to_str().unwrap()
            ))
            .arg(&merged)
            .status()
            .expect("failed to start fuse-overlayfs");

        if !fuse_status.success() {
            errln("box", "fuse-overlayfs mount failed! check /dev/fuse permissions.");
            return;
        }

        let shell = find_shell(&merged); // shell is now inside the merged view

        // 2. run proot pointing to the MERGED folder as root
        let mut cmd = Command::new(proot_bin);
        cmd
            .env_clear()
            .env("HOME", "/root")
            .env("TERM", std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string()))
            .env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
            .arg("-r").arg(&merged) // <-- THIS is the magic
            .arg("-0")
            .arg("-b").arg("/dev")
            .arg("-b").arg("/proc")
            .arg("-b").arg("/sys")
            .arg("-w").arg("/")
            .arg("--link2symlink")
            .arg(shell)
            .status()
            .expect("failed to run proot");

        // 3. cleanup: unmount when done so we don't leave zombie mounts
        Command::new("fusermount")
            .arg("-u")
            .arg(&merged)
            .status()
            .ok();

        infoln("box", "exited box and unmounted delta");
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
    let guard = match MountGuard::new(&sys_path, Some(&geteuid().to_string())) {
        Ok(m) => m,
        Err(e) => {
            errln("box", &e);
            return;
        }
    };

    let shell = find_shell(guard.root());

    infoln("box", &format!("entering box with {}", shell));

    #[cfg(target_os = "android")]
    match Command::new("chroot")
        .env_clear() // kill everything termux gave us
        .env("HOME", "/root")
        .env("TERM", std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string()))
        .env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
        .env_remove("LD_PRELOAD")
        .arg(guard.root())
        .arg(shell)
        .status()
    {
        Ok(_) => {} // shell finished, ignore exit code
        Err(e) => errln("box", &format!("chroot failed: {}", e)),
    }

    #[cfg(not(target_os = "android"))]
    match Command::new("chroot")
        .env_clear() // kill everything termux gave us
        .env("HOME", "/root")
        .env("TERM", std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string()))
        .env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")
        .env_remove("LD_PRELOAD")
        .arg(guard.root())
        .arg(shell)
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