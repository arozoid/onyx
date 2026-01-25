use crate::helpers::{errln, time_get, BLUE, DIM, ESC, RED, GREEN, YELLOW};

use std::fs;
use minreq;
use std::process::Command;

//=== variables ===//
const VERSION: &str = "v0.1.0 build 26w04c";

// #[derive(Debug)]
// pub struct CpuInfo {
//     arch: &'static str,
//     vendor: &'static str,
//     note: Option<&'static str>,
// }

// pub const CPU_LIST: &[CpuInfo] = &[
//     // --- ARM64 / aarch64 ---
//     CpuInfo { arch: "aarch64", vendor: "aarch64", note: Some("Generic ARM64") },
//     CpuInfo { arch: "aarch64", vendor: "arm64", note: Some("Alternative ARM64 name") },
//     CpuInfo { arch: "aarch64", vendor: "armv8", note: Some("ARMv8 generic") },
//     CpuInfo { arch: "aarch64", vendor: "armv8l", note: Some("ARMv8 little-endian") },

//     // Cortex cores (SBCs, Raspberry Pi)
//     CpuInfo { arch: "aarch64", vendor: "cortex-a53", note: Some("RPi 3 / small SBC core") },
//     CpuInfo { arch: "aarch64", vendor: "cortex-a55", note: Some("RPi 4 / efficiency core") },
//     CpuInfo { arch: "aarch64", vendor: "cortex-a57", note: Some("High-end SBC") },
//     CpuInfo { arch: "aarch64", vendor: "cortex-a72", note: Some("RPi 4 / high-performance SBC") },
//     CpuInfo { arch: "aarch64", vendor: "cortex-a73", note: Some("High-end ARM board") },
//     CpuInfo { arch: "aarch64", vendor: "cortex-a75", note: Some("Server / mobile cores") },
//     CpuInfo { arch: "aarch64", vendor: "cortex-a76", note: Some("Recent ARM laptop/server") },
//     CpuInfo { arch: "aarch64", vendor: "cortex-a77", note: Some("Recent ARM laptop/server") },
//     CpuInfo { arch: "aarch64", vendor: "cortex-a78", note: Some("High-performance cores") },
//     CpuInfo { arch: "aarch64", vendor: "cortex-x1", note: Some("Premium ARM cores") },
//     CpuInfo { arch: "aarch64", vendor: "cortex-x2", note: Some("Premium ARM cores") },
//     CpuInfo { arch: "aarch64", vendor: "cortex-x3", note: Some("Premium ARM cores") },

//     // ARM server
//     CpuInfo { arch: "aarch64", vendor: "neoverse-n1", note: Some("Server / cloud CPUs") },
//     CpuInfo { arch: "aarch64", vendor: "neoverse-n2", note: Some("Server / cloud CPUs") },
//     CpuInfo { arch: "aarch64", vendor: "neoverse-v1", note: Some("Server / cloud CPUs") },

//     // Android / mobile SoCs
//     CpuInfo { arch: "aarch64", vendor: "kirin", note: Some("Huawei SoCs") },
//     CpuInfo { arch: "aarch64", vendor: "exynos", note: Some("Samsung SoCs") },
//     CpuInfo { arch: "aarch64", vendor: "snapdragon", note: Some("Qualcomm SoCs") },
//     CpuInfo { arch: "aarch64", vendor: "mediatek", note: Some("MediaTek SoCs") },
//     CpuInfo { arch: "aarch64", vendor: "rockchip", note: Some("Rockchip SBCs") },
//     CpuInfo { arch: "aarch64", vendor: "allwinner", note: Some("Allwinner SoCs") },

//     // Apple Silicon
//     CpuInfo { arch: "aarch64", vendor: "apple", note: Some("Apple M1/M2") },
//     CpuInfo { arch: "aarch64", vendor: "m1", note: Some("Apple M1") },
//     CpuInfo { arch: "aarch64", vendor: "m2", note: Some("Apple M2") },
//     CpuInfo { arch: "aarch64", vendor: "m3", note: Some("Apple M3") },
//     CpuInfo { arch: "aarch64", vendor: "m4", note: Some("Apple M4") },
//     CpuInfo { arch: "aarch64", vendor: "m5", note: Some("Apple M5") },

//     // --- x86_64 CPUs ---
//     CpuInfo { arch: "x86_64", vendor: "x86_64", note: Some("Generic 64-bit Intel/AMD") },
//     CpuInfo { arch: "x86_64", vendor: "intel", note: Some("Intel CPU") },
//     CpuInfo { arch: "x86_64", vendor: "amd", note: Some("AMD CPU") },
//     CpuInfo { arch: "x86_64", vendor: "core2", note: Some("Intel Core 2") },
//     CpuInfo { arch: "x86_64", vendor: "nehalem", note: Some("Intel Nehalem") },
//     CpuInfo { arch: "x86_64", vendor: "sandybridge", note: Some("Intel Sandy Bridge") },
//     CpuInfo { arch: "x86_64", vendor: "ivybridge", note: Some("Intel Ivy Bridge") },
//     CpuInfo { arch: "x86_64", vendor: "haswell", note: Some("Intel Haswell") },
//     CpuInfo { arch: "x86_64", vendor: "broadwell", note: Some("Intel Broadwell") },
//     CpuInfo { arch: "x86_64", vendor: "skylake", note: Some("Intel Skylake") },
//     CpuInfo { arch: "x86_64", vendor: "cascadelake", note: Some("Intel Cascade Lake") },
//     CpuInfo { arch: "x86_64", vendor: "zen", note: Some("AMD Zen") },
//     CpuInfo { arch: "x86_64", vendor: "zen2", note: Some("AMD Zen 2") },
//     CpuInfo { arch: "x86_64", vendor: "zen3", note: Some("AMD Zen 3") },
//     CpuInfo { arch: "x86_64", vendor: "epyc", note: Some("AMD EPYC server") },
// ];

//=== helpers ===//
fn parse_mem_line(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn get_kernel() -> (Output, bool) {
    let kernel = fs::read_to_string("/proc/version")
        .unwrap_or_else(|_| "unknown".to_string());
    if kernel == "unknown" {
      let kernel = Command::new("uname")
        .arg("-r")
        .output()
        .unwrap_or("unknown");
      (kernel, false)
    } else {
      (kernel, true)
    }
}

fn get_mem() -> (u64, u64) {
    let contents = fs::read_to_string("/proc/meminfo")
        .expect("couldn't read meminfo. are you on linux?");

    let mut total = 0;
    let mut available = 0;

    for line in contents.lines() {
        if line.starts_with("MemTotal:") {
            total = parse_mem_line(line);
        } else if line.starts_with("MemAvailable:") {
            available = parse_mem_line(line);
        }
    }

    let used = total - available;
    (used, total)
}

fn is_version_higher(current: &str, target: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
         .map(|s| s.parse().unwrap_or(0))
         .collect()
    };

    let current_parts = parse(current);
    let target_parts = parse(target);

    // rust vectors compare lexicographically: [6, 8, 0] > [4, 14, 0]
    current_parts > target_parts
}

fn check_latest_version() -> Option<String> {
    let url = "https://raw.githubusercontent.com/arozoid/onyx/refs/heads/main/version.txt";
    let resp = minreq::get(url).send().ok()?;
    if resp.status_code == 200 {
        Some(resp.as_str().ok()?.trim().to_string())
    } else {
        None
    }
}

fn check_file_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}

//=== cli ===//
pub fn cmd() {
    //=== system vars ===//
    // memory in MB
    let (used, total) = get_mem();
    let used = used / 1024;
    let total = total / 1024;

    // Architecture
    let arch_output = Command::new("uname")
        .arg("-m")
        .output()
        .unwrap_or_else(|_| {
            errln("doctor", "failed to get system architecture");
            std::process::exit(1);
        });
    let arch = String::from_utf8_lossy(&arch_output.stdout).trim().to_string();

    // Linux kernel
    let kernel = get_kernel();
    if kernel.1 {
      kernel.0 = kernel.0.replacen('(', "\x1b[2m(", 1);
    } else {
      kernel.0 = format!("Linux version {}", kernel.0);
    }

    let version_part = kernel.0.split_whitespace().nth(2).unwrap_or(""); 
    // cleanup "6.8.0-88-generic" to just "6.8.0"
    let version_num = version_part.split('-').next().unwrap_or("");

    let target = "4.14";

    //=== software vars ===//
    let latest_version = check_latest_version();
    let latest_version = latest_version
        .as_deref()
        .unwrap_or("")
        .split('(')
        .next()
        .unwrap_or("")
        .trim();

    let box64 = check_file_exists("/home/onyx/box64/");
    let proot = check_file_exists("/home/onyx/proot/");
    let glibc = check_file_exists("/home/onyx/glibc/");

    println!("{BLUE}[>== onyx doctor ==<]{ESC}");

    //=== system ===//
    println!("{BLUE}system:{ESC}");

    let kv;
    let mv;

    println!("    {GREEN}[arch]{ESC} {arch}");
    if is_version_higher(version_num, target) {
        print!("    {GREEN}[kernel]{ESC} {kernel.0}{ESC}");
        kv = true;
    } else {
        print!("    {RED}[kernel]{ESC} {kernel.0}{ESC}");
        kv = false;
    }

    if total >= 1024 {
        println!("    {GREEN}[memory]{ESC} {used} MB / {total} MB");
        mv = 2;
    } else if total >= 512 {
        println!("    {YELLOW}[memory]{ESC} {used} MB / {total} MB");
        mv = 1;
    } else {
        println!("    {RED}[memory]{ESC} {used} MB / {total} MB");
        mv = 0;
    }

    match (kv, mv) {
        (true, 2) => {
            println!("  {BLUE}✔ system is well supported{ESC}");
            println!("    onyx should run boxes comfortably.");
        }
        (true, 1) => {
            println!("  {BLUE}⚠ system is supported with limits{ESC}");
            println!("    expect slower performance on heavier boxes.");
        }
        (true, 0) => {
            println!("  {RED}✖ system is very constrained{ESC}");
            println!("    only minimal boxes are recommended.");
        }
        (false, 2) => {
            println!("  {BLUE}⚠ kernel is older than recommended{ESC}");
            println!("    basic boxes may still work.");
        }
        (false, 1) => {
            println!("  {RED}✖ limited support detected{ESC}");
            println!("    older kernel and low memory may cause issues.");
        }
        (false, 0) => {
            println!("  {RED}✖ system is not recommended{ESC}");
            println!("    onyx may fail or behave unpredictably.");
        }
        _ => {
            println!("  {RED}✖ unknown system state{ESC}");
        }
    }
    println!();

    //=== software ===//
    println!("{BLUE}software:{ESC}");
    if VERSION != latest_version && !latest_version.is_empty() {
        println!("    {GREEN}[onyx]{ESC} {VERSION} (latest)");
    } else if !latest_version.is_empty() {
        println!("    {YELLOW}[onyx]{ESC} {VERSION} (latest: {latest_version})");
    } else {
        println!("    {YELLOW}[onyx]{ESC} {VERSION} (latest: unknown)");
    }

    let root;
    let euid = unsafe { libc::geteuid() };
    if euid == 0 {
        root = true;
    } else {
        root = false;
    }
    
    if root {
        println!("    {GREEN}[root]{ESC} running as root, chroot activated");
    } else {
        println!("    {YELLOW}[root]{ESC} non-root user, using proot");
    }

    #[cfg(target_os = "android")]
    if proot {
        println!("    {GREEN}[proot]{ESC} installed");
    } else {
        println!("    {YELLOW}[proot]{ESC} not installed, onyx will not work");
    }

    #[cfg(not(target_os = "android"))]
    if proot {
        println!("    {GREEN}[proot]{ESC} installed");
    } else {
        println!("    {YELLOW}[proot]{ESC} not installed, root required");
    }

    if box64 && arch == "aarch64" {
        println!("    {GREEN}[box64]{ESC} installed");
    } else if !box64 && arch == "aarch64" {
        println!("    {RED}[box64]{ESC} not installed");
    } else {
        println!("    {DIM}[box64]{ESC} not required on x86_64");
    }

    if glibc && arch == "aarch64" {
        println!("    {GREEN}[glibc]{ESC} installed");
    } else if !glibc && arch == "aarch64" {
        println!("    {RED}[glibc]{ESC} not installed");
    } else {
        println!("    {DIM}[glibc]{ESC} not required on x86_64");
    }

    #[cfg(target_os = "android")]
    match (box64, proot, glibc) {
        (true, true, true) => {
            println!("  {BLUE}✔ software setup looks good{ESC}");
            println!("    you should be able to run boxes.");
        }
        (false, true, false) => {
            println!("  {BLUE}✔ software setup missing box64{ESC}");
            println!("    only arm boxes will work.");
        }
        (false, false, false) | (true, false, true) => {
            println!("  {RED}✖ missing critical components{ESC}");
            println!("    boxes may fail to run.");
        }
        _ => {
            println!("  {YELLOW}⚠ incomplete software setup{ESC}");
            println!("    some boxes may not work as expected.");
        }
    }
    println!();
}