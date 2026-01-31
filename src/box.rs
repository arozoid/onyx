use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{UNIX_EPOCH};
use std::io::{self, Write};
use nix::unistd::User;

use std::os::unix::fs::FileTypeExt;
use walkdir::WalkDir;
use nix::unistd::geteuid;
use nix::sched::{self, CloneFlags};
use dir_size;

use crate::profile::{read_current_profile, load_profiles, Profile, MemoryConfig::{self, Unlimited, Percent, Fixed}, apply_profile_cpu};
use crate::helpers::{errln, BLUE, ESC, infoln, rooted, ONYX_DIR, BLUEB, set_nice, set_memory_limit, RED, YELLOW, DIM};
use crate::check_file_authority;

//=== mount guard ===//
struct MountGuard {
    mounts: Vec<PathBuf>,
    merged: PathBuf,
    is_overlay: bool,
}

impl MountGuard {
    /// `root` = sys_path (ONYX_DIR/sys/<system_name>)
    /// `uid`  = user id
    /// `system_name` = e.g., "debian", "alpine"
    fn new(root: &Path, uid: Option<&str>, system_name: &str) -> Result<Self, String> {
        let mut is_overlay = false;
        
        let merged = if let Some(uid) = uid {
            is_overlay = true;
            
            // NEW PATH LOGIC: ONYX_DIR/delta/<uid>/<system_name>/...
            // this prevents different OSs from sharing the same 'upper' layer
            let base = ONYX_DIR.join("delta").join(uid).join(system_name);
            let upper = base.join("upper");
            let work  = base.join("work");
            let merged = base.join("merged");

            for d in [&upper, &work, &merged] {
                std::fs::create_dir_all(d)
                    .map_err(|e| format!("failed to create {}: {}", d.display(), e))?;
            }

            let opts = format!(
                "lowerdir={},upperdir={},workdir={}",
                root.display(),
                upper.display(),
                work.display()
            );
            
            run("mount", &["-t", "overlay", "overlay", "-o", &opts, merged.to_str().unwrap()])?;

            merged
        } else {
            root.to_path_buf()
        };

        let mut mounts = Vec::new();

        // -- make / private to avoid mount leakage to host --
        run("mount", &["--make-rprivate", "/"])?;

        // bind mounts logic (proc, dev, sys)
        let binds = vec![
            ("proc", "proc", vec!["-t", "proc"]),
            ("/dev", "dev", vec!["--bind"]),
            ("/dev/pts", "dev/pts", vec!["--bind"]),
            ("/sys", "sys", vec!["--bind"]),
        ];

        for (src, dest_rel, args) in binds {
            let dest = merged.join(dest_rel);
            std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
            
            // convert to string once so we don't keep fighting the borrow checker
            let dest_str = dest.to_str().ok_or("invalid path")?.to_string();
            
            let mut cmd_args = args.clone();
            cmd_args.push(src);
            cmd_args.push(&dest_str);
            
            run("mount", &cmd_args)?;
            
            // extra hardening for /sys and /dev using our string reference
            if dest_rel == "sys" {
                run("mount", &["-o", "remount,ro,bind", &dest_str])?;
            }
            if dest_rel == "dev" {
                run("mount", &["--make-slave", &dest_str])?;
            }

            // NOW we move it. once it's in the vector, 'dest' is gone.
            mounts.push(dest);
        }

        Ok(Self { mounts, merged, is_overlay })
    }
    fn root(&self) -> &Path {
        &self.merged
    }
}

impl Drop for MountGuard {
    fn drop(&mut self) {
        // 1. reverse unmount children (pts -> dev -> sys -> proc)
        for m in self.mounts.iter().rev() {
            let _ = Command::new("umount").arg("-l").arg(m).status();
        }

        // 2. unmount the overlay itself
        if self.is_overlay {
            let _ = Command::new("umount").arg("-l").arg(&self.merged).status();
            // 3. clean up the MERGED mountpoint directory
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
        "apply-delta" => {
            let perms = check_file_authority(&ONYX_DIR).unwrap();
            if perms.0 == true || perms.1 == true {
                let name = args.get(3).expect("error: missing <name>");
                let system = args.get(4).expect("error: missing <system>");
                apply_delta(name, system).expect("failed to execute merge");
            } else {
                errln("box", "this user cannot edit the rootfs.");
            }
        }
        _ => {
            errln("box", &format!("unknown box command: {}", args[2]));
        }
    }
}

fn apply_delta(username: &str, system_name: &str) -> io::Result<()> {
    // 1. get current real UID as a baseline
    let current_uid = nix::unistd::getuid().as_raw().to_string();

    // 2. resolve the target UID
    let uid = if username == "self" || username == "" {
        current_uid
    } else if let Ok(Some(user)) = User::from_name(username) {
        user.uid.as_raw().to_string()
    } else if let Ok(val) = std::env::var("SUDO_UID") {
        val // we are in sudo, use the caller's ID
    } else if username.chars().all(|c| c.is_numeric()) {
        username.to_string() // user just passed "1000"
    } else {
        // final straw: check if the username matches the current user's name
        // (sometimes from_name fails but we know who we are)
        let current_user_name = std::env::var("USER").unwrap_or_default();
        if username == current_user_name {
            current_uid
        } else {
            // we really can't find it
            return Err(io::Error::new(io::ErrorKind::NotFound, "user lookup failed completely"));
        }
    };
    
    // 2. build paths based on system
    // assuming your structure is: onyx/sys/<system_name>
    let brick_path = ONYX_DIR.join("sys").join(system_name);
    let delta_path = ONYX_DIR.join("delta").join(&uid).join(&system_name).join("upper");

    // 3. sanity checks
    if !brick_path.exists() {
        errln("box", format!("system brick '{}' not found at {}", system_name, brick_path.display()).as_str());
        return Ok(());
    }

    if !delta_path.exists() {
        errln("box", format!("no delta found for user {} at {}", username, delta_path.display()).as_str());
        return Ok(());
    }

    // 4. the "no turning back" confirmation
    println!("{YELLOW}⚠ PERMANENT MERGE:{ESC} user '{}' -> system '{}'", username, system_name);
    print!("{BLUE}[box]{ESC} this will overwrite files in {}. confirm? [y/N]: ", system_name);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() != "y" {
        infoln("box", "merge aborted by user.");
        return Ok(());
    }

    infoln("box", format!("committing delta to {}...", system_name).as_str());

    // 1. handle removals (whiteouts) FIRST
    // we do this before rsync so we don't copy whiteout devices into the brick
    for entry in WalkDir::new(&delta_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let metadata = entry.metadata()?;

        // fuse-overlayfs uses character devices with 0/0 as whiteouts
        if metadata.file_type().is_char_device() {
            let relative_path = path.strip_prefix(&delta_path).map_err(|_| {
                io::Error::new(io::ErrorKind::Other, "path prefix mismatch")
            })?;
            let target_in_brick = brick_path.join(relative_path);
            
            if target_in_brick.exists() {
                println!("{DIM}[box] dbg:{ESC} removing whiteout: {}", target_in_brick.display());
                let target_meta = fs::symlink_metadata(&target_in_brick)?;
                if target_meta.is_dir() {
                    fs::remove_dir_all(&target_in_brick)?;
                } else {
                    fs::remove_file(&target_in_brick)?;
                }
            }
        }
    }

    // 2. sync additions and modifications
    let status = Command::new("rsync")
        .arg("-a")
        .arg("-v")
        .arg("--ignore-times")
        .arg(format!("{}/", delta_path.display()))
        .arg(&brick_path)
        .status()?;

    if status.success() {
        infoln("box", "merge complete. cleaning up delta folder...");
        
        // nuke the whole system-specific delta dir (upper + work)
        // delta_path is .../delta/UID/SYSTEM/upper -> parent is .../delta/UID/SYSTEM/
        if let Some(system_delta_root) = delta_path.parent() {
            if let Err(e) = fs::remove_dir_all(system_delta_root) {
                errln("box", format!("failed to nuke delta dir: {}", e).as_str());
            }
        }
        
        infoln("box", "delta flushed. system is now updated.");
    } else {
        errln("box", "rsync failed! brick might be in a partial state.");
    }

    Ok(())
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

// helper to keep the main block clean
fn run_proot_session(root_path: &Path) {
    let proot_bin = ONYX_DIR.join("bin/proot");
    let shell = find_shell(root_path);
    
    Command::new(proot_bin)
        .arg("-r").arg(root_path)
        .arg("-0")
        .arg("-b").arg("/dev").arg("-b").arg("/proc").arg("-b").arg("/sys")
        .arg("--link2symlink")
        .arg("-w").arg("/")
        .arg(shell)
        .status()
        .expect("failed to run proot");
}

fn run_standalone_proot(sys_path: &Path) {
    // on android, sys_path should be a writable copy of the rootfs
    run_proot_session(sys_path);
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

    let strcommand = command.join(" ");

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
        let is_android = std::env::var("PREFIX").map(|s| s.contains("com.termux")).unwrap_or(false);
        let has_fuse = std::path::Path::new("/dev/fuse").metadata().is_ok();

        // define paths
        let uid_str = geteuid().as_raw().to_string();
        let delta_dir = ONYX_DIR.join("delta").join(&uid_str).join(&args[3]);
        let upper = delta_dir.join("upper");
        let work = delta_dir.join("work");
        let merged = delta_dir.join("merged");

        // we only attempt the fuse dance if we aren't on android OR if /dev/fuse is miraculously open
        let use_overlay = !is_android || has_fuse;

        if use_overlay {
            infoln("box", "launching namespaced session with proot...");

            for dir in [&upper, &work, &merged] {
                fs::create_dir_all(dir).unwrap();
            }

            let fuse_bin = ONYX_DIR.join("bin/fuse-overlayfs");
            let proot_bin = ONYX_DIR.join("bin/proot");
            let shell = "/bin/bash"; // or use your find_shell logic

            // we build one giant command string that unshare executes
            // 1. mount the fuse layer
            // 2. run proot pointing to the newly merged layer
            let chain_cmd = format!(
                "{} -f -o lowerdir={},upperdir={},workdir={},squash_to_root {} & sleep 1 && {} -r {} -0 -b /dev -b /proc -b /sys --link2symlink -w / {} -c {}",
                fuse_bin.display(),
                sys_path.display(),
                upper.display(),
                work.display(),
                merged.display(),
                proot_bin.display(),
                merged.display(),
                shell,
                strcommand
            );

            let status = Command::new("unshare")
                .args(&["-U", "-r", "-m", "bash", "-c", &chain_cmd])
                .status()
                .expect("failed to execute namespaced chain");

            if !status.success() {
                errln("box", "session failed, falling back...");
                let proot_bin = ONYX_DIR.join("bin/proot");
                let shell = find_shell(&sys_path);
                
                Command::new(proot_bin)
                    .arg("-r").arg(&sys_path)
                    .arg("-0")
                    .arg("-b").arg("/dev").arg("-b").arg("/proc").arg("-b").arg("/sys")
                    .arg("--link2symlink")
                    .arg("-w").arg("/")
                    .arg(shell)
                    .status()
                    .expect("failed to run proot");
            }
        } else {
            infoln("box", "android detected: using standalone mode (no delta)");
            let proot_bin = ONYX_DIR.join("bin/proot");
            let shell = find_shell(&sys_path);
            
            Command::new(proot_bin)
                .arg("-r").arg(&sys_path)
                .arg("-0")
                .arg("-b").arg("/dev").arg("-b").arg("/proc").arg("-b").arg("/sys")
                .arg("--link2symlink")
                .arg("-w").arg("/")
                .arg(shell)
                .status()
                .expect("failed to run proot");
        }
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
    let guard = match MountGuard::new(&sys_path, Some(&geteuid().to_string()), &args[3]) {
        Ok(m) => m,
        Err(e) => {
            errln("box", &e);
            return;
        }
    };

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
        let is_android = std::env::var("PREFIX").map(|s| s.contains("com.termux")).unwrap_or(false);
        let has_fuse = std::path::Path::new("/dev/fuse").metadata().is_ok();

        // define paths
        let uid_str = geteuid().as_raw().to_string();
        let delta_dir = ONYX_DIR.join("delta").join(&uid_str).join(&args[3]);
        let upper = delta_dir.join("upper");
        let work = delta_dir.join("work");
        let merged = delta_dir.join("merged");

        // we only attempt the fuse dance if we aren't on android OR if /dev/fuse is miraculously open
        let use_overlay = !is_android || has_fuse;

        if use_overlay {
            infoln("box", "launching namespaced session with proot...");

            for dir in [&upper, &work, &merged] {
                fs::create_dir_all(dir).unwrap();
            }

            let fuse_bin = ONYX_DIR.join("bin/fuse-overlayfs");
            let proot_bin = ONYX_DIR.join("bin/proot");
            let shell = "/bin/bash"; // or use your find_shell logic

            // we build one giant command string that unshare executes
            // 1. mount the fuse layer
            // 2. run proot pointing to the newly merged layer
            let chain_cmd = format!(
                "{} -f -o lowerdir={},upperdir={},workdir={},squash_to_root {} & sleep 1 && {} -r {} -0 -b /dev -b /proc -b /sys --link2symlink -w / {}",
                fuse_bin.display(),
                sys_path.display(),
                upper.display(),
                work.display(),
                merged.display(),
                proot_bin.display(),
                merged.display(),
                shell
            );

            let status = Command::new("unshare")
                .args(&["-U", "-r", "-m", "bash", "-c", &chain_cmd])
                .status()
                .expect("failed to execute namespaced chain");

            if !status.success() {
                errln("box", "session failed, falling back...");
                run_standalone_proot(&sys_path);
            }
        } else {
            infoln("box", "android detected: using standalone mode (no delta)");
            run_standalone_proot(&sys_path);
        }
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
    let guard = match MountGuard::new(&sys_path, Some(&geteuid().to_string()), &args[3]) {
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