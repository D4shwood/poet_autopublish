//! Handing files to the desktop: the default app for a file, and the editor.

use std::path::Path;
use std::process::Command;

/// The desktop's "open with the default app": xdg-open, or Explorer on Windows.
pub fn default_app(path: &Path) -> Command {
    let mut c = Command::new(if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    });
    c.arg(path);
    c
}

/// $VISUAL, then $EDITOR, then the platform's own (the default app, or Notepad).
pub fn editor(path: &Path) -> Command {
    match std::env::var("VISUAL").or_else(|_| std::env::var("EDITOR")) {
        Ok(e) => {
            let mut c = Command::new(e);
            c.arg(path);
            c
        }
        Err(_) if cfg!(windows) => {
            let mut c = Command::new("notepad");
            c.arg(path);
            c
        }
        Err(_) => default_app(path),
    }
}
