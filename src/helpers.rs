use std::fs::File;
use std::io::{self, Write, Read};
use std::time::Duration;
use std::path::PathBuf;
use std::fs;
use nix::unistd::Uid;
use std::os::unix::fs::PermissionsExt;

use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use ureq::{Agent, Error};
use libc::{rlimit, RLIMIT_AS, setrlimit, cpu_set_t, sched_setaffinity, CPU_SET, CPU_ZERO};

//=== variables ===//
// normal
pub const RED: &str = "\x1b[31m";
pub const BLUE: &str = "\x1b[34m";
pub const DIM: &str = "\x1b[2m";
pub const ESC: &str = "\x1b[0m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";

// bold
pub const BLUEB: &str = "\x1b[1;34m";

/// global onyx dir in the user’s home
pub static ONYX_DIR: Lazy<PathBuf> = Lazy::new(|| {
    // determine base path
    #[cfg(target_os = "android")]
    let p = {
        let home = std::env::var("HOME").unwrap_or("/data/data/com.termux/files/home".into());
        PathBuf::from(home).join(".onyx")
    };

    #[cfg(not(target_os = "android"))]
    let p = PathBuf::from("/home/onyx");

    // if it already exists, just return it
    if p.exists() {
        return p;
    }

    // only create if it doesn't exist
    if let Err(e) = fs::create_dir_all(&p) {
        errln("onyx", &format!("failed to create {}: {}", p.display(), e));
        std::process::exit(1);
    }

    // create standard subfolders
    for folder in &["bin/core", "glibc", "box64", "sys", "tmp", "profiles"] {
        let sub = p.join(folder);
        if let Err(e) = fs::create_dir_all(&sub) {
            errln("onyx", &format!("failed to create {}: {}", sub.display(), e));
            std::process::exit(1);
        }
    }

    if let Err(e) = fs::File::create(p.join("current-profile")) {
        errln("onyx", &format!("failed to create current-profile: {}", e));
        std::process::exit(1);
    }

    // make base folder world-readable/writable/executable
    if let Ok(mut perms) = fs::metadata(&p).map(|m| m.permissions()) {
        perms.set_mode(0o777);
        let _ = fs::set_permissions(&p, perms);
    }

    p
});

//=== helper funcs ===//
pub fn set_nice(nice_level: i32) -> io::Result<()> {
    // unsafe because nice is a libc syscall
    let prev = unsafe { libc::nice(nice_level) };
    if prev == -1 {
        // could be error
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(0) {
            // on some systems -1 can be valid previous nice
            Ok(())
        } else {
            Err(err)
        }
    } else {
        Ok(())
    }
}

pub fn set_memory_limit(bytes: u64) -> Result<(), std::io::Error> {
    let limit = rlimit {
        rlim_cur: bytes,
        rlim_max: bytes,
    };
    let res = unsafe { setrlimit(RLIMIT_AS, &limit) }; // RLIMIT_AS limits virtual memory
    if res == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// pin the current process to `cores`
/// `cores` is a slice of CPU indexes, e.g. &[0,1]
pub fn pin_cpu(cores: &[usize]) -> io::Result<()> {
    if cores.is_empty() {
        return Ok(()); // no pinning requested
    }

    let mut set = unsafe { std::mem::zeroed::<cpu_set_t>() };
    unsafe { CPU_ZERO(&mut set) };

    for &core in cores {
        unsafe { CPU_SET(core, &mut set) };
    }

    let pid = 0; // 0 = current thread
    let size = std::mem::size_of::<cpu_set_t>();

    let res = unsafe { sched_setaffinity(pid, size, &set) };
    if res != 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn rooted() -> bool {
    Uid::effective().is_root()
}

pub fn time_get() -> String {
    let ts = time_format::now().unwrap();
    let datetime = time_format::strftime_local("%Y-%m-%d %H:%M:%S", ts).unwrap();
    datetime
}

pub fn errln(program: &str, msg: &str) {
    let t = time_get();
    eprintln!("{RED}[{program}] err:{ESC} {msg} {DIM}[{t}]{ESC}");
}

pub fn infoln(program: &str, msg: &str) {
    let t = time_get();
    println!("{BLUE}[{program}]{ESC} {msg} {DIM}[{t}]{ESC}");
}

pub fn fetch(url: &str) -> Option<String> {
    let mut resp = ureq::get(url).call().ok()?; // Response<Body>

    if resp.status() != 200 {
        return None;
    }

    let body = resp.body_mut().read_to_string().ok()?; // <-- THIS is the key

    Some(body.trim().to_string())
}

/// download a file safely to temp first, then rename
pub fn download(url: &str, filename: &str) -> io::Result<PathBuf> {
    let final_path = ONYX_DIR.join(filename);
    let temp_path = final_path.with_extension("tmp");
    let mut out = File::create(&temp_path)?;

    // build agent with timeout
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .timeout_connect(Some(Duration::from_secs(60)))
        .build()
        .into();

    // try up to 3 times
    let mut response = None;
    for attempt in 1..=3 {
        match agent.get(url).call() {
            Ok(resp) => {
                response = Some(resp);
                break;
            }
            Err(Error::Timeout(err)) => {
                errln(
                    "onyx",
                    format!(
                        "{YELLOW}spurious network:{ESC} attempt {attempt}/3 failed: {err}"
                    )
                    .as_str(),
                );
                std::thread::sleep(Duration::from_secs(2));
            }
            Err(err) => return Err(io::Error::new(io::ErrorKind::Other, err.to_string())),
        }
    }

    // check if all attempts failed
    let mut response = match response {
        Some(resp) => resp,
        None => { 
            errln("onyx", "download failed after 3 attempts. check your internet connection.");
            return Err(io::Error::new(io::ErrorKind::Other, "all download attempts failed"));
        },
    };

    // read content length if available
    let total_size = response.body().content_length().unwrap_or(0);

    // spinner & progress bar
    let pb = if total_size > 0 {
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::with_template("{spinner:.cyan.bold} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏✔") // juicy npm-like frames
                .progress_chars("## "),
        );
        Some(pb)
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{spinner:.cyan.bold} {msg}")
                .unwrap()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏✔")
        );
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_message("Downloading...");
        Some(pb)
    };

    // read and write
    let mut reader = response.body_mut().as_reader();
    let mut buffer = [0u8; 8192];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 { break; }
        out.write_all(&buffer[..n])?;
        if let Some(ref pb) = pb { pb.inc(n as u64); }
    }

    let dl_msg = format!("{BLUE}[onyx]{ESC} downloaded '{filename}' successfully");
    if let Some(ref pb) = pb {
        pb.finish_with_message(dl_msg);
        std::io::stdout().flush().unwrap();
    }

    std::fs::rename(&temp_path, &final_path)?;
    Ok(final_path)
}

pub fn file_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}
