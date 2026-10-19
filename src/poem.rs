use anyhow::{Context, Result};
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static SIGNATURE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\s*[-–—]?\s*(?P<author>[^\d~]+?)[.,]?\s*(?P<approx>~)?\s*(?P<d>\d{1,2})[./](?P<m>\d{1,2})[./](?P<y>\d{4})(?:\s*[-–—]?\s*(?P<H>\d{1,2})[.:](?P<M>\d{2}))?\s*$",
    )
    .unwrap()
});

static BARE_DATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?P<d>\d{1,2})[./](?P<m>\d{1,2})[./](?P<y>\d{4})\s*$").unwrap()
});

pub const DRAFT_TAGS: [&str; 2] = ["#taslak", "#draft"];

#[derive(Debug, Clone, PartialEq)]
pub struct Signature {
    pub author: String,
    pub date: NaiveDate,
    pub time: Option<NaiveTime>,
    /// `~08.10.2026`: the exact day is unknown; only its week is.
    pub approx: bool,
}

impl Signature {
    pub fn parse(line: &str) -> Option<Signature> {
        let c = SIGNATURE.captures(line)?;
        let author = c["author"]
            .trim()
            .trim_end_matches(['.', ','])
            .trim()
            .to_string();
        if author.is_empty() {
            return None;
        }
        let date = NaiveDate::from_ymd_opt(
            c["y"].parse().ok()?,
            c["m"].parse().ok()?,
            c["d"].parse().ok()?,
        )?;
        let time = match (c.name("H"), c.name("M")) {
            (Some(h), Some(m)) => Some(NaiveTime::from_hms_opt(
                h.as_str().parse().ok()?,
                m.as_str().parse().ok()?,
                0,
            )?),
            _ => None,
        };
        let approx = c.name("approx").is_some();
        Some(Signature {
            author,
            date,
            time: if approx { None } else { time },
            approx,
        })
    }

    /// The canonical form written by "Sign & publish".
    pub fn format(author: &str, at: NaiveDateTime) -> String {
        format!("- {}. {}", author, at.format("%d.%m.%Y - %H.%M"))
    }

    /// Date used for week grouping: early-morning poems belong to the night before.
    pub fn effective_date(&self, cutoff: NaiveTime) -> NaiveDate {
        match self.time {
            Some(t) if t < cutoff => self.date - Duration::days(1),
            _ => self.date,
        }
    }

    /// Poems with an approximate date sort after the dated ones of their week.
    pub fn sort_key(&self) -> (bool, NaiveDateTime) {
        (
            self.approx,
            self.date.and_time(self.time.unwrap_or(NaiveTime::MIN)),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Published,
    Draft,
    Unsigned,
    /// Signed by someone other than the configured author.
    Foreign(String),
}

#[derive(Debug, Clone)]
pub struct Poem {
    pub path: PathBuf,
    pub file_name: String,
    pub title: String,
    /// Lines between the title and the signature, with the draft tag removed
    /// and surrounding blank lines trimmed.
    pub body: Vec<String>,
    pub signature: Option<Signature>,
    pub draft: bool,
    pub raw: String,
}

impl Poem {
    pub fn parse(path: &Path, raw: &str) -> Poem {
        let raw = raw.replace("\r\n", "\n");
        let file_name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut lines: Vec<String> = raw.lines().map(|l| l.trim_end().to_string()).collect();

        let title = match lines.first() {
            Some(t) if !t.trim().is_empty() => t.trim().to_string(),
            _ => file_name.trim_end_matches(".txt").to_string(),
        };
        if !lines.is_empty() {
            lines.remove(0);
        }

        // Draft tag: the first non-empty line after the title.
        let mut draft = false;
        if let Some(i) = lines.iter().position(|l| !l.trim().is_empty())
            && DRAFT_TAGS.contains(&lines[i].trim().to_lowercase().as_str())
        {
            draft = true;
            lines.remove(i);
        }

        // Signature: the last non-empty line.
        let mut signature = None;
        if let Some(i) = lines.iter().rposition(|l| !l.trim().is_empty())
            && let Some(sig) = Signature::parse(&lines[i])
        {
            signature = Some(sig);
            lines.truncate(i);
        }

        while lines.first().is_some_and(|l| l.trim().is_empty()) {
            lines.remove(0);
        }
        while lines.last().is_some_and(|l| l.trim().is_empty()) {
            lines.pop();
        }

        Poem {
            path: path.to_path_buf(),
            file_name,
            title,
            body: lines,
            signature,
            draft,
            raw,
        }
    }

    pub fn status(&self, author: &str) -> Status {
        if self.draft {
            return Status::Draft;
        }
        match &self.signature {
            None => Status::Unsigned,
            Some(sig) if !same_author(&sig.author, author) => Status::Foreign(sig.author.clone()),
            Some(_) => Status::Published,
        }
    }

    /// Hash of the title and body, ignoring the signature and draft tag.
    pub fn body_hash(&self) -> String {
        hash_str(&format!("{}\n{}", self.title, self.body.join("\n")))
    }

    pub fn word_count(&self) -> usize {
        self.body.iter().map(|l| l.split_whitespace().count()).sum()
    }

    pub fn line_count(&self) -> usize {
        self.body.iter().filter(|l| !l.trim().is_empty()).count()
    }
}

fn same_author(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

pub fn hash_str(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Poems are files with no extension or a `.txt` extension.
/// Hidden, backup, temporary and `_`-prefixed files are ignored.
pub fn is_poem_file(path: &Path) -> bool {
    let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
        return false;
    };
    if name.starts_with('.') || name.starts_with('_') || name.ends_with('~') {
        return false;
    }
    // Titles may contain dots ("Hamd olsun. Son"), so only a short
    // alphanumeric suffix counts as a real extension.
    match name.rsplit_once('.') {
        Some((stem, ext))
            if !stem.is_empty()
                && !ext.is_empty()
                && ext.len() <= 5
                && ext.chars().all(|c| c.is_ascii_alphanumeric()) =>
        {
            ext.eq_ignore_ascii_case("txt")
        }
        _ => true,
    }
}

fn is_epigraph_file(path: &Path) -> bool {
    path.file_name()
        .map(|n| n.to_string_lossy().to_lowercase().starts_with("_epigraf"))
        .unwrap_or(false)
}

#[derive(Debug, Clone)]
pub struct Epigraph {
    pub date: NaiveDate,
    pub lines: Vec<String>,
}

impl Epigraph {
    /// The quote, followed by a `dd.mm.yyyy` line that decides its week.
    pub fn parse(raw: &str) -> Option<Epigraph> {
        let lines: Vec<String> = raw.lines().map(|l| l.trim_end().to_string()).collect();
        let i = lines.iter().rposition(|l| !l.trim().is_empty())?;
        let c = BARE_DATE.captures(&lines[i])?;
        let date = NaiveDate::from_ymd_opt(
            c["y"].parse().ok()?,
            c["m"].parse().ok()?,
            c["d"].parse().ok()?,
        )?;
        let mut quote: Vec<String> = lines[..i].to_vec();
        while quote.first().is_some_and(|l| l.trim().is_empty()) {
            quote.remove(0);
        }
        while quote.last().is_some_and(|l| l.trim().is_empty()) {
            quote.pop();
        }
        Some(Epigraph { date, lines: quote })
    }
}

#[derive(Debug, Default)]
pub struct Scan {
    pub poems: Vec<Poem>,
    pub epigraphs: Vec<Epigraph>,
}

pub fn scan(dir: &Path) -> Result<Scan> {
    let mut out = Scan::default();
    let entries = fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if is_epigraph_file(&path) {
            if let Ok(text) = fs::read_to_string(&path)
                && let Some(e) = Epigraph::parse(&text)
            {
                out.epigraphs.push(e);
            }
            continue;
        }
        if !is_poem_file(&path) {
            continue;
        }
        // Non-UTF-8 files are not poems.
        if let Ok(text) = fs::read_to_string(&path) {
            out.poems.push(Poem::parse(&path, &text));
        }
    }
    out.poems.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn signature_variants() {
        let s = Signature::parse("- Deniz Yalın. 13.10.2026 - 22.58").unwrap();
        assert_eq!(s.author, "Deniz Yalın");
        assert_eq!(s.date, NaiveDate::from_ymd_opt(2026, 10, 13).unwrap());
        assert_eq!(s.time, Some(t(22, 58)));

        let s = Signature::parse("Deniz Yalın 13/10/2026 22:58").unwrap();
        assert_eq!(s.author, "Deniz Yalın");
        assert_eq!(s.time, Some(t(22, 58)));

        let s = Signature::parse("- Deniz Yalın. 13.10.2026").unwrap();
        assert_eq!(s.time, None);

        let s = Signature::parse("-Deniz Yalın ~08.10.2026").unwrap();
        assert_eq!(s.author, "Deniz Yalın");
        assert!(s.approx);
        assert_eq!(s.date, NaiveDate::from_ymd_opt(2026, 10, 8).unwrap());
        assert!(
            !Signature::parse("- Deniz Yalın. 09.10.2026")
                .unwrap()
                .approx
        );
        assert!(Signature::parse("-Deniz Yalın xx.10.2026").is_none());

        assert!(Signature::parse("B00").is_none());
        assert!(Signature::parse("Hamd olsun. Hamd ettirene.").is_none());
        assert!(Signature::parse("- Deniz.31.02.2026").is_none());
    }

    #[test]
    fn night_cutoff() {
        // Monday 12.10.2026 00:17 belongs to Sunday the 11th.
        let s = Signature::parse("- S. 12.10.2026 - 00.17").unwrap();
        assert_eq!(
            s.effective_date(t(5, 0)),
            NaiveDate::from_ymd_opt(2026, 10, 11).unwrap()
        );
        let s = Signature::parse("- S. 12.10.2026").unwrap();
        assert_eq!(
            s.effective_date(t(5, 0)),
            NaiveDate::from_ymd_opt(2026, 10, 12).unwrap()
        );
    }

    #[test]
    fn parse_poem() {
        let raw = "Liman\n\nKıyıda beklerken,\n    Martılar\n\nSon.\n\n- Deniz Yalın. 13.10.2026 - 22.58\n";
        let p = Poem::parse(Path::new("/x/Liman"), raw);
        assert_eq!(p.title, "Liman");
        assert_eq!(
            p.body,
            vec!["Kıyıda beklerken,", "    Martılar", "", "Son."]
        );
        assert!(p.signature.is_some());
        assert_eq!(p.status("deniz yalın"), Status::Published);
        assert!(matches!(p.status("Someone"), Status::Foreign(_)));
    }

    #[test]
    fn signature_edits_keep_body_hash() {
        let a = Poem::parse(Path::new("B10"), "B10\n\nB01\n-Deniz Yalın xx.10.2026\n");
        let b = Poem::parse(Path::new("B10"), "B10\n\nB01\n-Deniz Yalın ~8.10.2026 \n");
        assert_ne!(a.raw, b.raw);
        // a's "xx" line isn't a signature, so it is still part of a's body.
        let c = Poem::parse(Path::new("B10"), "B10\n\nB01\n-Deniz Yalın 09.10.2026\n");
        assert_eq!(b.body_hash(), c.body_hash());
    }

    #[test]
    fn draft_tag() {
        let raw = "Başlık\n#taslak\n\nbir mısra\n- Deniz Yalın. 13.10.2026 - 22.58";
        let p = Poem::parse(Path::new("Başlık"), raw);
        assert!(p.draft);
        assert_eq!(p.body, vec!["bir mısra"]);
        assert_eq!(p.status("Deniz Yalın"), Status::Draft);
    }

    #[test]
    fn unsigned() {
        let p = Poem::parse(Path::new("B10"), "B10\n\nB01 satır\nB00\n");
        assert_eq!(p.status("Deniz Yalın"), Status::Unsigned);
        assert_eq!(p.body, vec!["B01 satır", "B00"]);
    }

    #[test]
    fn poem_files() {
        assert!(is_poem_file(Path::new("/p/Liman")));
        assert!(is_poem_file(Path::new("/p/Kum, Fener")));
        assert!(is_poem_file(Path::new("/p/a.txt")));
        assert!(!is_poem_file(Path::new("/p/a.md")));
        assert!(!is_poem_file(Path::new("/p/.hidden")));
        assert!(!is_poem_file(Path::new("/p/a~")));
        assert!(!is_poem_file(Path::new("/p/_epigraf")));
        assert!(!is_poem_file(Path::new("/p/a.swp")));
        assert!(is_poem_file(Path::new("/p/Hamd olsun. Son")));
    }

    #[test]
    fn epigraph() {
        let e = Epigraph::parse("Bir söz.\n— Biri\n\n13.10.2026\n").unwrap();
        assert_eq!(e.lines, vec!["Bir söz.", "— Biri"]);
        assert_eq!(e.date, NaiveDate::from_ymd_opt(2026, 10, 13).unwrap());
    }
}
