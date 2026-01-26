use std::fs::File;
use std::io::{self, Write, Read};
use std::time::Duration;
use std::path::PathBuf;
use std::fs;
use nix::unistd::Uid;

use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use ureq::{Agent, Error};

//=== variables ===//
pub const RED: &str = "\x1b[31m";
pub const BLUE: &str = "\x1b[34m";
pub const DIM: &str = "\x1b[2m";
pub const ESC: &str = "\x1b[0m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";

/// global onyx dir in the user’s home
pub static ONYX_DIR: Lazy<PathBuf> = Lazy::new(|| {
    #[cfg(target_os = "android")]
    {
        let p = PathBuf::from(std::env::var("HOME").unwrap()).join(".onyx");
        if !file_exists(&p.to_str().unwrap()) {
            match std::fs::create_dir_all(&p) {
                Ok(_) => {}
                Err(_) => {
                    errln("onyx", "failed to create onyx directory (insufficient permissions)");
                    std::process::exit(1);
                }
            }
        }
        return p;
    }
    let p = PathBuf::from("/home/onyx");
    if rooted() && !file_exists(&p.to_str().unwrap()) {
        std::fs::create_dir_all(&p).unwrap();
        p
    } else if file_exists(&p.to_str().unwrap()) {
        p
    } else {
        errln("onyx", "failed to create onyx directory (insufficient permissions)");
        std::process::exit(1);
    }
});

//=== helper funcs ===//
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