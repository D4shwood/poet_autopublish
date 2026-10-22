//! The bridge between the shared Rust core and the Android app.
//!
//! The app reads the poems folder itself (files and the `.poet/` settings) and
//! hands the text over. Everything that has rules — signatures, pen names,
//! weeks, collections, publications, search, the writing screen's file format —
//! is decided here by the same code as the desktop app.

use chrono::{Local, NaiveTime};
use poetautopublish::authors::{self as core_authors, Author, Authors};
use poetautopublish::lang::{self, Lang};
use poetautopublish::poem::{Poem, Scan, Signature, Status};
use poetautopublish::{compose, prosody, search, shared, stats, volume};
use std::collections::HashMap;
use std::path::Path;

uniffi::setup_scaffolding!();

// ------------------------------------------------------------------ input

/// A poem file as the app read it.
#[derive(uniffi::Record)]
pub struct PoemFile {
    /// The file name inside its folder.
    pub file_name: String,
    /// The subfolder (collection) it is in; None for the weekly series.
    pub collection: Option<String>,
    pub text: String,
    /// Last change, in milliseconds since 1970; orders the drafts.
    pub modified: i64,
}

/// The texts of the folder's `.poet/` files; None when a file is missing.
#[derive(uniffi::Record, Default)]
pub struct SharedTexts {
    pub profile: Option<String>,
    pub authors: Option<String>,
    pub weeks: Option<String>,
}

// ------------------------------------------------------------------ output

#[derive(uniffi::Record, Clone)]
pub struct Profile {
    pub author: String,
    pub series: String,
    /// "auto", "tr" or "en".
    pub lang: String,
    /// "HH:MM".
    pub night_cutoff: String,
    /// How unnamed weeks are named, e.g. "{week}. Hafta {yy}'"; "" for dates.
    pub week_format: String,
}

#[derive(uniffi::Record, Clone)]
pub struct AuthorEntry {
    pub name: String,
    pub pen_names: Vec<String>,
}

#[derive(uniffi::Enum, Debug, PartialEq, Clone)]
pub enum PoemStatus {
    Published,
    Draft,
    Unsigned,
    /// Signed by another author; tracked but not published.
    Foreign {
        author: String,
    },
}

#[derive(uniffi::Record, Clone)]
pub struct PoemInfo {
    /// "Folder/name" for a poem in a collection, else the file name.
    pub key: String,
    pub file_name: String,
    pub collection: Option<String>,
    pub title: String,
    pub body: Vec<String>,
    pub status: PoemStatus,
    /// The name as signed, which may be a pen name.
    pub signed_as: Option<String>,
    /// "05.10.2026"; None for an undated signature.
    pub date: Option<String>,
    /// The list column, as the terminal shows it: "~05.10 21.30 ☾".
    pub when: String,
    pub approx: bool,
    /// Where it was first published, e.g. "Mürekkep #2".
    pub publication: Option<String>,
    pub words: u32,
    pub lines: u32,
    pub modified: i64,
}

#[derive(uniffi::Record)]
pub struct WeekInfo {
    /// ISO week, "2026-W41"; the key for a custom name.
    pub key: String,
    /// "Cilt N".
    pub number: u32,
    /// The custom name, or "5 Ekim Haftası" / "Week of 5 October".
    pub label: String,
    /// Its PDF's file name in the published folder.
    pub pdf: String,
    /// Keys of its poems, oldest first.
    pub poems: Vec<String>,
}

#[derive(uniffi::Record)]
pub struct Bar {
    pub label: String,
    pub count: u64,
}

#[derive(uniffi::Record)]
pub struct StatsInfo {
    pub poems: u32,
    pub lines: u32,
    pub words: u32,
    pub streak: u32,
    pub longest_streak: u32,
    /// Poems per week (the series) or per year (a collection), oldest first.
    pub per_group: Vec<Bar>,
    /// Signed poems per hour of the day, 24 entries.
    pub by_hour: Vec<u64>,
}

/// The series (name None) or one collection.
#[derive(uniffi::Record)]
pub struct SpaceInfo {
    pub name: Option<String>,
    /// Every poem file in it, whatever its status.
    pub poems: Vec<String>,
    /// Its published poems: newest week first for the series, by title for a collection.
    pub published: Vec<String>,
    /// The PDF of the whole space (the book, or the collection's book).
    pub pdf: String,
    pub stats: StatsInfo,
}

/// One poem in the publications tree, in drawing order (journal → issue → title).
#[derive(uniffi::Record)]
pub struct PubEntry {
    pub journal: String,
    /// "#2", "#2 · Almanak" or "Almanak"; None without issue or edition.
    pub issue: Option<String>,
    pub key: String,
    /// "~31.10.2024", or empty for an undated signature.
    pub when: String,
    /// The signature, unless it is the user's real name (which is on the cover).
    pub shown_author: Option<String>,
}

#[derive(uniffi::Enum)]
pub enum SigRow {
    Author {
        index: u32,
        label: String,
        count: u32,
        is_main: bool,
        has_name: bool,
    },
    Pen {
        author: u32,
        index: u32,
        name: String,
        count: u32,
    },
    /// A signature no author claims.
    Unknown { name: String, count: u32 },
}

#[derive(uniffi::Record)]
pub struct Hit {
    pub key: String,
    pub title: String,
    pub line: String,
}

// ------------------------------------------------------------------ the library

/// The parsed poems folder. Built once per refresh; queries are cheap.
#[derive(uniffi::Object)]
pub struct PoetCore {
    scan: Scan,
    authors: Authors,
    profile: shared::Profile,
    lang: Lang,
    cutoff: NaiveTime,
    weeks: Vec<volume::Week>,
    collections: Vec<volume::Collection>,
    modified: HashMap<String, i64>,
}

#[uniffi::export]
impl PoetCore {
    #[uniffi::constructor]
    pub fn new(files: Vec<PoemFile>, texts: SharedTexts) -> std::sync::Arc<PoetCore> {
        let profile = texts
            .profile
            .as_deref()
            .and_then(shared::profile_from_toml)
            .unwrap_or_default();
        let authors = Authors::new(
            texts
                .authors
                .as_deref()
                .map(shared::authors_from_toml)
                .unwrap_or_default(),
            &profile.author,
        );
        let mut scan = Scan::default();
        let mut folders: std::collections::BTreeMap<String, Vec<Poem>> = Default::default();
        let mut modified = HashMap::new();
        for f in files {
            let mut p = Poem::parse(Path::new(&f.file_name), &f.text);
            if let Some(c) = &f.collection {
                p.file_name = format!("{c}/{}", p.file_name);
                p.collection = Some(c.clone());
            }
            modified.insert(p.file_name.clone(), f.modified);
            match f.collection {
                Some(c) => folders.entry(c).or_default().push(p),
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
        let names = texts
            .weeks
            .as_deref()
            .map(shared::weeks_from_toml)
            .unwrap_or_default();
        volume::name_weeks(&mut weeks, &names, &profile.week_format);
        let collections = volume::collections(&scan.collections, &authors);
        std::sync::Arc::new(PoetCore {
            scan,
            authors,
            profile,
            lang,
            cutoff,
            weeks,
            collections,
            modified,
        })
    }

    /// "tr" or "en", after resolving "auto".
    pub fn lang(&self) -> String {
        code(self.lang).into()
    }

    pub fn profile(&self) -> Profile {
        Profile {
            author: self.profile.author.clone(),
            series: self.profile.series.clone(),
            lang: self.profile.lang.clone(),
            night_cutoff: self.profile.night_cutoff.clone(),
            week_format: self.profile.week_format.clone(),
        }
    }

    pub fn authors(&self) -> Vec<AuthorEntry> {
        self.authors
            .list
            .iter()
            .map(|a| AuthorEntry {
                name: a.name.clone(),
                pen_names: a.pen_names.clone(),
            })
            .collect()
    }

    pub fn main_author(&self) -> u32 {
        self.authors.main as u32
    }

    /// Every poem file, series first, then each collection.
    pub fn poems(&self) -> Vec<PoemInfo> {
        self.scan.all_poems().map(|p| self.info(p)).collect()
    }

    /// The series' weeks, oldest first.
    pub fn weeks(&self) -> Vec<WeekInfo> {
        self.weeks
            .iter()
            .map(|w| WeekInfo {
                key: w.key.clone(),
                number: w.number as u32,
                label: w.label(self.lang),
                pdf: w.file_name.clone(),
                poems: w.poems.iter().map(|p| p.file_name.clone()).collect(),
            })
            .collect()
    }

    /// Collection names, in folder order.
    pub fn collections(&self) -> Vec<String> {
        self.scan
            .collections
            .iter()
            .map(|(n, _)| n.clone())
            .collect()
    }

    /// The series (`None`) or a collection.
    pub fn space(&self, name: Option<String>) -> SpaceInfo {
        let today = Local::now().date_naive();
        match name.as_deref().and_then(|n| self.collection_index(n)) {
            None => {
                let published: Vec<&Poem> =
                    self.weeks.iter().flat_map(|w| w.poems.iter()).collect();
                SpaceInfo {
                    name: None,
                    poems: self
                        .scan
                        .poems
                        .iter()
                        .map(|p| p.file_name.clone())
                        .collect(),
                    published: self
                        .weeks
                        .iter()
                        .rev()
                        .flat_map(|w| w.poems.iter())
                        .map(|p| p.file_name.clone())
                        .collect(),
                    pdf: volume::book_file_name(&self.profile.series),
                    stats: stats_info(stats::compute(
                        &published,
                        stats::per_week(&self.weeks, self.lang),
                        today,
                    )),
                }
            }
            Some(i) => {
                let c = &self.collections[i];
                let published: Vec<&Poem> = c.poems.iter().collect();
                let mut all: Vec<&Poem> = self.scan.collections[i].1.iter().collect();
                all.sort_by_key(|p| volume::turkish_key(&p.title));
                SpaceInfo {
                    name: Some(c.name.clone()),
                    poems: all.iter().map(|p| p.file_name.clone()).collect(),
                    published: published.iter().map(|p| p.file_name.clone()).collect(),
                    pdf: c.file_name.clone(),
                    stats: stats_info(stats::compute(
                        &published,
                        stats::per_year(&published),
                        today,
                    )),
                }
            }
        }
    }

    /// The space's publications tree, flattened in drawing order.
    pub fn publications(&self, space: Option<String>) -> Vec<PubEntry> {
        let poems = self.space_poems(space.as_deref());
        let main_name = core_authors::norm(&self.authors.main_author().name);
        volume::publication_order(&poems, &self.authors)
            .into_iter()
            .map(|p| {
                let sig = p.signature.as_ref().unwrap();
                let pubn = sig.publication().unwrap();
                let mut when = sig
                    .date
                    .map(|d| d.format("%d.%m.%Y").to_string())
                    .unwrap_or_default();
                if sig.approx {
                    when.insert(0, '~');
                }
                PubEntry {
                    journal: pubn.name.clone(),
                    issue: pubn.issue_label(),
                    key: p.file_name.clone(),
                    when,
                    shown_author: (core_authors::norm(&sig.author) != main_name)
                        .then(|| sig.author.clone()),
                }
            })
            .collect()
    }

    /// The İmzalar table: the main author first, then the others with their pen
    /// names, then signatures nobody claims.
    pub fn signature_rows(&self) -> Vec<SigRow> {
        let counts = core_authors::signature_counts(self.scan.all_poems());
        let n = |s: &str| core_authors::count_of(&counts, s) as u32;
        let mut rows = vec![];
        for i in self.authors.order() {
            let a = &self.authors.list[i];
            let total = n(&a.name) * (!a.name.trim().is_empty()) as u32
                + a.pen_names.iter().map(|p| n(p)).sum::<u32>();
            rows.push(SigRow::Author {
                index: i as u32,
                label: a.label(),
                count: total,
                is_main: i == self.authors.main,
                has_name: !a.name.trim().is_empty(),
            });
            for (j, pen) in a.pen_names.iter().enumerate() {
                rows.push(SigRow::Pen {
                    author: i as u32,
                    index: j as u32,
                    name: pen.clone(),
                    count: n(pen),
                });
            }
        }
        for name in self.authors.unclaimed(&counts) {
            let count = n(&name);
            rows.push(SigRow::Unknown { name, count });
        }
        rows
    }

    /// Titles and lines that contain the query (Turkish-aware), in one space.
    pub fn search(&self, query: String, space: Option<String>) -> Vec<Hit> {
        let poems: Vec<Poem> = self
            .space_poems(space.as_deref())
            .into_iter()
            .cloned()
            .collect();
        search::search_text(&poems, &query)
            .into_iter()
            .map(|h| Hit {
                key: h.file_name,
                title: h.title,
                line: h.line,
            })
            .collect()
    }

    /// Lines from the user's own poems for the blank page.
    pub fn inspiration_lines(&self) -> Vec<String> {
        let mine: Vec<Poem> = self
            .scan
            .all_poems()
            .filter(|p| p.status(&self.authors) == Status::Published)
            .cloned()
            .collect();
        compose::inspiration_lines(&mine)
    }

    /// A UI string in the library's language, from the same table as the terminal.
    pub fn t(&self, key: String) -> String {
        self.lang.t(&key).to_string()
    }
}

impl PoetCore {
    fn collection_index(&self, name: &str) -> Option<usize> {
        self.collections.iter().position(|c| c.name == name)
    }

    fn space_poems(&self, space: Option<&str>) -> Vec<&Poem> {
        match space.and_then(|n| self.collection_index(n)) {
            None => self.scan.poems.iter().collect(),
            Some(i) => self.scan.collections[i].1.iter().collect(),
        }
    }

    fn info(&self, p: &Poem) -> PoemInfo {
        let sig = p.signature.as_ref();
        PoemInfo {
            key: p.file_name.clone(),
            file_name: Path::new(&p.file_name)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            collection: p.collection.clone(),
            title: p.title.clone(),
            body: p.body.clone(),
            status: match p.status(&self.authors) {
                Status::Published => PoemStatus::Published,
                Status::Draft => PoemStatus::Draft,
                Status::Unsigned => PoemStatus::Unsigned,
                Status::Foreign(author) => PoemStatus::Foreign { author },
            },
            signed_as: sig.map(|s| s.author.clone()),
            date: sig
                .and_then(|s| s.date)
                .map(|d| d.format("%d.%m.%Y").to_string()),
            when: sig.map(|s| when_column(s, self.cutoff)).unwrap_or_default(),
            approx: sig.is_some_and(|s| s.approx),
            publication: sig.and_then(|s| s.publication.clone()),
            words: p.word_count() as u32,
            lines: p.line_count() as u32,
            modified: self.modified.get(&p.file_name).copied().unwrap_or(0),
        }
    }
}

/// The terminal's date column: "dd.mm", "~" when approximate, " HH.MM" when
/// timed, " ☾" when the night cutoff moves it to the day before.
fn when_column(s: &Signature, cutoff: NaiveTime) -> String {
    let Some(date) = s.date else {
        return String::new();
    };
    let mut w = date.format("%d.%m").to_string();
    if s.approx {
        w.insert(0, '~');
    }
    if let Some(t) = s.time {
        w.push_str(&t.format(" %H.%M").to_string());
    }
    if s.effective_date(cutoff) != s.date {
        w.push_str(" ☾");
    }
    w
}

fn stats_info(s: stats::Stats) -> StatsInfo {
    StatsInfo {
        poems: s.poems as u32,
        lines: s.lines as u32,
        words: s.words as u32,
        streak: s.streak as u32,
        longest_streak: s.longest_streak as u32,
        per_group: s
            .per_group
            .into_iter()
            .map(|(label, count)| Bar { label, count })
            .collect(),
        by_hour: s.by_hour.to_vec(),
    }
}

fn code(l: Lang) -> &'static str {
    if l == Lang::Tr { "tr" } else { "en" }
}

// ------------------------------------------------------------------ writing

/// A poem file opened for editing: lines without the draft tag.
#[derive(uniffi::Record)]
pub struct DraftText {
    pub lines: Vec<String>,
    pub is_draft: bool,
}

#[uniffi::export]
pub fn open_poem(raw: String) -> DraftText {
    let d = compose::open(&raw);
    DraftText {
        lines: d.lines,
        is_draft: d.is_draft,
    }
}

/// The file text for the lines; the draft tag goes after the title.
#[uniffi::export]
pub fn poem_content(lines: Vec<String>, is_draft: bool) -> String {
    compose::content(&lines, is_draft)
}

/// The lines with the signature appended, signed now.
#[uniffi::export]
pub fn sign_poem(lines: Vec<String>, author: String) -> Vec<String> {
    let mut lines = lines;
    compose::sign(&mut lines, &author, Local::now().naive_local());
    lines
}

#[uniffi::export]
pub fn is_signed(lines: Vec<String>) -> bool {
    compose::is_signed(&lines)
}

#[uniffi::export]
pub fn word_count(lines: Vec<String>) -> u32 {
    compose::word_count(&lines) as u32
}

#[uniffi::export]
pub fn line_count(lines: Vec<String>) -> u32 {
    compose::line_count(&lines) as u32
}

/// The file name for a title, made unique against the names already taken.
#[uniffi::export]
pub fn file_name_for(title: String, taken: Vec<String>) -> String {
    compose::unique_name(&compose::sanitize(&title), |n| taken.iter().any(|t| t == n))
}

/// One line's syllables and rhyme, for the writer's margin.
#[derive(uniffi::Record)]
pub struct LineMark {
    /// Index into the lines given (0 is the title).
    pub line: u32,
    pub syllables: u32,
    /// "a", "b+r" (the r marks a redif); None when it rhymes with nothing.
    pub rhyme: Option<String>,
    /// The line breaks the meter most lines keep.
    pub off: bool,
}

#[derive(uniffi::Record)]
pub struct Verse {
    /// "7+7", "11"; None for free verse.
    pub meter: Option<String>,
    pub fits: u32,
    pub of: u32,
    pub marks: Vec<LineMark>,
}

/// Syllables, rhymes and the meter of a poem being written. The title, blank
/// lines and the signature get no mark.
#[uniffi::export]
pub fn verse(lines: Vec<String>) -> Verse {
    let idx: Vec<usize> = (1..lines.len())
        .filter(|&i| !lines[i].trim().is_empty() && Signature::parse(&lines[i]).is_none())
        .collect();
    let texts: Vec<&str> = idx.iter().map(|&i| lines[i].as_str()).collect();
    let meter = prosody::meter(&texts);
    let rhymes = prosody::rhymes(&texts);
    let marks = idx
        .iter()
        .enumerate()
        .map(|(k, &i)| LineMark {
            line: i as u32,
            syllables: prosody::line_shape(texts[k]).total as u32,
            rhyme: rhymes[k]
                .as_ref()
                .map(|r| format!("{}{}", r.letter, if r.redif { "+r" } else { "" })),
            off: meter.as_ref().is_some_and(|m| !m.fits(texts[k])),
        })
        .collect();
    Verse {
        fits: meter.as_ref().map_or(0, |m| m.fits as u32),
        of: meter.as_ref().map_or(texts.len() as u32, |m| m.of as u32),
        meter: meter.map(|m| m.label()),
        marks,
    }
}

/// Whether a file in the poems folder is a poem, by the desktop's rule.
#[uniffi::export]
pub fn is_poem_file_name(name: String) -> bool {
    poetautopublish::poem::is_poem_file(Path::new(&name))
}

// ------------------------------------------------------------------ the .poet/ files

#[uniffi::export]
pub fn profile_toml(profile: Profile) -> String {
    shared::profile_to_toml(&shared::Profile {
        author: profile.author.trim().to_string(),
        series: profile.series.trim().to_string(),
        lang: profile.lang,
        night_cutoff: profile.night_cutoff,
        week_format: profile.week_format,
    })
}

/// weeks.toml after renaming one week; an empty name restores the default.
#[uniffi::export]
pub fn rename_week(weeks: Option<String>, key: String, name: String) -> String {
    let mut names = weeks
        .as_deref()
        .map(shared::weeks_from_toml)
        .unwrap_or_default();
    shared::rename_week(&mut names, &key, &name);
    shared::weeks_to_toml(&names)
}

#[derive(uniffi::Enum)]
pub enum AuthorEdit {
    AddPen { author: u32, pen: String },
    RemovePen { author: u32, pen: u32 },
    AddAuthor { name: String, pen: String },
    RemoveAuthor { author: u32 },
}

/// authors.toml after one edit, or None when the edit isn't allowed (a name
/// that already belongs to someone, removing the main author…).
#[uniffi::export]
pub fn edit_authors(
    authors: Option<String>,
    main_name: String,
    edit: AuthorEdit,
) -> Option<String> {
    let list: Vec<Author> = authors
        .as_deref()
        .map(shared::authors_from_toml)
        .unwrap_or_default();
    let mut a = Authors::new(list, &main_name);
    let ok = match edit {
        AuthorEdit::AddPen { author, pen } => {
            (author as usize) < a.list.len() && a.add_pen_name(author as usize, &pen)
        }
        AuthorEdit::RemovePen { author, pen } => {
            a.remove_pen_name(author as usize, pen as usize).is_some()
        }
        AuthorEdit::AddAuthor { name, pen } => a.add_author(&name, &pen),
        AuthorEdit::RemoveAuthor { author } => a.remove_author(author as usize).is_some(),
    };
    ok.then(|| shared::authors_to_toml(&a.list))
}

/// The authors.toml text after the setup screen: the main author's real name
/// and pen names replaced, everyone else kept.
#[uniffi::export]
pub fn set_main_author(
    authors: Option<String>,
    old_name: String,
    name: String,
    pens: Vec<String>,
) -> String {
    let list: Vec<Author> = authors
        .as_deref()
        .map(shared::authors_from_toml)
        .unwrap_or_default();
    let old = if old_name.trim().is_empty() {
        &name
    } else {
        &old_name
    };
    let mut a = Authors::new(list, old);
    let main = a.main;
    a.list[main].name = name.trim().to_string();
    a.list[main].pen_names.clear();
    for pen in pens {
        a.add_pen_name(main, &pen);
    }
    shared::authors_to_toml(&a.list)
}

/// A UI string for a language code, before any poems are read (setup screen).
#[uniffi::export]
pub fn tr(lang: String, key: String) -> String {
    let l = match lang.as_str() {
        "en" => Lang::En,
        "tr" => Lang::Tr,
        _ => lang::system_lang(),
    };
    l.t(&key).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, collection: Option<&str>, text: &str) -> PoemFile {
        PoemFile {
            file_name: name.into(),
            collection: collection.map(Into::into),
            text: text.into(),
            modified: 0,
        }
    }

    fn texts() -> SharedTexts {
        SharedTexts {
            profile: Some("author = \"Deniz Yalın\"\nlang = \"auto\"\n".into()),
            authors: Some(
                "[[author]]\nname = \"Deniz Yalın\"\npen_names = [\"K.M Ozan\"]\n\n\
                 [[author]]\nname = \"Ece Tan\"\n"
                    .into(),
            ),
            weeks: Some("\"2026-W41\" = \"İlk\"\n".into()),
        }
    }

    #[test]
    fn library() {
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
            file(
                "Konuk",
                None,
                "Konuk\n\nbaşkası\n\n- Ece Tan. 06.10.2026 Mürekkep #1\n",
            ),
            file(
                "Dawn",
                Some("Eski"),
                "Dawn\n\nQuietly\n\n- K.M Ozan. Mürekkep #2\n",
            ),
            file(
                "Kayıp",
                Some("Eski"),
                "Kayıp\n\nkim yazdı\n\n- Gölge. 01.01.2020\n",
            ),
        ];
        let core = PoetCore::new(files, texts());
        assert_eq!(core.lang(), "tr");

        let weeks = core.weeks();
        assert_eq!(weeks.len(), 1);
        assert_eq!(weeks[0].label, "İlk");
        assert_eq!(weeks[0].poems, vec!["Liman", "Gece"]);

        let poems = core.poems();
        let by = |k: &str| poems.iter().find(|p| p.key == k).unwrap();
        assert_eq!(by("Gece").when, "12.10 01.30 ☾");
        assert_eq!(by("Taslak").status, PoemStatus::Draft);
        assert_eq!(by("Eski/Dawn").status, PoemStatus::Published);

        let series = core.space(None);
        assert_eq!(series.stats.poems, 2);
        assert_eq!(series.published, vec!["Liman", "Gece"]);
        let eski = core.space(Some("Eski".into()));
        assert_eq!(
            eski.poems,
            vec!["Dawn", "Kayıp"]
                .iter()
                .map(|n| format!("Eski/{n}"))
                .collect::<Vec<_>>()
        );

        // The series' publications include the other author's poem, with their name.
        let pubs = core.publications(None);
        assert_eq!(pubs.len(), 1);
        assert_eq!(
            (pubs[0].journal.as_str(), pubs[0].issue.as_deref()),
            ("Mürekkep", Some("#1"))
        );
        assert_eq!(pubs[0].shown_author.as_deref(), Some("Ece Tan"));
        // A pen name shows; the real name would not.
        assert_eq!(
            core.publications(Some("Eski".into()))[0]
                .shown_author
                .as_deref(),
            Some("K.M Ozan")
        );

        let rows = core.signature_rows();
        assert!(matches!(
            &rows[0],
            SigRow::Author {
                is_main: true,
                count: 3,
                ..
            }
        ));
        assert!(
            rows.iter()
                .any(|r| matches!(r, SigRow::Unknown { name, count: 1 } if name == "Gölge"))
        );

        // Turkish case: "MISRA" is "mısra", "misra" is not.
        assert_eq!(core.search("MISRA".into(), None).len(), 2);
        assert_eq!(core.search("misra".into(), None).len(), 0);
        assert_eq!(core.t("drafts".into()), "✎ Taslaklar");
    }

    #[test]
    fn week_pattern() {
        let mut t = texts();
        t.profile =
            Some("author = \"Deniz Yalın\"\nweek_format = \"{week}. Hafta {yy}'\"\n".into());
        t.weeks = None;
        let core = PoetCore::new(
            vec![file(
                "Liman",
                None,
                "Liman\n\nbir\n\n- Deniz Yalın. 27.10.2026 - 21.30\n",
            )],
            t,
        );
        assert_eq!(core.weeks()[0].label, "44. Hafta 26'");
        assert_eq!(core.space(None).stats.per_group[0].label, "44. Hafta 26'");
    }

    #[test]
    fn verse_marks() {
        let v = verse(
            [
                "Kıyı",
                "",
                "Gece ağır bir rüzgâr eser uzak kıyıdan",
                "Bir kuş konar dallara eser serin kıyıdan",
                "Deniz yine sessizdi",
                "",
                "- Deniz Yalın. 05.10.2026 - 21.30",
            ]
            .map(String::from)
            .to_vec(),
        );
        assert_eq!((v.meter.as_deref(), v.fits, v.of), (Some("7+7"), 2, 3));
        assert_eq!(v.marks.len(), 3);
        assert_eq!(v.marks[0].rhyme.as_deref(), Some("a+r"));
        assert!(v.marks[2].off);
    }

    #[test]
    fn writing() {
        let d = open_poem("Liman\n#taslak\n\nyarım\n".into());
        assert!(d.is_draft);
        let signed = sign_poem(d.lines, "Deniz Yalın".into());
        assert!(is_signed(signed.clone()));
        assert!(poem_content(signed, false).starts_with("Liman\n\nyarım\n\n- Deniz Yalın. "));
        assert_eq!(
            file_name_for("Kum/Fener".into(), vec!["Kum-Fener".into()]),
            "Kum-Fener (2)"
        );
    }

    #[test]
    fn settings_files() {
        let t = texts();
        let after = edit_authors(
            t.authors.clone(),
            "Deniz Yalın".into(),
            AuthorEdit::AddPen {
                author: 1,
                pen: "E.T".into(),
            },
        )
        .unwrap();
        assert!(after.contains("E.T"));
        assert!(
            edit_authors(
                t.authors.clone(),
                "Deniz Yalın".into(),
                AuthorEdit::AddPen {
                    author: 1,
                    pen: "k.m. ozan".into()
                }
            )
            .is_none()
        );
        assert!(
            edit_authors(
                t.authors.clone(),
                "Deniz Yalın".into(),
                AuthorEdit::RemoveAuthor { author: 0 }
            )
            .is_none()
        );

        let set = set_main_author(
            t.authors,
            "Deniz Yalın".into(),
            "Deniz Yalın".into(),
            vec!["D.Y".into()],
        );
        assert!(set.contains("D.Y") && !set.contains("K.M Ozan") && set.contains("Ece Tan"));

        assert_eq!(
            rename_week(None, "2026-W41".into(), "İlk".into()).trim(),
            "2026-W41 = \"İlk\""
        );
    }
}
