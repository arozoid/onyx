use crate::helpers::{errln, infoln, ONYX_DIR};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

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


#[derive(Debug, Deserialize)]
pub struct Profile {
    pub name: String,
    pub description: Option<String>,
    pub nice: i32,
    pub memory: MemoryConfig,
    pub cpu: Option<CpuConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum MemoryConfig {
    #[serde(rename = "unlimited")]
    Unlimited,

    #[serde(rename = "percent")]
    Percent { value: u8 },

    #[serde(rename = "fixed")]
    Fixed { mb: u64 },
}

#[derive(Debug, Deserialize)]
pub struct CpuConfig {
    pub cores: usize,
}

pub fn cmd(args: Vec<String>) {
    if args.len() < 2 {
        errln("profile", "no subcommand provided");
        errln("profile", "see 'onyx help profile' for usage");
    }

    match args[2].as_str() {
        "list" => {
            // list performance profiles
            infoln("profile", "listing performance profiles...");
            let profiles = load_profiles(&ONYX_DIR.join("profiles")).unwrap();
            for profile in profiles.values() {
                println!("{}", profile.name);
            }
        }
        "use" => {
            // use a performance profile
            infoln("profile", format!("choosing '{}' performance profile...", args[3]).as_str()); 
        }
        _ => {
            errln("profile", "unknown subcommand");
            errln("profile", "see 'onyx help profile' for usage");
        }
    }
}