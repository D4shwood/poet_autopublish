//! Local draft history: a snapshot of a poem every time its content changes.

use crate::config::history_dir;
use crate::poem::hash_str;
use anyhow::Result;
use chrono::{Local, NaiveDateTime};
use std::fs;
use std::path::PathBuf;

pub struct Snapshot {
    pub path: PathBuf,
    pub at: NaiveDateTime,
}

fn poem_dir(file_name: &str) -> PathBuf {
    history_dir().join(file_name)
}

/// Oldest first.
pub fn list(file_name: &str) -> Vec<Snapshot> {
    let mut out: Vec<Snapshot> = fs::read_dir(poem_dir(file_name))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let stem = path.file_stem()?.to_string_lossy().to_string();
            let at = NaiveDateTime::parse_from_str(&stem, "%Y%m%d-%H%M%S").ok()?;
            Some(Snapshot { path, at })
        })
        .collect();
    out.sort_by_key(|s| s.at);
    out
}

/// Saves a snapshot if the text differs from the newest one, then prunes to `keep`.
pub fn snapshot(file_name: &str, text: &str, keep: usize) -> Result<bool> {
    let existing = list(file_name);
    if let Some(last) = existing.last()
        && fs::read_to_string(&last.path).map(|t| hash_str(&t)).ok() == Some(hash_str(text))
    {
        return Ok(false);
    }
    let dir = poem_dir(file_name);
    fs::create_dir_all(&dir)?;
    // A second change within the same second overwrites that snapshot.
    let name = Local::now().format("%Y%m%d-%H%M%S").to_string();
    fs::write(dir.join(format!("{name}.txt")), text)?;

    let all = list(file_name);
    if keep > 0 && all.len() > keep {
        for s in &all[..all.len() - keep] {
            let _ = fs::remove_file(&s.path);
        }
    }
    Ok(true)
}

/// Keeps history attached to a draft when its file is renamed.
pub fn rename(old: &str, new: &str) {
    let (from, to) = (poem_dir(old), poem_dir(new));
    if from.exists() && !to.exists() {
        if let Some(parent) = to.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::rename(from, to);
    }
}
