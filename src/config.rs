use anyhow::{Context, Result};
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const APP: &str = "poetautopublish";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Folder that holds the poem text files.
    pub poems_dir: String,
    /// Where the PDFs are written.
    pub out_dir: String,
    /// Optional second folder that receives a copy of every PDF ("" = off).
    pub mirror_dir: String,
    /// Only poems signed with this name are published.
    pub author: String,
    /// Name of the series / journal printed on the covers.
    pub series: String,
    /// "auto", "tr" or "en".
    pub lang: String,
    /// Poems signed before this time count as the previous day.
    pub night_cutoff: String,
    /// Draft-history snapshots kept per poem.
    pub history_keep: usize,
    /// Program that opens PDFs ("" = the desktop default via xdg-open).
    pub pdf_viewer: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            poems_dir: "~/poetry".into(),
            out_dir: "~/poetry_published".into(),
            mirror_dir: String::new(),
            author: "Your Name".into(),
            series: "Contra Noir".into(),
            lang: "auto".into(),
            night_cutoff: "05:00".into(),
            history_keep: 50,
            pdf_viewer: String::new(),
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        config_dir().join("config.toml")
    }

    /// Loads the config, writing a default one on first run.
    pub fn load() -> Result<Config> {
        let path = Self::path();
        if !path.exists() {
            fs::create_dir_all(config_dir())?;
            let text = toml::to_string_pretty(&Config::default())?;
            fs::write(&path, text)?;
        }
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let cfg: Config =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        Ok(cfg)
    }

    pub fn poems_dir(&self) -> PathBuf {
        expand(&self.poems_dir)
    }

    pub fn out_dir(&self) -> PathBuf {
        expand(&self.out_dir)
    }

    pub fn mirror_dir(&self) -> Option<PathBuf> {
        let s = self.mirror_dir.trim();
        (!s.is_empty()).then(|| expand(s))
    }

    pub fn night_cutoff(&self) -> NaiveTime {
        NaiveTime::parse_from_str(self.night_cutoff.trim(), "%H:%M")
            .unwrap_or_else(|_| NaiveTime::from_hms_opt(5, 0, 0).unwrap())
    }

    /// The user's own template.tex overrides the built-in one.
    pub fn template(&self) -> String {
        let user = config_dir().join("template.tex");
        fs::read_to_string(user)
            .unwrap_or_else(|_| include_str!("../templates/template.tex").to_string())
    }
}

/// Expands a leading `~` to the home directory.
pub fn expand(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        home().join(rest)
    } else if p == "~" {
        home()
    } else {
        PathBuf::from(p)
    }
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| home().join(".config"))
        .join(APP)
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| home().join(".local/share"))
        .join(APP)
}

pub fn state_dir() -> PathBuf {
    dirs::state_dir()
        .unwrap_or_else(|| home().join(".local/state"))
        .join(APP)
}

pub fn history_dir() -> PathBuf {
    data_dir().join("history")
}

pub fn log_path() -> PathBuf {
    state_dir().join("build.log")
}

pub fn ensure_dir(p: &Path) -> Result<()> {
    fs::create_dir_all(p).with_context(|| format!("creating {}", p.display()))
}

/// Custom week names, keyed by ISO week ("2026-W41"), in weeks.toml.
pub fn week_names() -> BTreeMap<String, String> {
    fs::read_to_string(config_dir().join("weeks.toml"))
        .ok()
        .and_then(|t| toml::from_str(&t).ok())
        .unwrap_or_default()
}

/// Sets a week's name; an empty name restores the default.
pub fn set_week_name(key: &str, name: &str) -> Result<()> {
    let mut names = week_names();
    let name = name.trim();
    if name.is_empty() {
        names.remove(key);
    } else {
        names.insert(key.to_string(), name.to_string());
    }
    fs::create_dir_all(config_dir())?;
    fs::write(
        config_dir().join("weeks.toml"),
        toml::to_string_pretty(&names)?,
    )?;
    Ok(())
}
