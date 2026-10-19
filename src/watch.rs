use crate::build::{self, log};
use crate::config::Config;
use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};
use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

/// Changes are acted on once the folder has been quiet this long,
/// so an editor's burst of writes becomes one rebuild.
const QUIET: Duration = Duration::from_secs(2);

pub struct Watcher {
    _inner: RecommendedWatcher,
    rx: Receiver<()>,
    pending: Option<Instant>,
}

/// Only real content changes count. Reads (our own builds open every poem)
/// and hidden editor files are ignored.
fn relevant(e: &Event) -> bool {
    let kind = matches!(
        e.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    );
    let visible = e.paths.iter().any(|p| {
        p.file_name()
            .map(|n| !n.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
    });
    kind && visible
}

pub fn watch_dir(dir: &Path) -> Result<Watcher> {
    let (tx, rx) = channel();
    let mut inner = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if let Ok(e) = res
            && relevant(&e)
        {
            let _ = tx.send(());
        }
    })?;
    inner.watch(dir, RecursiveMode::NonRecursive)?;
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

    /// Blocks until a burst of changes has settled; false if the watcher stopped.
    pub fn wait(&mut self) -> bool {
        loop {
            match self.rx.recv_timeout(Duration::from_millis(250)) {
                Ok(()) => self.pending = Some(Instant::now()),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return false,
            }
            if self.settled() {
                return true;
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

/// The background service: one build at start, then one per burst of changes.
pub fn run() -> Result<()> {
    let cfg = Config::load()?;
    let dir = cfg.poems_dir();
    log(&format!("watching {}", dir.display()));
    run_build(&cfg);
    let mut w = watch_dir(&dir)?;
    while w.wait() {
        // Re-read the config so edits apply without a restart.
        let cfg = Config::load().unwrap_or(cfg.clone());
        run_build(&cfg);
    }
    Ok(())
}
