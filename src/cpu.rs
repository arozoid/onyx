use std::fs;
use std::path::Path;

use crate::helpers::{errln};

#[derive(Debug)]
struct CpuCore {
    cur_khz: u64,
    max_khz: u64,
    weight: f64,
}

fn read_u64(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn detect_arch_factor() -> f64 {
    match std::env::consts::ARCH {
        "x86" | "x86_64" => 0.9,
        "aarch64" => 1.0,
        _ => 1.0,
    }
}

fn is_big_core(max_khz: u64) -> bool {
    // heuristic: big cores usually clock higher
    max_khz >= 1_900_000
}

fn read_cpu_cores() -> Vec<CpuCore> {
    let mut cores = Vec::new();

    let cpu_root = Path::new("/sys/devices/system/cpu");

    for entry in fs::read_dir(cpu_root).unwrap_or_else(|_| {
      errln("doctor", "panic! could not read cpu info");
      std::process::exit(1);
    }) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let name = entry.file_name();
        let name = name.to_string_lossy();

        if !name.starts_with("cpu") || name == "cpufreq" {
            continue;
        }

        let freq_path = entry.path().join("cpufreq");
        if !freq_path.exists() {
            continue;
        }

        let cur = read_u64(&freq_path.join("scaling_cur_freq"));
        let max = read_u64(&freq_path.join("scaling_max_freq"));

        let (cur, max) = match (cur, max) {
            (Some(c), Some(m)) if m > 0 => (c, m),
            _ => continue,
        };

        let weight = if is_big_core(max) { 1.8 } else { 1.0 };

        cores.push(CpuCore {
            cur_khz: cur,
            max_khz: max,
            weight,
        });
    }

    cores
}

fn compute_onyx_units(cores: &[CpuCore]) -> (f64, f64) {
    let arch_factor = detect_arch_factor();

    let mut mcu: f64 = 0.0;
    let mut scu: f64 = 0.0;

    for c in cores {
        let contrib = c.weight * (c.cur_khz as f64 / c.max_khz as f64);
        mcu += contrib;
        scu = scu.max(contrib);
    }

    (mcu * arch_factor, scu * arch_factor)
}

pub fn cmd() -> (f64, f64) {
    let cores = read_cpu_cores();

    if cores.is_empty() {
        errln("doctor", "panic! no cpu cores detected! defaulting to apple pi");
        return (3.14159, 3.14159);
    }

    let (mcu, scu) = compute_onyx_units(&cores);

    (mcu, scu)
}