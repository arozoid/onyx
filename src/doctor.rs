use crate::cpu;

use crate::helpers::{errln, BLUE, DIM, ESC, RED, GREEN, YELLOW, fetch, file_exists, infoln, rooted, BLUEB};

use std::fs;
use std::process::Command;

//=== variables ===//
pub const VERSION: &str = "v0.1.0 RC 1";

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

fn get_kernel() -> (String, bool) {
    let kernel = fs::read_to_string("/proc/version").unwrap_or_else(|_| "unknown".to_string());
    if kernel.trim() == "unknown" {
        // fallback to uname -r
        let output = Command::new("uname")
            .arg("-r")
            .output()
            .unwrap_or_else(|_| -> std::process::Output {
              errln("doctor", "panic! failed to get kernel from any method! defaulting to 4.14");
              let c = Command::new("echo")
                .arg("4.14-??-generic (failed to fetch kernel)")
                .output()
                .unwrap();
              c
            });
        let kernel_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (kernel_str, false)
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

//=== cli ===//
pub fn cmd() -> (bool, i32, bool, bool, bool, bool, String, String) {
    infoln("doctor", "fetching info");

    //=== system vars ===//
    // memory in MB
    let (used, total) = get_mem();
    let used = used / 1024;
    let total = total / 1024;
    
    // CPU (onyx unit)
    let (mcu, scu) = cpu::cmd();

    // Architecture
    let arch_output = Command::new("uname")
        .arg("-m")
        .output()
        .unwrap_or_else(|_| -> std::process::Output {
            errln("doctor", "panic! failed to get system architecture! both '/proc/version' and 'uname' fetched an error! defaulting to aarch64");
            let c = Command::new("echo")
                .arg("aarch64")
                .output()
                .unwrap();
            c
        });
    let arch = String::from_utf8_lossy(&arch_output.stdout).trim().to_string();

    // Linux kernel
    let kernelname;
    let kernel = get_kernel();
    if kernel.1 {
      kernelname = kernel.0.replacen('(', "\x1b[2m(", 1);
    } else {
      kernelname = format!("Linux version {}\n", kernel.0);
    }

    let version_part = kernelname.split_whitespace().nth(2).unwrap_or(""); 
    // cleanup "6.8.0-88-generic" to just "6.8.0"
    let version_num = version_part.split('-').next().unwrap_or("");

    let target = "4.14";

    //=== software vars ===//
    let latest_version = fetch("https://raw.githubusercontent.com/arozoid/onyx/refs/heads/main/version.txt");
    let latest_version = latest_version
        .as_deref()
        .unwrap_or("");

    let box64 = file_exists("/home/onyx/box64/");
    let proot = file_exists("/home/onyx/bin/proot");
    let glibc = file_exists("/home/onyx/glibc/");

    println!("{BLUEB}[>== onyx doctor ==<]{ESC}");

    //=== system ===//
    println!("{BLUEB}system:{ESC}");

    let kv;
    let mv;

    println!("    {BLUE}[arch]{ESC} {arch}");
    if is_version_higher(version_num, "5.15") {
        print!("    {BLUE}[kernel]{ESC} {kernelname}{ESC}");
        kv = true;
    } else if is_version_higher(version_num, target) {
        print!("    {GREEN}[kernel]{ESC} {kernelname}{ESC}");
        kv = true;
    } else {
        print!("    {RED}[kernel]{ESC} {kernelname}{ESC}");
        kv = false;
    }
    
    let cpu_score = (scu * (3 as f64)) + (mcu * (1 as f64));
    
    let (cpu_color, cpu_level) = if cpu_score >= 50.0 {
        (BLUE, 2) // great stuff
    } else if cpu_score >= 25.0 {
        (GREEN, 2) // smooth, capable
    } else if cpu_score >= 10.0 {
        (YELLOW, 1) // decent 
    } else {
        (RED, 0) // sluggish
    };
    
    println!("    {cpu_color}[cpu]{ESC} mcu: {:.2} oU | scu: {:.2} oU (onyx units)", mcu, scu);

    if total >= 4096 {
        println!("    {BLUE}[memory]{ESC} {used} MB / {total} MB");
        mv = 2;
    } else if total >= 2048 {
        println!("    {GREEN}[memory]{ESC} {used} MB / {total} MB");
        mv = 2;
    } else if total >= 512 {
        println!("    {YELLOW}[memory]{ESC} {used} MB / {total} MB");
        mv = 1;
    } else {
        println!("    {RED}[memory]{ESC} {used} MB / {total} MB");
        mv = 0;
    }

    match (kv, mv, cpu_level) {
        // === ideal ===
        (true, 2, 2) => {
            println!("  {BLUEB}✔ system is well supported{ESC}");
            println!("    strong cpu and memory available.");
            println!("    onyx should run boxes comfortably.");
        }
    
        // === good but limited ===
        (true, 1, 2) | (true, 2, 1) => {
            println!("  {BLUEB}⚠ system is supported with limits{ESC}");
            println!("    performance may dip under heavy workloads.");
        }
    
        // === cpu bottleneck ===
        (true, _, 0) => {
            println!("  {BLUEB}⚠ cpu is weak{ESC}");
            println!("    single-thread or heavy boxes may struggle.");
        }
    
        // === memory bottleneck ===
        (true, 0, _) => {
            println!("  {BLUEB}✖ system is memory constrained{ESC}");
            println!("    only minimal boxes are recommended.");
        }
    
        // === old kernel but usable ===
        (false, 2, 2) => {
            println!("  {BLUEB}⚠ kernel is older than recommended{ESC}");
            println!("    cpu and memory are strong, but expect quirks.");
        }
    
        // === mixed bad ===
        (false, 1, _) | (false, _, 1) => {
            println!("  {BLUEB}✖ limited support detected{ESC}");
            println!("    older kernel or weak cpu may cause issues.");
        }
    
        // === worst case ===
        (false, 0, _) | (_, _, 0) => {
            println!("  {BLUEB}✖ system is not recommended{ESC}");
            println!("    onyx may fail or behave unpredictably.");
        }
        
        _ => {
            println!("  system report failed");
        }
    }
    println!();

    //=== software ===//
    println!("{BLUEB}software:{ESC}");
    if VERSION == latest_version {
        println!("    {GREEN}[onyx]{ESC} {VERSION} (latest)");
    } else if !latest_version.is_empty() {
        println!("    {YELLOW}[onyx]{ESC} {VERSION} (latest: {latest_version})");
    } else {
        println!("    {YELLOW}[onyx]{ESC} {VERSION} (latest: unknown)");
    }

    let root = rooted();
    
    if root {
        println!("    {GREEN}[root]{ESC} running as root, chroot activated");
    } else {
        println!("    {YELLOW}[root]{ESC} non-root user, using proot");
    }

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

    if arch == "aarch64" {
      match (box64, proot, glibc, root) {
          (true, true, true, true) | (true, false, true, true) | (true, true, true, false) => {
              println!("  {BLUEB}✔ software setup looks good{ESC}");
              println!("    you should be able to run boxes.");
          }
          (false, true, false, false) | (false, true, false, true) | (false, false, false, true) => {
              println!("  {BLUEB}⚠ software setup missing box64{ESC}");
              println!("    only arm boxes will work.");
          }
          (false, false, false, false) | (true, false, true, false) | (false, false, true, false) | (true, false, false, false) => {
              println!("  {BLUEB}✖ missing critical components{ESC}");
              println!("    proot is missing. boxes may fail to run.");
          }
          (true, true, false, true) | (true, true, false, false) | (true, false, false, true) => {
              println!("  {BLUEB}⚠ incomplete software setup{ESC}");
              println!("    some boxes may not work as expected. install glibc to run x86_64 boxes.");
          }
          (false, true, true, true) | (false, true, true, false) | (false, false, true, true) => {
              println!("  {BLUEB}⚠ incomplete software setup{ESC}");
              println!("    some boxes may not work as expected. install box64 to run x86_64 boxes.");
          }
      }
    } else {
      match (proot, root) {
          (true, false) | (true, true) | (false, true) => {
              println!("  {BLUEB}✔ software setup looks good{ESC}");
              println!("    you should be able to run boxes.");
          }
          (false, false) => {
              println!("  {BLUEB}⚠ root or proot access required{ESC}");
              println!("    please run onyx as root to use boxes, or install proot");
          }
      }
    }
    println!();

    (kv, mv, root, box64, proot, glibc, arch, latest_version.to_string())
}