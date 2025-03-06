use std::option::Option;
use eframe::egui::CursorIcon::Default;
use std::cell::LazyCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::{fs, io};

#[derive(Debug, Deserialize, Serialize, Hash)]
struct SkinEntry {
    skinid: String,
    #[serde(rename = "skin name")]
    skin_name: String,
    name: String,
}

static SKIN_ENTRIES: LazyLock<HashMap<u32, SkinEntry>> = LazyLock::new(|| {
    let skins: Vec<SkinEntry> =
        serde_json::from_str(include_str!("data/character_data.json")).expect("Invalid JSON");
    let skin_map: HashMap<u32, SkinEntry> = skins
        .into_iter()
        .map(|entry| (entry.skinid.parse().unwrap(), entry))
        .collect();

    skin_map
});
static SKIN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    let re = Regex::new(r"[0-9]{4}\/[0-9]{7}").unwrap();
    re
});

pub fn collect_files(paths: &mut Vec<PathBuf>, dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(paths, &path)?;
        } else {
            paths.push(entry.path());
        }
    }
    Ok(())
}

enum ModType {
    Default(String),
    Custom(String),
}
pub fn get_character_mod_skin(file: &str) -> Option<ModType> {
    let skin_id = SKIN_REGEX.clone().captures(&file);
    if let Some(skin_id) = skin_id {
        let skin_id = skin_id[0].to_string();
        let skin_id = &skin_id[5..];
        let skin = SKIN_ENTRIES.get(&(skin_id.parse().unwrap()));
        if let Some(skin) = skin {
            if skin.skin_name == "Default" {
                return Some(ModType::Default(format!(
                    "{} - {}",
                    &skin.name, &skin.skin_name
                )));
            }
            return Some(ModType::Custom(format!(
                "{} - {}",
                &skin.name, &skin.skin_name
            )));
        }
        return None;
    } else {
        return None;
    }
}
pub fn get_current_pak_characteristics(mod_contents: Vec<String>) -> String {
    let mut is_default: Option<String> = None;
    for file in &mod_contents {
        if let Some(stripped) = file.strip_prefix("Marvel/Content/Marvel/") {
            let category = stripped.split('/').into_iter().next().unwrap_or_default();
            if category == "Characters" {
                let mod_type = get_character_mod_skin(&stripped);
                if let Some(mod_type) = mod_type {
                    match mod_type {
                        ModType::Default(default) => {
                            info!("Default skin, we keep looping");
                            is_default = Some(default);
                        }
                        ModType::Custom(skin_name) => return skin_name,
                    }
                } else {
                    return "Character (Unknown)".to_string();
                }
            } else if category == "UI" {
                return "UI".to_string();
            } else if category == "Movies" {
                return "Movies".to_string();
            }
        }
        if file.contains("WwiseAudio") {
            return "Audio".to_string();
        }
    }
    if let Some(is_default) = is_default {
        return is_default;
    }
    "Unknown".to_string()
}

use log::info;
use serde::{Deserialize, Serialize};
use regex_lite::Regex;

pub fn find_marvel_rivals() -> Option<PathBuf> {
    let shit = get_steam_library_paths();
    if shit.is_empty() {
        return None;
    }

    for lib in shit {
        let path = lib.join("steamapps/common/MarvelRivals/MarvelGame/Marvel/Content/Paks");
        if path.exists() {
            return Some(path);
        }
    }
    println!("Marvel Rivals not found.");
    None
}

/// Reads `libraryfolders.vdf` to find additional Steam libraries.
fn get_steam_library_paths() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    let vdf_path = PathBuf::from("C:/Program Files (x86)/Steam/steamapps/libraryfolders.vdf");

    #[cfg(target_os = "linux")]
    let vdf_path = PathBuf::from("~/.steam/steam/steamapps/libraryfolders.vdf");

    if !vdf_path.exists() {
        return vec![];
    }

    let content = fs::read_to_string(vdf_path).ok().unwrap_or_default();
    let mut paths = Vec::new();

    for line in content.lines() {
        // if line.contains('"') {
        //     let path: String = line
        //         .split('"')
        //         .nth(3)  // Extracts the path
        //         .map(|s| s.replace("\\\\", "/"))?; // Fix Windows paths
        //     paths.push(PathBuf::from(path).join("steamapps/common"));
        // }
        if line.trim().starts_with("\"path\"") {
            let path = line
                .split("\"")
                .nth(3)
                .map(|s| PathBuf::from(s.replace("\\\\", "\\")));
            info!("Found steam library path: {:?}", path);
            paths.push(path.unwrap());
        }
    }

    paths
}
