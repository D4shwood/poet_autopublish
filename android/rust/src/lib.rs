//! The bridge between the shared Rust core and the Android app.
//!
//! Android reads the poems folder through the storage picker, which hands out
//! content URIs rather than paths, so the app reads each file's text itself and
//! passes it here. Everything that has rules (signatures, pen names, weeks,
//! stats) is decided by the same code as the desktop app.

use chrono::{Local, NaiveDate, NaiveTime};
use poetautopublish::authors::{Author, Authors};
use poetautopublish::lang::{self, Lang};
use poetautopublish::poem::{Poem, Scan, Signature, Status};
use poetautopublish::{stats, volume};
use std::path::Path;

uniffi::setup_scaffolding!();

/// A poem file as the app read it.
#[derive(uniffi::Record)]
pub struct PoemFile {
    /// The file name inside its folder.
    pub file_name: String,
    /// The subfolder (collection) it is in; None for the weekly series.
    pub collection: Option<String>,
    pub text: String,
}

#[derive(uniffi::Record)]
pub struct AuthorEntry {
    /// The real name; may be empty when only pen names are known.
    pub name: String,
    pub pen_names: Vec<String>,
}

/// Who the user is and how their series works, from the app's settings.
#[derive(uniffi::Record)]
pub struct Profile {
    /// The name the user signs with.
    pub author: String,
    /// Every author, the user included, with their pen names.
    pub authors: Vec<AuthorEntry>,
    pub series: String,
    /// "auto", "tr" or "en".
    pub lang: String,
    /// "HH:MM"; poems signed before it belong to the day before.
    pub night_cutoff: String,
}

#[derive(uniffi::Enum, Debug, PartialEq)]
pub enum PoemStatus {
    Published,
    Draft,
    Unsigned,
    /// Signed by another author; tracked but not published.
    Foreign {
        author: String,
    },
}

#[derive(uniffi::Record)]
pub struct PoemInfo {
    /// "Folder/name" for a poem in a collection, else the file name.
    pub key: String,
    pub collection: Option<String>,
    pub title: String,
    pub body: Vec<String>,
    pub status: PoemStatus,
    /// The name as signed, which may be a pen name.
    pub signed_as: Option<String>,
    /// ISO date ("2026-10-05"); None for an undated signature.
    pub date: Option<String>,
    /// "21:30" when the signature has a time.
    pub time: Option<String>,
    /// The day is approximate (`~05.10.2026`).
    pub approx: bool,
    /// Where it was first published, e.g. "Mürekkep #2".
    pub publication: Option<String>,
    pub words: u32,
    pub lines: u32,
}

#[derive(uniffi::Record)]
pub struct WeekInfo {
    /// ISO week, "2026-W41".
    pub key: String,
    /// "Cilt N".
    pub number: u32,
    /// The custom name, or "5 Ekim Haftası" / "Week of 5 October".
    pub label: String,
    /// Keys of its poems, oldest first.
    pub poems: Vec<String>,
}

#[derive(uniffi::Record)]
pub struct StatsInfo {
    pub poems: u32,
    pub lines: u32,
    pub words: u32,
    pub streak: u32,
    pub longest_streak: u32,
    /// Signed poems per hour of the day, 24 entries.
    pub by_hour: Vec<u64>,
}

#[derive(uniffi::Record)]
pub struct PoetLibrary {
    /// "tr" or "en", after resolving "auto".
    pub lang: String,
    pub poems: Vec<PoemInfo>,
    pub weeks: Vec<WeekInfo>,
    pub stats: StatsInfo,
}

/// Reads every poem and groups the user's published ones into weeks.
/// `week_names` maps ISO week keys to names the user gave them.
#[uniffi::export]
pub fn library(
    files: Vec<PoemFile>,
    profile: Profile,
    week_names: std::collections::HashMap<String, String>,
) -> PoetLibrary {
    let authors = authors(&profile);
    let mut scan = Scan::default();
    let mut folders: std::collections::BTreeMap<String, Vec<Poem>> = Default::default();
    for f in files {
        let mut p = Poem::parse(Path::new(&f.file_name), &f.text);
        match f.collection {
            Some(c) => {
                p.file_name = format!("{c}/{}", p.file_name);
                p.collection = Some(c.clone());
                folders.entry(c).or_default().push(p);
            }
            None => scan.poems.push(p),
        }
    }
    scan.poems.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    scan.collections = folders.into_iter().collect();
    scan.sign_undated(&authors);

    let lang = match profile.lang.as_str() {
        "tr" => Lang::Tr,
        "en" => Lang::En,
        _ => lang::resolve("auto", &scan.poems),
    };
    let cutoff = NaiveTime::parse_from_str(profile.night_cutoff.trim(), "%H:%M")
        .unwrap_or(NaiveTime::from_hms_opt(5, 0, 0).unwrap());

    let mut weeks = volume::group(
        &scan.poems,
        &scan.epigraphs,
        &authors,
        cutoff,
        &profile.series,
    );
    for w in &mut weeks {
        w.name = week_names.get(&w.key).cloned();
    }

    let published: Vec<&Poem> = weeks.iter().flat_map(|w| w.poems.iter()).collect();
    let s = stats::compute(
        &published,
        stats::per_week(&weeks, lang),
        Local::now().date_naive(),
    );

    PoetLibrary {
        lang: if lang == Lang::Tr { "tr" } else { "en" }.into(),
        poems: scan.all_poems().map(|p| info(p, &authors)).collect(),
        weeks: weeks
            .iter()
            .map(|w| WeekInfo {
                key: w.key.clone(),
                number: w.number as u32,
                label: w.label(lang),
                poems: w.poems.iter().map(|p| p.file_name.clone()).collect(),
            })
            .collect(),
        stats: StatsInfo {
            poems: s.poems as u32,
            lines: s.lines as u32,
            words: s.words as u32,
            streak: s.streak as u32,
            longest_streak: s.longest_streak as u32,
            by_hour: s.by_hour.to_vec(),
        },
    }
}

/// The signature line "Sign & publish" appends: `- Name. 05.10.2026 - 21.30`.
#[uniffi::export]
pub fn signature_now(author: String) -> String {
    Signature::format(&author, Local::now().naive_local())
}

/// Whether a line is a valid dated signature, for live feedback while typing.
#[uniffi::export]
pub fn is_signature(line: String) -> bool {
    Signature::parse(&line).is_some()
}

/// Whether a file in the poems folder is a poem (no extension or `.txt`,
/// not hidden), by the same rule as the desktop app.
#[uniffi::export]
pub fn is_poem_file_name(name: String) -> bool {
    poetautopublish::poem::is_poem_file(Path::new(&name))
}

fn authors(profile: &Profile) -> Authors {
    let list = profile
        .authors
        .iter()
        .map(|a| Author {
            name: a.name.clone(),
            pen_names: a.pen_names.clone(),
        })
        .collect();
    Authors::new(list, &profile.author)
}

fn info(p: &Poem, authors: &Authors) -> PoemInfo {
    let sig = p.signature.as_ref();
    PoemInfo {
        key: p.file_name.clone(),
        collection: p.collection.clone(),
        title: p.title.clone(),
        body: p.body.clone(),
        status: match p.status(authors) {
            Status::Published => PoemStatus::Published,
            Status::Draft => PoemStatus::Draft,
            Status::Unsigned => PoemStatus::Unsigned,
            Status::Foreign(author) => PoemStatus::Foreign { author },
        },
        signed_as: sig.map(|s| s.author.clone()),
        date: sig.and_then(|s| s.date).map(|d: NaiveDate| d.to_string()),
        time: sig
            .and_then(|s| s.time)
            .map(|t| t.format("%H:%M").to_string()),
        approx: sig.is_some_and(|s| s.approx),
        publication: sig.and_then(|s| s.publication.clone()),
        words: p.word_count() as u32,
        lines: p.line_count() as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Profile {
        Profile {
            author: "Deniz Yalın".into(),
            authors: vec![
                AuthorEntry {
                    name: "Deniz Yalın".into(),
                    pen_names: vec!["K.M Ozan".into()],
                },
                AuthorEntry {
                    name: "Ece Tan".into(),
                    pen_names: vec![],
                },
            ],
            series: "Contra Noir".into(),
            lang: "auto".into(),
            night_cutoff: "05:00".into(),
        }
    }

    fn file(name: &str, collection: Option<&str>, text: &str) -> PoemFile {
        PoemFile {
            file_name: name.into(),
            collection: collection.map(Into::into),
            text: text.into(),
        }
    }

    #[test]
    fn groups_and_classifies() {
        let files = vec![
            file(
                "Liman",
                None,
                "Liman\n\nbir mısra\n\n- Deniz Yalın. 05.10.2026 - 21.30\n",
            ),
            // 01.30 on Monday the 12th belongs to Sunday the 11th, the week before.
            file(
                "Gece",
                None,
                "Gece\n\nbir mısra\n\n- K.M Ozan. 12.10.2026 - 01.30\n",
            ),
            file("Taslak", None, "Taslak\n#taslak\n\nyarım\n"),
            file("Konuk", None, "Konuk\n\nbaşkası\n\n- Ece Tan. 06.10.2026\n"),
            file(
                "Dawn",
                Some("Eski"),
                "Dawn\n\nQuietly\n\n- K.M Ozan. Mürekkep #2\n",
            ),
        ];
        let lib = library(
            files,
            profile(),
            [("2026-W41".to_string(), "İlk".to_string())].into(),
        );

        assert_eq!(lib.lang, "tr");
        assert_eq!(lib.weeks.len(), 1);
        assert_eq!(lib.weeks[0].label, "İlk");
        assert_eq!(lib.weeks[0].poems, vec!["Liman", "Gece"]);
        assert_eq!(lib.stats.poems, 2);

        let status = |k: &str| &lib.poems.iter().find(|p| p.key == k).unwrap().status;
        assert_eq!(*status("Taslak"), PoemStatus::Draft);
        assert_eq!(
            *status("Konuk"),
            PoemStatus::Foreign {
                author: "Ece Tan".into()
            }
        );
        let dawn = lib.poems.iter().find(|p| p.key == "Eski/Dawn").unwrap();
        assert_eq!(dawn.status, PoemStatus::Published);
        assert_eq!(dawn.publication.as_deref(), Some("Mürekkep #2"));
        assert_eq!(dawn.date, None);
    }

    #[test]
    fn signs() {
        let line = signature_now("Deniz Yalın".into());
        assert!(line.starts_with("- Deniz Yalın. "));
        assert!(is_signature(line));
    }
}
