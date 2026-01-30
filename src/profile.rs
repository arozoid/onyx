use crate::helpers::{errln, infoln, ONYX_DIR, pin_cpu, BLUE, ESC, BLUEB, GREEN, RED, YELLOW, BOLD};
use serde::{Serialize, Deserialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path},
};

fn prof_table() {
    let profiles = load_profiles(&ONYX_DIR.join("profiles")).unwrap();
    let mut ordered: Vec<_> = profiles.values().collect();
    ordered.sort_by(|a, b| {
        a.score()
            .cmp(&b.score())
            .then_with(|| a.name.cmp(&b.name))
    });

    // column widths
    let name_w  = 12;
    let score_w = 8;
    let mem_w   = 10;
    let cpu_w   = 8;
    let nice_w  = 4;

    // header
    println!(
        "{BLUEB}{:<name_w$} {:<score_w$} {:<mem_w$} {:<cpu_w$} {:<nice_w$}{ESC}",
        "name", "score", "memory", "cpu", "nice",
        name_w = name_w,
        score_w = score_w,
        mem_w = mem_w,
        cpu_w = cpu_w,
        nice_w = nice_w,
    );

    println!("{BOLD}{}{ESC}", "==".repeat(name_w + score_w + mem_w + cpu_w + nice_w + 5));

    // rows
    for p in ordered {
        let mem_color = match p.memory_severity() {
            0 => GREEN,
            1 => YELLOW,
            _ => RED,
        };

        let cpu_color = match &p.cpu {
            None => GREEN,
            Some(cpu) if cpu.cores >= 2 => YELLOW,
            _ => RED,
        };

        let nice_color = match p.nice {
            0..=5  => GREEN,
            6..=15 => YELLOW,
            _      => RED,
        };

        println!(
            "{BLUEB}{:<name_w$}{ESC} \
            {BLUE}{:<score_w$}{ESC} \
            {mem_color}{:<mem_w$}{ESC} \
            {cpu_color}{:<cpu_w$}{ESC} \
            {nice_color}{:<nice_w$}{ESC}    {}",
            p.name,
            p.score(),
            p.memory_display(),
            p.cpu_display(),
            p.nice,
            p.description.as_deref().unwrap_or(""),
            name_w = name_w,
            score_w = score_w,
            mem_w = mem_w,
            cpu_w = cpu_w,
            nice_w = nice_w,
        );
    }
}

pub fn load_profiles(dir: &Path) -> std::io::Result<HashMap<String, Profile>> {
    let mut profiles = HashMap::new();

    if !dir.exists() {
        return Ok(profiles);
    }

    for entry in fs::read_dir(dir)? {
        let path = entry?.path();

        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }

        let data = fs::read_to_string(&path)?;
        let profile: Profile = toml::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        profiles.insert(profile.name.clone(), profile);
    }

    Ok(profiles)
}

pub fn read_current_profile() -> Option<String> {
    let path = ONYX_DIR.join("current-profile");
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

pub fn apply_profile_cpu(profile: &Profile) {
    if let Some(cpu) = &profile.cpu {
        // pin to first N cores
        let cores: Vec<usize> = (0..cpu.cores).collect();
        if let Err(e) = pin_cpu(&cores) {
            eprintln!("warning: failed to pin CPU cores: {}", e);
        } else {
            println!("CPU pinned to cores: {:?}", cores);
        }
    } else {
        println!("No CPU pinning set for this profile");
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Profile {
    pub name: String,
    pub description: Option<String>,
    pub nice: i32,
    pub memory: MemoryConfig,
    pub cpu: Option<CpuConfig>,
}

impl Profile {
    fn memory_display(&self) -> String {
        match self.memory {
            MemoryConfig::Unlimited => "unlimited".into(),
            MemoryConfig::Percent { value: p } => format!("{p}% RAM"),
            MemoryConfig::Fixed { mb } => format!("{mb} MB"),
        }
    }

    fn cpu_display(&self) -> String {
        match &self.cpu {
            None => "all".into(),
            Some(n) => n.cores.to_string(),
        }
    }

    fn memory_severity(&self) -> u8 {
        match self.memory {
            MemoryConfig::Unlimited => 0,
            MemoryConfig::Percent { value: p } if p >= 60 => 0,
            MemoryConfig::Percent { value: p } if p >= 30 => 1,
            MemoryConfig::Fixed { mb } if mb >= 512 => 1,
            _ => 2,
        }
    }
    fn memory_weight(&self) -> u64 {
        match self.memory {
            MemoryConfig::Unlimited => 0, // best

            MemoryConfig::Percent { value } => {
                // scale so 100% = 0, 1% = max penalty
                let value = value.max(1); // avoid divide by zero
                100_000 / value as u64
            }

            MemoryConfig::Fixed { mb } => {
                // scale so 1024 MB = moderate, 12 MB = huge, smooth
                // weight = 100_000 - mb * 90
                100_000_u64.saturating_sub(mb as u64 * 90)
            }
        }
    }
    fn cpu_weight(&self) -> u64 {
        match &self.cpu {
            None => 0,                    // unlimited
            Some(cpu) => 1000 - cpu.cores as u64 * 100,
        }
    }
    fn nice_weight(&self) -> u64 {
        self.nice as u64
    }
    fn score(&self) -> u64 {
        self.memory_weight() * 10
            + self.cpu_weight() * 2
            + self.nice_weight()
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum MemoryConfig {
    #[serde(rename = "unlimited")]
    Unlimited,

    #[serde(rename = "percent")]
    Percent { value: u8 },

    #[serde(rename = "fixed")]
    Fixed { mb: u64 },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CpuConfig {
    pub cores: usize,
}

pub fn cmd(args: Vec<String>) {
    if args.len() < 3 {
        errln("profile", "no subcommand provided");
        errln("profile", "see 'onyx help profile' for usage");
        return;
    }

    match args[2].as_str() {
        "list" => {
            // list performance profiles
            infoln("profile", "listing performance profiles...");
            println!("{BLUEB}[>== profiles ==<]{ESC}");
            prof_table();
        }
        "use" => {
            // use a performance profile
            if args.len() < 4 {
                errln("profile", "no profile name provided");
                errln("profile", "see 'onyx help profile' for usage");
                return;
            }

            let profiles = load_profiles(&ONYX_DIR.join("profiles")).unwrap();
            if !profiles.contains_key(&args[3]) {
                errln("profile", &format!("profile '{}' does not exist", args[3]));
                return;
            }

            if let Err(e) = fs::write(ONYX_DIR.join("current-profile"), args[3].clone()) {
                errln("profile", &format!("failed to set profile: {}", e));
                return;
            }
            infoln("profile", format!("chose '{}' performance profile.", args[3]).as_str());
        }
        _ => {
            errln("profile", "unknown subcommand");
            errln("profile", "see 'onyx help profile' for usage");
        }
    }
}

fn profile_path(name: &str) -> std::path::PathBuf {
    Path::new(ONYX_DIR.join("profiles").to_str().expect("failed to get profiles directory")).join(format!("{}.toml", name))
}

fn load_profile(name: &str) -> Option<Profile> {
    let path = profile_path(name);
    if path.exists() {
        let s = fs::read_to_string(path).ok()?;
        toml::from_str(&s).ok()
    } else {
        None
    }
}

fn save_profile(profile: &Profile) {
    let path = profile_path(&profile.name);
    let toml_str = toml::to_string_pretty(profile).expect("Failed to serialize profile");
    fs::write(path, toml_str).expect("Failed to write profile");
}