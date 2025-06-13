use std::collections::HashMap;
use std::option::Option;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::{fs, io};

#[derive(Debug, Deserialize, Serialize, Hash)]
struct SkinEntry {
    skinid: String,
    skin_name: String,
    name: String,
}

static SKIN_ENTRIES: LazyLock<HashMap<String, SkinEntry>> = LazyLock::new(|| {
    let skins: Vec<SkinEntry> =
        serde_json::from_str(include_str!("data/character_data.json")).expect("Invalid JSON");
    let skin_map: HashMap<String, SkinEntry> = skins
        .into_iter()
        .map(|entry| (entry.skinid.clone(), entry))
        .collect();

    skin_map
});

static SKIN_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[0-9]{4}\/[0-9]{7}").unwrap());

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

pub enum ModType {
    Default(String),
    Custom(String),
}
pub fn get_character_mod_skin(skin_id: &str) -> Option<ModType> {
    let skin = SKIN_ENTRIES.get(&skin_id.to_string());
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
    None
}

fn extract_character_from_path(path: &str) -> Option<String> {
    // Look for the "PC/" part of the path and extract the name that follows it
    if let Some(start) = path.rfind("/PC/") {
        // Extract the character name after "/PC/"
        let start_pos = start + "/PC/".len();
        if let Some(end) = path[start_pos..].find('/') {
            return Some(path[start_pos..start_pos + end].to_string());
        }
    }
    None
}

// Helper function to extract the character from the path
fn extract_character_from_path_audio(path: &str) -> Option<String> {
    // Look for the "ActionVoice" part of the path and extract the name that follows it
    if let Some(start) = path.rfind("/ActionVoice/") {
        // Extract the character name after "/ActionVoice/"
        let start_pos = start + "/ActionVoice/".len();
        if let Some(end) = path[start_pos..].find('/') {
            return Some(path[start_pos..start_pos + end].to_string());
        }
    }
    None
}

// Helper function to extract the language from the path
fn extract_language_from_path(path: &str) -> Option<String> {
    // Look for the "L10N" part of the path and extract the language that follows it
    if let Some(start) = path.rfind("L10N/") {
        // Extract the language code after "/L10N/"
        let start_pos = start + "L10N/".len();
        if let Some(end) = path[start_pos..].find('/') {
            return Some(path[start_pos..start_pos + end].to_string());
        }
    }
    None
}

pub fn get_current_pak_characteristics(mod_contents: Vec<String>) -> String {
    let mut fallback: Option<String> = None;

    for file in &mod_contents {
        let path = file
            .strip_prefix("SB/Content/SB/")
            .or_else(|| file.strip_prefix("/Game/"))
            .unwrap_or(file);

        let category = path.split('/').next().unwrap_or_default();

        match category {
            "L10N" => {
                // Extract character and language from the path
                if let Some(character) = extract_character_from_path_audio(path) {
                    if let Some(language) = extract_language_from_path(path) {
                        return format!("Audio ({} - {})", character, language);
                    }
                }
                // Fallback logic in case the extraction fails
                return "Audio (Unknown)".to_string();
            }
            "Art" => if let Some(character) = extract_character_from_path(path) {
                match get_character_mod_skin(character.as_str()) {
                    Some(ModType::Custom(skin)) => return skin,
                    Some(ModType::Default(name)) => fallback = Some(name),
                    None => {
                        debug!("No character data found. Trying next");
                    }
                }
            },
            "UI" => return "UI".to_string(),
            "Movies" => return "Movies".to_string(),
            _ if path.contains("WwiseAudio") => return "Audio".to_string(),
            _ => {}
        }
    }

    fallback.unwrap_or_else(|| "Unknown".to_string())
}

use log::{debug, info};
use regex_lite::Regex;
use serde::{Deserialize, Serialize};

pub fn find_marvel_rivals() -> Option<PathBuf> {
    let shit = get_steam_library_paths();
    if shit.is_empty() {
        return None;
    }

    for lib in shit {
        let path = lib.join("steamapps/common/StellarBlade/SB/Content/Paks");
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
