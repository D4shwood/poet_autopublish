use crate::config::{self, Config, ensure_dir, log_path, state_dir};
use crate::history;
use crate::lang::{self, Lang};
use crate::latex::{self, Ctx};
use crate::poem::{self, Status, hash_str};
use crate::volume::{self, Week};
use anyhow::{Context, Result, bail};
use chrono::{Duration, Local, NaiveDate};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const STYLES: [&str; 6] = [
    "fontspec.sty",
    "geometry.sty",
    "xcolor.sty",
    "needspace.sty",
    "fancyhdr.sty",
    "hyperref.sty",
];

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    /// output file name -> hash of the .tex it was built from
    pub outputs: BTreeMap<String, String>,
    /// poem file name -> revision tracking
    pub poems: BTreeMap<String, PoemState>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PoemState {
    /// Hashes of the title and body only, so fixing a signature isn't a revision.
    /// (Older state files stored whole-file hashes under other names; those
    /// entries load empty here and simply start over.)
    #[serde(default)]
    pub first_body: String,
    #[serde(default)]
    pub last_body: String,
    pub revised: Option<NaiveDate>,
}

impl State {
    pub fn path(out_dir: &Path) -> PathBuf {
        out_dir.join(".state.json")
    }

    pub fn load(out_dir: &Path) -> State {
        fs::read_to_string(Self::path(out_dir))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    fn save(&self, out_dir: &Path) -> Result<()> {
        let tmp = out_dir.join(".state.json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        fs::rename(tmp, Self::path(out_dir))?;
        Ok(())
    }

    pub fn revisions(&self) -> HashMap<String, NaiveDate> {
        self.poems
            .iter()
            .filter_map(|(k, v)| v.revised.map(|d| (k.clone(), d)))
            .collect()
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub built: Vec<String>,
    pub removed: Vec<String>,
    pub unchanged: usize,
    pub unsigned: Vec<String>,
    pub drafts: Vec<String>,
    pub foreign: Vec<(String, String)>,
    pub errors: Vec<String>,
}

impl Report {
    pub fn summary(&self) -> Vec<String> {
        let mut out = vec![];
        for f in &self.built {
            out.push(format!("built {f}"));
        }
        for f in &self.removed {
            out.push(format!("removed {f}"));
        }
        if self.built.is_empty() && self.removed.is_empty() && self.errors.is_empty() {
            out.push(format!("up to date ({} PDFs)", self.unchanged));
        }
        if !self.unsigned.is_empty() {
            out.push(format!("unsigned, skipped: {}", self.unsigned.join(", ")));
        }
        for (p, a) in &self.foreign {
            out.push(format!("signed by {a}, skipped: {p}"));
        }
        for e in &self.errors {
            out.push(format!("ERROR {e}"));
        }
        out
    }
}

pub fn check_deps() -> Result<()> {
    let ok = Command::new("xelatex")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        bail!(
            "xelatex not found. Install it with:\n  sudo pacman -S texlive-xetex texlive-latexrecommended texlive-latexextra"
        );
    }
    let missing: Vec<&str> = STYLES
        .iter()
        .copied()
        .filter(|s| {
            !Command::new("kpsewhich")
                .arg(s)
                .output()
                .map(|o| o.status.success() && !o.stdout.is_empty())
                .unwrap_or(false)
        })
        .collect();
    if !missing.is_empty() {
        bail!(
            "missing LaTeX packages: {}. Install them with:\n  sudo pacman -S texlive-latexrecommended texlive-latexextra",
            missing.join(", ")
        );
    }
    Ok(())
}

pub fn log(line: &str) {
    let path = log_path();
    let _ = ensure_dir(&state_dir());
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[{}] {}", Local::now().format("%Y-%m-%d %H:%M:%S"), line);
    }
}

/// Exclusive lock shared by the watcher, the TUI and CLI builds.
pub fn lock(out_dir: &Path) -> Result<File> {
    ensure_dir(out_dir)?;
    let f = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(out_dir.join(".lock"))?;
    f.lock_exclusive()?;
    Ok(f)
}

/// Everything the TUI needs to show, without building.
pub struct Library {
    pub scan: poem::Scan,
    pub weeks: Vec<Week>,
    /// Books of the subfolders, outside the weekly series.
    pub collections: Vec<volume::Collection>,
    pub lang: Lang,
    pub book: String,
}

pub fn library(cfg: &Config, poems_dir: &Path) -> Result<Library> {
    let scan = poem::scan(poems_dir)?;
    let lang = lang::resolve(&cfg.lang, &scan.poems);
    let mut weeks = volume::group(
        &scan.poems,
        &scan.epigraphs,
        &cfg.author,
        cfg.night_cutoff(),
        &cfg.series,
    );
    let names = config::week_names();
    for w in &mut weeks {
        w.name = names.get(&w.key).cloned();
    }
    let collections = volume::collections(&scan.collections, &cfg.author);
    Ok(Library {
        scan,
        weeks,
        collections,
        lang,
        book: volume::book_file_name(&cfg.series),
    })
}

pub fn build(cfg: &Config, poems_dir: Option<&Path>, force: bool) -> Result<Report> {
    check_deps()?;
    let poems_dir = poems_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| cfg.poems_dir());
    let out_dir = cfg.out_dir();
    let _lock = lock(&out_dir)?;

    let lib = library(cfg, &poems_dir)?;
    let mut report = Report::default();
    let today = Local::now().date_naive();

    for p in lib.scan.all_poems() {
        let _ = history::snapshot(&p.file_name, &p.raw, cfg.history_keep);
        match p.status(&cfg.author) {
            Status::Unsigned => report.unsigned.push(p.file_name.clone()),
            Status::Draft => report.drafts.push(p.file_name.clone()),
            Status::Foreign(a) => report.foreign.push((p.file_name.clone(), a)),
            Status::Published => {}
        }
    }

    // Revision tracking for published poems.
    let mut state = State::load(&out_dir);
    let published: Vec<&poem::Poem> = lib
        .weeks
        .iter()
        .flat_map(|w| w.poems.iter())
        .chain(lib.collections.iter().flat_map(|c| c.poems.iter()))
        .collect();
    state
        .poems
        .retain(|k, _| published.iter().any(|p| &p.file_name == k));
    for p in &published {
        let sig_date = p.signature.as_ref().unwrap().date;
        let body = p.body_hash();
        let entry = state.poems.entry(p.file_name.clone()).or_default();
        if entry.first_body.is_empty() {
            *entry = PoemState {
                first_body: body.clone(),
                last_body: body.clone(),
                revised: None,
            };
        }
        if entry.last_body != body {
            entry.last_body = body;
            if entry.last_body == entry.first_body {
                entry.revised = None;
            } else if today > sig_date + Duration::days(1) {
                entry.revised = Some(today);
            }
        }
    }
    let revisions = state.revisions();

    let template = cfg.template();
    let ctx = Ctx {
        template: &template,
        series: &cfg.series,
        author: &cfg.author,
        lang: lib.lang,
        revisions: &revisions,
    };

    let mut wanted: Vec<(String, String, u8)> = lib
        .weeks
        .iter()
        .map(|w| (w.file_name.clone(), latex::week_doc(w, &ctx), 1))
        .collect();
    if !lib.weeks.is_empty() {
        wanted.push((lib.book.clone(), latex::book_doc(&lib.weeks, &ctx), 2));
    }
    for c in &lib.collections {
        wanted.push((c.file_name.clone(), latex::collection_doc(c, &ctx), 2));
    }

    let mut outputs = BTreeMap::new();
    for (name, tex, runs) in &wanted {
        let hash = hash_str(tex);
        let target = out_dir.join(name);
        let fresh = !force && target.exists() && state.outputs.get(name) == Some(&hash);
        if fresh {
            report.unchanged += 1;
            outputs.insert(name.clone(), hash);
            continue;
        }
        match compile(tex, &target, *runs) {
            Ok(()) => {
                report.built.push(name.clone());
                outputs.insert(name.clone(), hash);
            }
            Err(e) => {
                report.errors.push(format!("{name}: {e:#}"));
                // Keep the old PDF and its old hash, so the next build retries.
                if let Some(old) = state.outputs.get(name) {
                    outputs.insert(name.clone(), old.clone());
                }
            }
        }
    }

    for old in state.outputs.keys() {
        if !outputs.contains_key(old) {
            let _ = fs::remove_file(out_dir.join(old));
            report.removed.push(old.clone());
        }
    }
    state.outputs = outputs;
    state.save(&out_dir)?;

    if let Some(mirror) = cfg.mirror_dir()
        && let Err(e) = sync_mirror(&out_dir, &mirror, &state, &report)
    {
        report.errors.push(format!("mirror: {e:#}"));
    }

    for line in report.summary() {
        log(&line);
    }
    Ok(report)
}

fn sync_mirror(out_dir: &Path, mirror: &Path, state: &State, report: &Report) -> Result<()> {
    ensure_dir(mirror)?;
    for name in state.outputs.keys() {
        let dst = mirror.join(name);
        if report.built.contains(name) || !dst.exists() {
            fs::copy(out_dir.join(name), &dst)?;
        }
    }
    for name in &report.removed {
        let _ = fs::remove_file(mirror.join(name));
    }
    Ok(())
}

/// Runs xelatex in a temporary folder and moves the PDF into place atomically.
fn compile(tex: &str, target: &Path, runs: u8) -> Result<()> {
    let dir = tempfile::tempdir()?;
    fs::write(dir.path().join("doc.tex"), tex)?;
    for _ in 0..runs {
        let out = Command::new("xelatex")
            .args(["-interaction=nonstopmode", "-halt-on-error", "doc.tex"])
            .current_dir(dir.path())
            .output()
            .context("running xelatex")?;
        if !out.status.success() {
            let log = fs::read_to_string(dir.path().join("doc.log")).unwrap_or_default();
            let tail: Vec<&str> = log
                .lines()
                .filter(|l| l.starts_with('!') || l.starts_with("l."))
                .take(6)
                .collect();
            bail!(
                "xelatex failed: {}",
                if tail.is_empty() {
                    "see log".into()
                } else {
                    tail.join(" | ")
                }
            );
        }
    }
    let parent = target.parent().unwrap();
    let tmp = parent.join(format!(
        ".{}.tmp",
        target.file_name().unwrap().to_string_lossy()
    ));
    fs::copy(dir.path().join("doc.pdf"), &tmp)?;
    fs::rename(&tmp, target)?;
    Ok(())
}
