#[cfg(feature = "preview")]
use super::preview::Preview;
use super::writer::{self, Editor};
use crate::build::{self, Library};
use crate::config::{Config, log_path};
use crate::history::{self, Snapshot};
use crate::install;
use crate::lang::Lang;
#[cfg(feature = "preview")]
use crate::latex::Ctx;
use crate::poem::{Poem, Status};
use crate::search::{self, Hit};
use crate::watch::{self, Watcher};
use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

#[derive(Clone, Copy, PartialEq)]
pub enum Item {
    NewPoem,
    ContinueDraft,
    Week(usize),
    All,
    /// A subfolder's book, outside the weekly series. ⏎ enters it as the space.
    Collection(usize),
    /// Inside a collection: its book (all its published poems).
    Book,
    /// Inside a collection: back to the weekly series.
    Series,
    /// The space's poems grouped by where they were published.
    Publications,
    /// Creates a collection folder from inside the app.
    NewCollection,
    Unsigned,
    Drafts,
    Stats,
    /// Authors, pen names and signature counts.
    Signatures,
    Quit,
}

pub enum Confirm {
    Force,
    Trash(PathBuf),
    RemoveAuthor(usize),
}

/// The steps of the author prompts on the İmzalar screen.
pub enum AuthorStep {
    /// A new author's real name (may be empty).
    Name,
    /// …then their first pen name (may be empty, but not both).
    FirstPen { name: String },
    /// Another pen name for an existing author.
    AddPen { author: usize },
}

pub enum Mode {
    Main,
    Writer(Box<Editor>),
    Search {
        query: String,
        hits: Vec<Hit>,
        sel: usize,
    },
    History {
        title: String,
        snaps: Vec<Snapshot>,
        sel: usize,
    },
    Stats,
    Log {
        scroll: usize,
    },
    #[cfg(feature = "preview")]
    Preview,
    Confirm(Confirm),
    WatcherMenu,
    /// The İmzalar screen; `sel` is the highlighted row.
    Signatures {
        sel: usize,
    },
    /// Typing an author's name or a pen name; `back` is the row to return to.
    AuthorPrompt {
        step: AuthorStep,
        input: String,
        back: usize,
    },
    /// Choosing which author an unclaimed signature belongs to.
    AssignPen {
        name: String,
        sel: usize,
        back: usize,
    },
    /// Typing a new name for the week at `idx`.
    /// Typing the name of a new collection.
    NewCollection {
        input: String,
    },
    Rename {
        idx: usize,
        input: String,
    },
}

/// Things the event loop has to do outside the app (they need the terminal).
pub enum Action {
    None,
    ExternalEditor(PathBuf),
}

#[derive(PartialEq)]
pub enum Watching {
    Service,
    Tui,
    Off,
}

pub struct App {
    pub cfg: Config,
    pub lib: Library,
    pub items: Vec<Item>,
    /// The space being worked in: None for the weekly series, or a collection's
    /// folder name. Lists, drafts, search, stats and new poems all follow it.
    pub space: Option<String>,
    pub left: usize,
    pub right: usize,
    pub focus_right: bool,
    pub mode: Mode,
    pub log_tail: Vec<String>,
    pub building: Arc<AtomicBool>,
    build_pending: bool,
    pub watching: Watching,
    tui_watch: Option<Watcher>,
    #[cfg(feature = "preview")]
    pub preview: Option<Preview>,
    last_refresh: Instant,
    last_service_check: Instant,
    pub flash: Option<(String, Instant)>,
    /// Set by the quit item or `q`; the app exits once no build is running.
    pub quitting: bool,
    pub quit: bool,
}

impl App {
    pub fn new(cfg: Config) -> Result<App> {
        let lib = build::library(&cfg, &cfg.poems_dir())?;
        let mut app = App {
            cfg,
            lib,
            items: vec![],
            space: None,
            left: 0,
            right: 0,
            focus_right: false,
            mode: Mode::Main,
            log_tail: vec![],
            building: Arc::new(AtomicBool::new(false)),
            build_pending: false,
            watching: Watching::Off,
            tui_watch: None,
            #[cfg(feature = "preview")]
            preview: None,
            last_refresh: Instant::now(),
            last_service_check: Instant::now() - Duration::from_secs(60),
            flash: None,
            quitting: false,
            quit: false,
        };
        app.refresh();
        app.check_service();
        Ok(app)
    }

    pub fn lang(&self) -> Lang {
        self.lib.lang
    }

    pub fn t(&self, key: &str) -> &'static str {
        self.lib.lang.t(key)
    }

    pub fn say(&mut self, msg: impl Into<String>) {
        self.flash = Some((msg.into(), Instant::now()));
    }

    // ---------------------------------------------------------------- data

    pub fn refresh(&mut self) {
        if let Ok(lib) = build::library(&self.cfg, &self.cfg.poems_dir()) {
            self.lib = lib;
        }
        // A collection that disappeared (renamed or deleted) drops back to the series.
        if self.space.is_some() && self.space_index().is_none() {
            self.space = None;
        }
        let mut items = vec![Item::NewPoem, Item::ContinueDraft];
        match self.space_index() {
            None => {
                for i in (0..self.lib.weeks.len()).rev() {
                    items.push(Item::Week(i));
                }
                items.push(Item::All);
                items.push(Item::Publications);
                for i in 0..self.lib.collections.len() {
                    items.push(Item::Collection(i));
                }
            }
            Some(cur) => {
                items.push(Item::Book);
                items.push(Item::Publications);
                items.push(Item::Series);
                for i in (0..self.lib.collections.len()).filter(|i| *i != cur) {
                    items.push(Item::Collection(i));
                }
            }
        }
        items.push(Item::NewCollection);
        items.push(Item::Unsigned);
        items.push(Item::Drafts);
        items.push(Item::Stats);
        items.push(Item::Signatures);
        items.push(Item::Quit);
        self.items = items;
        self.left = self.left.min(self.items.len() - 1);
        let n = self.right_poems().len();
        self.right = self.right.min(n.saturating_sub(1));
        self.log_tail = read_log_tail(200);
        self.last_refresh = Instant::now();
    }

    fn check_service(&mut self) {
        self.last_service_check = Instant::now();
        if install::service_active() {
            self.tui_watch = None;
            self.watching = Watching::Service;
        } else {
            if self.tui_watch.is_none() {
                self.tui_watch = watch::watch_dir(&self.cfg.poems_dir()).ok();
            }
            self.watching = if self.tui_watch.is_some() {
                Watching::Tui
            } else {
                Watching::Off
            };
        }
    }

    pub fn current_item(&self) -> Item {
        self.items[self.left]
    }

    /// Index of the current collection in `lib.scan.collections` / `lib.collections`.
    pub fn space_index(&self) -> Option<usize> {
        let name = self.space.as_ref()?;
        self.lib
            .scan
            .collections
            .iter()
            .position(|(n, _)| n == name)
    }

    /// "Contra Noir", or the collection's name.
    pub fn space_name(&self) -> String {
        self.space
            .clone()
            .unwrap_or_else(|| self.cfg.series.clone())
    }

    /// Every poem file of the current space, whatever its status.
    pub fn space_poems(&self) -> Vec<&Poem> {
        match self.space_index() {
            None => self.lib.scan.poems.iter().collect(),
            Some(i) => self.lib.scan.collections[i].1.iter().collect(),
        }
    }

    /// The published poems of the current space.
    pub fn space_published(&self) -> Vec<&Poem> {
        match self.space_index() {
            None => self.lib.weeks.iter().flat_map(|w| w.poems.iter()).collect(),
            Some(i) => self.lib.collections[i].poems.iter().collect(),
        }
    }

    fn enter_space(&mut self, space: Option<String>) {
        self.space = space;
        self.left = 0;
        self.right = 0;
        self.focus_right = false;
        self.refresh();
    }

    pub fn count(&self, item: Item) -> usize {
        match item {
            Item::Week(i) => self.lib.weeks[i].poems.len(),
            Item::All => self.lib.weeks.iter().map(|w| w.poems.len()).sum(),
            Item::Collection(i) => self.lib.scan.collections[i].1.len(),
            Item::Book | Item::Series | Item::Publications => self.right_poems_for(item).len(),
            Item::Unsigned => self.with_status(|s| *s == Status::Unsigned).len(),
            Item::Drafts | Item::ContinueDraft => self.with_status(|s| *s == Status::Draft).len(),
            _ => 0,
        }
    }

    /// Poems of the current space with a given status.
    fn with_status(&self, f: impl Fn(&Status) -> bool) -> Vec<&Poem> {
        self.space_poems()
            .into_iter()
            .filter(|p| f(&p.status(&self.lib.authors)))
            .collect()
    }

    /// The poems listed in the right panel for the selected item.
    pub fn right_poems(&self) -> Vec<&Poem> {
        self.right_poems_for(self.current_item())
    }

    fn right_poems_for(&self, item: Item) -> Vec<&Poem> {
        match item {
            Item::Book => self.space_published(),
            // Sorted journal → issue → title, which is the order the tree is drawn in.
            // The whole issue as an archive: tracked poems by other authors too.
            Item::Publications => {
                let mut v: Vec<&Poem> = self
                    .space_poems()
                    .into_iter()
                    .filter(|p| {
                        matches!(
                            p.status(&self.lib.authors),
                            Status::Published | Status::Foreign(_)
                        )
                    })
                    .filter(|p| {
                        p.signature
                            .as_ref()
                            .is_some_and(|s| s.publication.is_some())
                    })
                    .collect();
                v.sort_by_key(|p| {
                    let pubn = p.signature.as_ref().unwrap().publication().unwrap();
                    let issue = pubn.issue.clone().unwrap_or_default();
                    (
                        crate::volume::turkish_key(&pubn.name),
                        // Issue numbers sort as numbers (#2 before #10), others after.
                        issue.parse::<u64>().unwrap_or(u64::MAX),
                        issue,
                        pubn.edition.unwrap_or_default(),
                        crate::volume::turkish_key(&p.title),
                    )
                });
                v
            }
            Item::Series => self
                .lib
                .weeks
                .iter()
                .rev()
                .flat_map(|w| w.poems.iter())
                .collect(),
            Item::Week(i) => self.lib.weeks[i].poems.iter().collect(),
            Item::All => self
                .lib
                .weeks
                .iter()
                .rev()
                .flat_map(|w| w.poems.iter())
                .collect(),
            // Everything in the folder, drafts and unsigned poems included.
            Item::Collection(i) => {
                let mut v: Vec<&Poem> = self.lib.scan.collections[i].1.iter().collect();
                v.sort_by_key(|p| crate::volume::turkish_key(&p.title));
                v
            }
            Item::Unsigned => self.with_status(|s| *s == Status::Unsigned),
            Item::Drafts | Item::ContinueDraft | Item::NewPoem => {
                let mut v = self.with_status(|s| *s == Status::Draft);
                v.sort_by_key(|p| std::cmp::Reverse(mtime(&p.path)));
                v
            }
            Item::Stats | Item::Signatures | Item::Quit | Item::NewCollection => vec![],
        }
    }

    fn selected_poem(&self) -> Option<&Poem> {
        if !self.focus_right {
            return None;
        }
        self.right_poems().get(self.right).copied()
    }

    /// PDF for the selection: a week, the full book, or the week holding a poem.
    fn selected_pdf(&self) -> Option<(PathBuf, Option<String>)> {
        let out = self.cfg.out_dir();
        if let Some(p) = self.selected_poem() {
            let holds = |poems: &[Poem]| poems.iter().any(|q| q.file_name == p.file_name);
            let file = self
                .lib
                .weeks
                .iter()
                .find(|w| holds(&w.poems))
                .map(|w| &w.file_name)
                .or_else(|| {
                    self.lib
                        .collections
                        .iter()
                        .find(|c| holds(&c.poems))
                        .map(|c| &c.file_name)
                })?;
            return Some((out.join(file), Some(p.title.clone())));
        }
        match self.current_item() {
            Item::Week(i) => Some((out.join(&self.lib.weeks[i].file_name), None)),
            Item::All => Some((out.join(&self.lib.book), None)),
            Item::Collection(i) => Some((out.join(&self.lib.collections[i].file_name), None)),
            Item::Book => self
                .space_index()
                .map(|i| (out.join(&self.lib.collections[i].file_name), None)),
            Item::Series => Some((out.join(&self.lib.book), None)),
            _ => None,
        }
        .filter(|(p, _)| p.exists())
    }

    // ---------------------------------------------------------------- background work

    pub fn start_build(&mut self, force: bool) {
        if self.building.load(Ordering::SeqCst) {
            self.build_pending = true;
            return;
        }
        self.building.store(true, Ordering::SeqCst);
        let flag = self.building.clone();
        let cfg = self.cfg.clone();
        std::thread::spawn(move || {
            if let Err(e) = build::build(&cfg, None, force) {
                build::log(&format!("ERROR {e:#}"));
            }
            flag.store(false, Ordering::SeqCst);
        });
    }

    pub fn tick(&mut self) {
        if self.tui_watch.as_mut().is_some_and(|w| w.poll()) {
            self.start_build(false);
        }
        // Safe quit: let a running build finish so no PDF is left half-written.
        if self.quitting && !self.building.load(Ordering::SeqCst) {
            self.quit = true;
            return;
        }
        if self.build_pending && !self.building.load(Ordering::SeqCst) {
            self.build_pending = false;
            self.start_build(false);
        }
        if let Mode::Writer(ed) = &mut self.mode
            && ed.autosave_due()
        {
            let _ = ed.save(&self.cfg);
        }
        if self.last_refresh.elapsed() >= Duration::from_secs(1) {
            self.refresh();
        }
        if self.last_service_check.elapsed() >= Duration::from_secs(3) {
            self.check_service();
        }
        if self
            .flash
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed() > Duration::from_secs(4))
        {
            self.flash = None;
        }
    }

    // ---------------------------------------------------------------- actions

    /// A blank page, saved into the current space (or a highlighted collection).
    pub fn new_poem(&mut self) {
        let all: Vec<Poem> = self.lib.scan.all_poems().cloned().collect();
        let mut ed = Editor::new(writer::inspiration(&all));
        let target = match self.current_item() {
            Item::Collection(i) => Some(i),
            _ => self.space_index(),
        };
        if let Some(i) = target {
            ed.dir = Some(self.cfg.poems_dir().join(&self.lib.scan.collections[i].0));
        }
        self.mode = Mode::Writer(Box::new(ed));
    }

    fn create_collection(&mut self, input: &str) {
        if input.trim().is_empty() {
            return;
        }
        let name = writer::sanitize(input);
        let dir = self.cfg.poems_dir().join(&name);
        match fs::create_dir_all(&dir) {
            Ok(()) => {
                build::log(&format!("new collection: {name}"));
                // Step straight into the new collection.
                self.refresh();
                self.enter_space(Some(name));
            }
            Err(e) => self.say(format!("{e}")),
        }
    }

    fn open_poem(&mut self, path: PathBuf) {
        match Editor::open(&path) {
            Ok(e) => self.mode = Mode::Writer(Box::new(e)),
            Err(e) => self.say(format!("{e:#}")),
        }
    }

    fn open_pdf(&mut self) {
        match self.selected_pdf() {
            Some((p, _)) => {
                // xdg-open may hand PDFs to a non-PDF app (e.g. a notes app), so
                // the config can name the viewer.
                let viewer = self.cfg.pdf_viewer.trim();
                let viewer = if viewer.is_empty() {
                    "xdg-open"
                } else {
                    viewer
                };
                let spawned = Command::new(viewer)
                    .arg(p)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
                if let Err(e) = spawned {
                    self.say(format!("{viewer}: {e}"));
                }
            }
            None => self.say(self.t("no_pdf")),
        }
    }

    /// The text proof of the selection: a poem's page within its volume,
    /// a whole volume, the book, or a lone draft/unsigned poem.
    #[cfg(feature = "preview")]
    fn open_preview(&mut self) {
        let revisions = build::State::load(&self.cfg.out_dir()).revisions();
        let ctx = Ctx {
            template: "",
            series: &self.cfg.series,
            author: &self.cfg.author,
            real_name: &self.lib.authors.main_author().name,
            lang: self.lib.lang,
            revisions: &revisions,
        };
        let preview = if let Some(p) = self.selected_poem() {
            match self
                .lib
                .weeks
                .iter()
                .find(|w| w.poems.iter().any(|q| q.file_name == p.file_name))
            {
                Some(w) => Preview::week(w, &ctx, Some(&p.file_name)),
                None => Preview::single(p, &ctx),
            }
        } else {
            match self.current_item() {
                Item::Week(i) => Preview::week(&self.lib.weeks[i], &ctx, None),
                Item::All if !self.lib.weeks.is_empty() => Preview::book(&self.lib.weeks, &ctx),
                _ => match self.right_poems().first() {
                    Some(p) => Preview::single(p, &ctx),
                    None => {
                        self.say(self.t("no_pdf"));
                        return;
                    }
                },
            }
        };
        self.preview = Some(preview);
        self.mode = Mode::Preview;
    }

    // ---------------------------------------------------------------- input

    /// Search covers the current space only.
    fn search_pool(&self) -> Vec<Poem> {
        self.space_poems().into_iter().cloned().collect()
    }

    pub fn on_paste(&mut self, s: &str) {
        let pool = self.search_pool();
        match &mut self.mode {
            Mode::Writer(ed) => {
                ed.insert_str(s);
            }
            Mode::Rename { input, .. }
            | Mode::NewCollection { input }
            | Mode::AuthorPrompt { input, .. } => input.push_str(s.lines().next().unwrap_or("")),
            Mode::Search { query, hits, sel } => {
                query.push_str(s.lines().next().unwrap_or(""));
                *hits = search::search(&pool, query);
                *sel = 0;
            }
            _ => {}
        }
    }

    pub fn on_key(&mut self, k: KeyEvent) -> Action {
        let mode = std::mem::replace(&mut self.mode, Mode::Main);
        let (mode, action) = match mode {
            Mode::Main => {
                let a = self.key_main(k);
                // key_main may have switched the mode itself.
                (std::mem::replace(&mut self.mode, Mode::Main), a)
            }
            Mode::Writer(ed) => (self.key_writer(ed, k), Action::None),
            Mode::Search { query, hits, sel } => {
                (self.key_search(query, hits, sel, k), Action::None)
            }
            Mode::History { title, snaps, sel } => {
                let n = snaps.len();
                match k.code {
                    KeyCode::Esc | KeyCode::Char('q') => (Mode::Main, Action::None),
                    KeyCode::Up | KeyCode::Char('k') => (
                        Mode::History {
                            title,
                            snaps,
                            sel: sel.saturating_sub(1),
                        },
                        Action::None,
                    ),
                    KeyCode::Down | KeyCode::Char('j') => (
                        Mode::History {
                            title,
                            snaps,
                            sel: (sel + 1).min(n.saturating_sub(1)),
                        },
                        Action::None,
                    ),
                    _ => (Mode::History { title, snaps, sel }, Action::None),
                }
            }
            Mode::Stats => match k.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('s') => {
                    (Mode::Main, Action::None)
                }
                _ => (Mode::Stats, Action::None),
            },
            Mode::Log { scroll } => match k.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('l') => {
                    (Mode::Main, Action::None)
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    (Mode::Log { scroll: scroll + 1 }, Action::None)
                }
                KeyCode::Down | KeyCode::Char('j') => (
                    Mode::Log {
                        scroll: scroll.saturating_sub(1),
                    },
                    Action::None,
                ),
                KeyCode::PageUp => (
                    Mode::Log {
                        scroll: scroll + 20,
                    },
                    Action::None,
                ),
                KeyCode::PageDown => (
                    Mode::Log {
                        scroll: scroll.saturating_sub(20),
                    },
                    Action::None,
                ),
                _ => (Mode::Log { scroll }, Action::None),
            },
            #[cfg(feature = "preview")]
            Mode::Preview => {
                if let Some(p) = &mut self.preview {
                    match k.code {
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('p') => {
                            self.preview = None;
                            return Action::None;
                        }
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::PageUp => p.turn(-1),
                        KeyCode::Right
                        | KeyCode::Char('l')
                        | KeyCode::PageDown
                        | KeyCode::Char(' ') => p.turn(1),
                        KeyCode::Up | KeyCode::Char('k') => p.scroll = p.scroll.saturating_sub(1),
                        KeyCode::Down | KeyCode::Char('j') => p.scroll += 1,
                        KeyCode::Home => p.turn(-(p.pages.len() as i32)),
                        KeyCode::End => p.turn(p.pages.len() as i32),
                        _ => {}
                    }
                }
                (Mode::Preview, Action::None)
            }
            Mode::Signatures { sel } => (super::signatures::key(self, sel, k), Action::None),
            Mode::AuthorPrompt { step, input, back } => (
                super::signatures::key_prompt(self, step, input, back, k),
                Action::None,
            ),
            Mode::AssignPen { name, sel, back } => (
                super::signatures::key_assign(self, name, sel, back, k),
                Action::None,
            ),
            Mode::Confirm(c) => {
                if let KeyCode::Char(ch) = k.code
                    && self.lang().yes(ch)
                {
                    match c {
                        Confirm::Force => self.start_build(true),
                        Confirm::RemoveAuthor(i) => super::signatures::remove_author(self, i),
                        Confirm::Trash(p) => match trash::delete(&p) {
                            Ok(()) => {
                                build::log(&format!("moved to trash: {}", p.display()));
                                self.refresh();
                            }
                            Err(e) => self.say(format!("{e}")),
                        },
                    }
                }
                (Mode::Main, Action::None)
            }
            Mode::NewCollection { mut input } => match k.code {
                KeyCode::Esc => (Mode::Main, Action::None),
                KeyCode::Enter => {
                    self.create_collection(&input);
                    (Mode::Main, Action::None)
                }
                KeyCode::Backspace => {
                    input.pop();
                    (Mode::NewCollection { input }, Action::None)
                }
                KeyCode::Char(c)
                    if !k.modifiers.contains(KeyModifiers::CONTROL)
                        || k.modifiers.contains(KeyModifiers::ALT) =>
                {
                    input.push(c);
                    (Mode::NewCollection { input }, Action::None)
                }
                _ => (Mode::NewCollection { input }, Action::None),
            },
            Mode::Rename { idx, mut input } => match k.code {
                KeyCode::Esc => (Mode::Main, Action::None),
                KeyCode::Enter => {
                    if let Some(w) = self.lib.weeks.get(idx) {
                        match crate::config::set_week_name(&w.key, &input) {
                            Ok(()) => {
                                build::log(&format!(
                                    "renamed {} to \"{}\"",
                                    w.file_name,
                                    input.trim()
                                ));
                                self.refresh();
                                self.start_build(false);
                            }
                            Err(e) => self.say(format!("{e:#}")),
                        }
                    }
                    (Mode::Main, Action::None)
                }
                KeyCode::Backspace => {
                    input.pop();
                    (Mode::Rename { idx, input }, Action::None)
                }
                KeyCode::Char(c)
                    if !k.modifiers.contains(KeyModifiers::CONTROL)
                        || k.modifiers.contains(KeyModifiers::ALT) =>
                {
                    input.push(c);
                    (Mode::Rename { idx, input }, Action::None)
                }
                _ => (Mode::Rename { idx, input }, Action::None),
            },
            Mode::WatcherMenu => {
                let r = match k.code {
                    KeyCode::Char('1') => install::service("start").map(|_| ()),
                    KeyCode::Char('2') => install::service("stop").map(|_| ()),
                    KeyCode::Char('3') => install::service("restart").map(|_| ()),
                    KeyCode::Char('4') => install::install().map(|_| ()),
                    _ => Ok(()),
                };
                if let Err(e) = r {
                    self.say(format!("{e:#}"));
                }
                self.check_service();
                (Mode::Main, Action::None)
            }
        };
        self.mode = mode;
        action
    }

    fn key_main(&mut self, k: KeyEvent) -> Action {
        let n_right = self.right_poems().len();
        match k.code {
            KeyCode::Char('q') => self.quitting = true,
            KeyCode::Char('r') => {
                if let Item::Week(i) = self.current_item() {
                    let w = &self.lib.weeks[i];
                    let input = w.label(self.lang());
                    self.mode = Mode::Rename { idx: i, input };
                }
            }
            KeyCode::Char('n') => self.new_poem(),
            KeyCode::Up | KeyCode::Char('k') => {
                if self.focus_right {
                    self.right = self.right.saturating_sub(1);
                } else {
                    self.left = self.left.saturating_sub(1);
                    self.right = 0;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.focus_right {
                    self.right = (self.right + 1).min(n_right.saturating_sub(1));
                } else {
                    self.left = (self.left + 1).min(self.items.len() - 1);
                    self.right = 0;
                }
            }
            KeyCode::Right | KeyCode::Tab if !self.focus_right && n_right > 0 => {
                self.focus_right = true
            }
            KeyCode::Left | KeyCode::BackTab | KeyCode::Esc => self.focus_right = false,
            KeyCode::Enter => {
                if let Some(p) = self.selected_poem() {
                    let path = p.path.clone();
                    self.open_poem(path);
                } else {
                    match self.current_item() {
                        Item::NewPoem => self.new_poem(),
                        Item::ContinueDraft => {
                            match self.right_poems().first().map(|p| p.path.clone()) {
                                Some(p) => self.open_poem(p),
                                None => self.new_poem(),
                            }
                        }
                        Item::Stats => self.mode = Mode::Stats,
                        Item::Signatures => self.mode = Mode::Signatures { sel: 0 },
                        Item::Quit => self.quitting = true,
                        Item::NewCollection => {
                            self.mode = Mode::NewCollection {
                                input: String::new(),
                            }
                        }
                        // ⏎ on a collection enters it; on Contra Noir goes back.
                        Item::Collection(i) => {
                            let name = self.lib.scan.collections[i].0.clone();
                            self.enter_space(Some(name));
                        }
                        Item::Series => self.enter_space(None),
                        _ if n_right > 0 => self.focus_right = true,
                        _ => {}
                    }
                }
            }
            KeyCode::Char('e') => {
                if let Some(p) = self.selected_poem() {
                    return Action::ExternalEditor(p.path.clone());
                }
            }
            KeyCode::Char('/') => {
                self.mode = Mode::Search {
                    query: String::new(),
                    hits: vec![],
                    sel: 0,
                }
            }
            KeyCode::Char('h') => {
                if let Some(p) = self.selected_poem() {
                    let snaps = history::list(&p.file_name);
                    let sel = snaps.len().saturating_sub(1);
                    self.mode = Mode::History {
                        title: p.title.clone(),
                        snaps,
                        sel,
                    };
                }
            }
            KeyCode::Char('o') => self.open_pdf(),
            #[cfg(feature = "preview")]
            KeyCode::Char('p') => self.open_preview(),
            KeyCode::Char('b') => self.start_build(false),
            KeyCode::Char('f') => self.mode = Mode::Confirm(Confirm::Force),
            KeyCode::Char('D') => {
                if let Some(p) = self.selected_poem() {
                    self.mode = Mode::Confirm(Confirm::Trash(p.path.clone()));
                }
            }
            KeyCode::Char('s') => self.mode = Mode::Stats,
            KeyCode::Char('w') => self.mode = Mode::WatcherMenu,
            KeyCode::Char('l') => self.mode = Mode::Log { scroll: 0 },
            _ => {}
        }
        Action::None
    }

    fn key_writer(&mut self, mut ed: Box<Editor>, k: KeyEvent) -> Mode {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let alt = k.modifiers.contains(KeyModifiers::ALT);
        if ed.confirm_sign {
            ed.confirm_sign = false;
            if let KeyCode::Char(c) = k.code
                && self.lang().yes(c)
            {
                match ed.sign(&self.cfg) {
                    Ok(()) => ed.message = Some(self.t("signed").into()),
                    Err(e) => ed.message = Some(format!("{e:#}")),
                }
            }
            return Mode::Writer(ed);
        }
        match k.code {
            KeyCode::Esc => {
                if ed.dirty
                    && let Err(e) = ed.save(&self.cfg)
                {
                    self.say(format!("{e:#}"));
                }
                self.refresh();
                return Mode::Main;
            }
            KeyCode::Char('s') if ctrl && !alt => {
                if let Err(e) = ed.save(&self.cfg) {
                    ed.message = Some(format!("{e:#}"));
                }
            }
            KeyCode::Char('g') if ctrl && !alt => {
                if !ed.is_signed() && !ed.is_empty() {
                    ed.confirm_sign = true;
                }
            }
            KeyCode::Char('t') if ctrl && !alt => ed.typewriter = !ed.typewriter,
            KeyCode::Char('z') if ctrl && !alt => ed.undo(),
            // AltGr arrives as Ctrl+Alt on some terminals; treat it as text.
            KeyCode::Char(c) if !ctrl || alt => ed.insert_char(c),
            KeyCode::Enter => ed.enter(),
            KeyCode::Backspace => ed.backspace(),
            KeyCode::Delete => ed.delete(),
            KeyCode::Tab => ed.insert_str("    "),
            KeyCode::Left => ed.left(),
            KeyCode::Right => ed.right(),
            KeyCode::Up => ed.up(1),
            KeyCode::Down => ed.down(1),
            KeyCode::PageUp => ed.up(20),
            KeyCode::PageDown => ed.down(20),
            KeyCode::Home => ed.home(),
            KeyCode::End => ed.end(),
            _ => {}
        }
        Mode::Writer(ed)
    }

    fn key_search(
        &mut self,
        mut query: String,
        mut hits: Vec<Hit>,
        mut sel: usize,
        k: KeyEvent,
    ) -> Mode {
        match k.code {
            KeyCode::Esc => return Mode::Main,
            KeyCode::Enter => {
                if let Some(h) = hits.get(sel) {
                    let path = self.cfg.poems_dir().join(&h.file_name);
                    self.open_poem(path);
                    return std::mem::replace(&mut self.mode, Mode::Main);
                }
            }
            KeyCode::Up => sel = sel.saturating_sub(1),
            KeyCode::Down => sel = (sel + 1).min(hits.len().saturating_sub(1)),
            KeyCode::Backspace => {
                query.pop();
                hits = search::search(&self.search_pool(), &query);
                sel = 0;
            }
            KeyCode::Char(c)
                if !k.modifiers.contains(KeyModifiers::CONTROL)
                    || k.modifiers.contains(KeyModifiers::ALT) =>
            {
                query.push(c);
                hits = search::search(&self.search_pool(), &query);
                sel = 0;
            }
            _ => {}
        }
        Mode::Search { query, hits, sel }
    }
}

fn mtime(p: &std::path::Path) -> SystemTime {
    fs::metadata(p)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

fn read_log_tail(n: usize) -> Vec<String> {
    let text = fs::read_to_string(log_path()).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..]
        .iter()
        .map(|s| s.to_string())
        .collect()
}
