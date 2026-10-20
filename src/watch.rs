use crate::build::{self, log};
use crate::config::Config;
use crate::sync;
use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};
use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

/// Changes are acted on once the folder has been quiet this long,
/// so an editor's burst of writes becomes one rebuild.
const QUIET: Duration = Duration::from_secs(2);

pub enum Wait {
    Changed,
    Quiet,
    Stopped,
}

/// How often the service checks the remote for poems written elsewhere.
const PULL_EVERY: Duration = Duration::from_secs(60);

pub struct Watcher {
    _inner: RecommendedWatcher,
    rx: Receiver<()>,
    pending: Option<Instant>,
}

/// Only real content changes count. Reads (our own builds open every poem),
/// hidden editor files and anything in a hidden folder (`.git`, `.poet`) are
/// ignored.
fn relevant(e: &Event, root: &Path) -> bool {
    let kind = matches!(
        e.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    );
    let visible = e.paths.iter().any(|p| {
        p.strip_prefix(root)
            .unwrap_or(p)
            .components()
            .all(|c| !c.as_os_str().to_string_lossy().starts_with('.'))
    });
    kind && visible
}

pub fn watch_dir(dir: &Path) -> Result<Watcher> {
    let (tx, rx) = channel();
    let root = dir.to_path_buf();
    let mut inner = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if let Ok(e) = res
            && relevant(&e, &root)
        {
            let _ = tx.send(());
        }
    })?;
    inner.watch(dir, RecursiveMode::Recursive)?;
    Ok(Watcher {
        _inner: inner,
        rx,
        pending: None,
    })
}

impl Watcher {
    fn settled(&mut self) -> bool {
        if self.pending.is_some_and(|t| t.elapsed() >= QUIET) {
            self.pending = None;
            return true;
        }
        false
    }

    /// Non-blocking: true once a burst of changes has settled.
    pub fn poll(&mut self) -> bool {
        while self.rx.try_recv().is_ok() {
            self.pending = Some(Instant::now());
        }
        self.settled()
    }

    /// Blocks until a burst of changes has settled, or `limit` passes quietly.
    pub fn wait(&mut self, limit: Duration) -> Wait {
        let start = Instant::now();
        loop {
            match self.rx.recv_timeout(Duration::from_millis(250)) {
                Ok(()) => self.pending = Some(Instant::now()),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return Wait::Stopped,
            }
            if self.settled() {
                return Wait::Changed;
            }
            if self.pending.is_none() && start.elapsed() >= limit {
                return Wait::Quiet;
            }
        }
    }
}

fn run_build(cfg: &Config) {
    match build::build(cfg, None, false) {
        Ok(r) => {
            for l in r.summary() {
                println!("{l}");
            }
        }
        Err(e) => {
            let msg = format!("ERROR {e:#}");
            log(&msg);
            eprintln!("{msg}");
        }
    }
}

/// Commits and exchanges poems with the remote; builds what arrived; pushes PDFs.
fn run_sync(cfg: &Config) {
    if !sync::enabled(cfg) {
        return;
    }
    match sync::sync(&cfg.poems_dir()) {
        Ok(out) => {
            for c in &out.conflicts {
                log(&format!(
                    "sync: {c} changed on both sides, kept \"{c} ({})\"",
                    sync::SIDE
                ));
            }
            if out.pulled {
                log("sync: new poems from the remote");
                run_build(cfg);
            }
        }
        Err(e) => log(&format!("ERROR sync: {e:#}")),
    }
    if let Err(e) = sync::push_pdfs(cfg) {
        log(&format!("ERROR sync pdf: {e:#}"));
    }
}

/// The background service: one build at start, then one per burst of changes,
/// and a sync with the remote after each and every minute.
pub fn run() -> Result<()> {
    let cfg = Config::load()?;
    let dir = cfg.poems_dir();
    log(&format!("watching {}", dir.display()));
    run_build(&cfg);
    run_sync(&cfg);
    let mut w = watch_dir(&dir)?;
    loop {
        let event = w.wait(PULL_EVERY);
        // Re-read the config so edits apply without a restart.
        let cfg = Config::load().unwrap_or(cfg.clone());
        match event {
            Wait::Changed => {
                run_build(&cfg);
                run_sync(&cfg);
            }
            Wait::Quiet => run_sync(&cfg),
            Wait::Stopped => return Ok(()),
        }
    }
}
