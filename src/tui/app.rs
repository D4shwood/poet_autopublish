use super::preview::Preview;
use super::writer::{self, Editor};
use crate::build::{self, Library};
use crate::config::{Config, log_path};
use crate::history::{self, Snapshot};
use crate::install;
use crate::lang::Lang;
use crate::poem::{Poem, Status};
use crate::search::{self, Hit};
use crate::watch::{self, Watcher};
use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_image::picker::Picker;
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
    Unsigned,
    Drafts,
    Stats,
}

pub enum Confirm {
    Force,
    Trash(PathBuf),
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
    Preview,
    Confirm(Confirm),
    WatcherMenu,
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
    pub left: usize,
    pub right: usize,
    pub focus_right: bool,
    pub mode: Mode,
    pub log_tail: Vec<String>,
    pub building: Arc<AtomicBool>,
    build_pending: bool,
    pub watching: Watching,
    tui_watch: Option<Watcher>,
    pub picker: Option<Picker>,
    pub preview: Option<Preview>,
    last_refresh: Instant,
    last_service_check: Instant,
    pub flash: Option<(String, Instant)>,
    pub quit: bool,
}

impl App {
    pub fn new(cfg: Config, picker: Option<Picker>) -> Result<App> {
        let lib = build::library(&cfg, &cfg.poems_dir())?;
        let mut app = App {
            cfg,
            lib,
            items: vec![],
            left: 0,
            right: 0,
            focus_right: false,
            mode: Mode::Main,
            log_tail: vec![],
            building: Arc::new(AtomicBool::new(false)),
            build_pending: false,
            watching: Watching::Off,
            tui_watch: None,
            picker,
            preview: None,
            last_refresh: Instant::now(),
            last_service_check: Instant::now() - Duration::from_secs(60),
            flash: None,
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

    fn say(&mut self, msg: impl Into<String>) {
        self.flash = Some((msg.into(), Instant::now()));
    }

    // ---------------------------------------------------------------- data

    pub fn refresh(&mut self) {
        if let Ok(lib) = build::library(&self.cfg, &self.cfg.poems_dir()) {
            self.lib = lib;
        }
        let mut items = vec![Item::NewPoem, Item::ContinueDraft];
        for i in (0..self.lib.weeks.len()).rev() {
            items.push(Item::Week(i));
        }
        items.push(Item::All);
        items.push(Item::Unsigned);
        items.push(Item::Drafts);
        items.push(Item::Stats);
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

    pub fn count(&self, item: Item) -> usize {
        match item {
            Item::Week(i) => self.lib.weeks[i].poems.len(),
            Item::All => self.lib.weeks.iter().map(|w| w.poems.len()).sum(),
            Item::Unsigned => self
                .with_status(|s| matches!(s, Status::Unsigned | Status::Foreign(_)))
                .len(),
            Item::Drafts | Item::ContinueDraft => self.with_status(|s| *s == Status::Draft).len(),
            _ => 0,
        }
    }

    fn with_status(&self, f: impl Fn(&Status) -> bool) -> Vec<&Poem> {
        self.lib
            .scan
            .poems
            .iter()
            .filter(|p| f(&p.status(&self.cfg.author)))
            .collect()
    }

    /// The poems listed in the right panel for the selected item.
    pub fn right_poems(&self) -> Vec<&Poem> {
        match self.current_item() {
            Item::Week(i) => self.lib.weeks[i].poems.iter().collect(),
            Item::All => self
                .lib
                .weeks
                .iter()
                .rev()
                .flat_map(|w| w.poems.iter())
                .collect(),
            Item::Unsigned => {
                self.with_status(|s| matches!(s, Status::Unsigned | Status::Foreign(_)))
            }
            Item::Drafts | Item::ContinueDraft | Item::NewPoem => {
                let mut v = self.with_status(|s| *s == Status::Draft);
                v.sort_by_key(|p| std::cmp::Reverse(mtime(&p.path)));
                v
            }
            Item::Stats => vec![],
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
            let w = self
                .lib
                .weeks
                .iter()
                .find(|w| w.poems.iter().any(|q| q.file_name == p.file_name))?;
            return Some((out.join(&w.file_name), Some(p.title.clone())));
        }
        match self.current_item() {
            Item::Week(i) => Some((out.join(&self.lib.weeks[i].file_name), None)),
            Item::All => Some((out.join(&self.lib.book), None)),
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

    pub fn new_poem(&mut self) {
        let insp = writer::inspiration(&self.lib.scan.poems);
        self.mode = Mode::Writer(Box::new(Editor::new(insp)));
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
                let _ = Command::new("xdg-open")
                    .arg(p)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
            }
            None => self.say(self.t("no_pdf")),
        }
    }

    fn open_preview(&mut self) {
        let Some((pdf, title)) = self.selected_pdf() else {
            self.say(self.t("no_pdf"));
            return;
        };
        let Some(picker) = &self.picker else {
            let _ = Command::new("xdg-open")
                .arg(&pdf)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            self.say(self.t("preview_fallback"));
            return;
        };
        match Preview::open(picker, &pdf, title.as_deref()) {
            Ok(p) => {
                self.preview = Some(p);
                self.mode = Mode::Preview;
            }
            Err(e) => self.say(format!("{e:#}")),
        }
    }

    // ---------------------------------------------------------------- input

    pub fn on_paste(&mut self, s: &str) {
        match &mut self.mode {
            Mode::Writer(ed) => {
                ed.insert_str(s);
            }
            Mode::Search { query, hits, sel } => {
                query.push_str(s.lines().next().unwrap_or(""));
                *hits = search::search(&self.lib.scan.poems, query);
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
            Mode::Preview => match k.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('p') => {
                    self.preview = None;
                    (Mode::Main, Action::None)
                }
                KeyCode::Left | KeyCode::Char('h') | KeyCode::PageUp => {
                    if let (Some(p), Some(picker)) = (&mut self.preview, &self.picker) {
                        let _ = p.turn(picker, -1);
                    }
                    (Mode::Preview, Action::None)
                }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::PageDown => {
                    if let (Some(p), Some(picker)) = (&mut self.preview, &self.picker) {
                        let _ = p.turn(picker, 1);
                    }
                    (Mode::Preview, Action::None)
                }
                _ => (Mode::Preview, Action::None),
            },
            Mode::Confirm(c) => {
                if let KeyCode::Char(ch) = k.code
                    && self.lang().yes(ch)
                {
                    match c {
                        Confirm::Force => self.start_build(true),
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
            KeyCode::Char('q') => self.quit = true,
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
                hits = search::search(&self.lib.scan.poems, &query);
                sel = 0;
            }
            KeyCode::Char(c)
                if !k.modifiers.contains(KeyModifiers::CONTROL)
                    || k.modifiers.contains(KeyModifiers::ALT) =>
            {
                query.push(c);
                hits = search::search(&self.lib.scan.poems, &query);
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
