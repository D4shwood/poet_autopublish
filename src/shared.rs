//! Settings that travel with the poems, in `<poems folder>/.poet/`:
//! who the author is (`profile.toml`), their pen names and the other authors
//! (`authors.toml`), and the names given to weeks (`weeks.toml`). Keeping them
//! next to the poems means a synced copy of the folder (on the phone, say)
//! sees the same names. Machine-only settings stay in `config.toml`.
//!
//! Each file has a text form (`*_from_toml` / `*_to_toml`) for apps that read
//! the folder themselves, and a path form for the desktop.

use crate::authors::Author;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const DIR: &str = ".poet";
pub const PROFILE: &str = "profile.toml";
pub const AUTHORS: &str = "authors.toml";
pub const WEEKS: &str = "weeks.toml";

pub fn dir(poems_dir: &Path) -> PathBuf {
    poems_dir.join(DIR)
}

/// Who writes in this folder and how its series works.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    /// Only poems signed with this name (or its pen names) are published.
    pub author: String,
    /// The name of the series printed on the covers.
    pub series: String,
    /// "auto", "tr" or "en".
    pub lang: String,
    /// "HH:MM"; poems signed before it count as the previous day.
    pub night_cutoff: String,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            author: String::new(),
            series: "Contra Noir".into(),
            lang: "auto".into(),
            night_cutoff: "05:00".into(),
        }
    }
}

pub fn profile_from_toml(text: &str) -> Option<Profile> {
    toml::from_str(text).ok()
}

pub fn profile_to_toml(p: &Profile) -> String {
    toml::to_string_pretty(p).unwrap_or_default()
}

pub fn load_profile(poems_dir: &Path) -> Option<Profile> {
    read(poems_dir, PROFILE).and_then(|t| profile_from_toml(&t))
}

pub fn save_profile(poems_dir: &Path, p: &Profile) -> Result<()> {
    write(poems_dir, PROFILE, &profile_to_toml(p))
}

#[derive(Default, Serialize, Deserialize)]
struct AuthorsFile {
    #[serde(default, rename = "author")]
    authors: Vec<Author>,
}

pub fn authors_from_toml(text: &str) -> Vec<Author> {
    toml::from_str::<AuthorsFile>(text)
        .map(|f| f.authors)
        .unwrap_or_default()
}

pub fn authors_to_toml(list: &[Author]) -> String {
    toml::to_string_pretty(&AuthorsFile {
        authors: list.to_vec(),
    })
    .unwrap_or_default()
}

pub fn load_authors(poems_dir: &Path) -> Vec<Author> {
    read(poems_dir, AUTHORS)
        .map(|t| authors_from_toml(&t))
        .unwrap_or_default()
}

pub fn save_authors(poems_dir: &Path, list: &[Author]) -> Result<()> {
    write(poems_dir, AUTHORS, &authors_to_toml(list))
}

/// Custom week names, keyed by ISO week ("2026-W41").
pub fn weeks_from_toml(text: &str) -> BTreeMap<String, String> {
    toml::from_str(text).unwrap_or_default()
}

pub fn weeks_to_toml(names: &BTreeMap<String, String>) -> String {
    toml::to_string_pretty(names).unwrap_or_default()
}

pub fn week_names(poems_dir: &Path) -> BTreeMap<String, String> {
    read(poems_dir, WEEKS)
        .map(|t| weeks_from_toml(&t))
        .unwrap_or_default()
}

/// Sets a week's name in the map; an empty name restores the default.
pub fn rename_week(names: &mut BTreeMap<String, String>, key: &str, name: &str) {
    let name = name.trim();
    if name.is_empty() {
        names.remove(key);
    } else {
        names.insert(key.to_string(), name.to_string());
    }
}

pub fn set_week_name(poems_dir: &Path, key: &str, name: &str) -> Result<()> {
    let mut names = week_names(poems_dir);
    rename_week(&mut names, key, name);
    write(poems_dir, WEEKS, &weeks_to_toml(&names))
}

/// Copies authors.toml and weeks.toml from the old per-machine place
/// (`~/.config/poetautopublish/`) the first time a folder has no `.poet/`.
/// The old files are left where they are.
pub fn migrate(poems_dir: &Path, old_config_dir: &Path) -> Result<()> {
    if !poems_dir.is_dir() {
        return Ok(());
    }
    for name in [AUTHORS, WEEKS] {
        let new = dir(poems_dir).join(name);
        let old = old_config_dir.join(name);
        if !new.exists() && old.exists() {
            fs::create_dir_all(dir(poems_dir))?;
            fs::copy(&old, &new)?;
        }
    }
    Ok(())
}

fn read(poems_dir: &Path, name: &str) -> Option<String> {
    fs::read_to_string(dir(poems_dir).join(name)).ok()
}

/// Writes through a temporary file, so a sync never sees half a file.
fn write(poems_dir: &Path, name: &str, text: &str) -> Result<()> {
    let d = dir(poems_dir);
    fs::create_dir_all(&d)?;
    let tmp = d.join(format!(".{name}.tmp"));
    fs::write(&tmp, text)?;
    fs::rename(tmp, d.join(name))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let p = Profile {
            author: "Deniz Yalın".into(),
            ..Profile::default()
        };
        assert_eq!(profile_from_toml(&profile_to_toml(&p)), Some(p));

        let list = vec![Author {
            name: "Deniz Yalın".into(),
            pen_names: vec!["K.M Ozan".into()],
        }];
        assert_eq!(authors_from_toml(&authors_to_toml(&list)), list);

        let mut weeks = BTreeMap::new();
        rename_week(&mut weeks, "2026-W41", " İlk ");
        assert_eq!(weeks_from_toml(&weeks_to_toml(&weeks))["2026-W41"], "İlk");
        rename_week(&mut weeks, "2026-W41", "");
        assert!(weeks.is_empty());
    }

    #[test]
    fn migrates_once_and_keeps_old_files() {
        let tmp = std::env::temp_dir().join(format!("poet-shared-{}", std::process::id()));
        let (poems, old) = (tmp.join("poems"), tmp.join("config"));
        fs::create_dir_all(&poems).unwrap();
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join(WEEKS), "\"2026-W41\" = \"İlk\"\n").unwrap();

        migrate(&poems, &old).unwrap();
        assert_eq!(week_names(&poems)["2026-W41"], "İlk");
        assert!(old.join(WEEKS).exists());

        // A name changed in the folder is not overwritten by a later run.
        set_week_name(&poems, "2026-W41", "Yeni").unwrap();
        migrate(&poems, &old).unwrap();
        assert_eq!(week_names(&poems)["2026-W41"], "Yeni");
        fs::remove_dir_all(tmp).unwrap();
    }
}
