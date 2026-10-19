//! The distraction-free writing screen.

use crate::config::Config;
use crate::history;
use crate::poem::{DRAFT_TAGS, Poem, Signature};
use anyhow::Result;
use chrono::{DateTime, Local};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthChar;

/// (logical row, start char, text, is a soft-wrapped continuation)
pub type VisualRow = (usize, usize, String, bool);

pub const AUTOSAVE: Duration = Duration::from_secs(30);
const UNDO_DEPTH: usize = 200;

pub struct Editor {
    pub lines: Vec<String>,
    /// Cursor: logical line and char index within it.
    pub row: usize,
    pub col: usize,
    pub path: Option<PathBuf>,
    pub is_draft: bool,
    pub dirty: bool,
    pub last_saved: Option<DateTime<Local>>,
    last_save_at: Instant,
    pub started: Instant,
    pub typewriter: bool,
    /// A faint line from an old poem, shown until the first keystroke.
    pub inspiration: Option<String>,
    pub confirm_sign: bool,
    pub message: Option<String>,
    undo: Vec<(Vec<String>, usize, usize)>,
    /// First visible visual row (normal scrolling).
    pub scroll: usize,
}

impl Editor {
    pub fn new(inspiration: Option<String>) -> Editor {
        Editor {
            lines: vec![String::new()],
            row: 0,
            col: 0,
            path: None,
            is_draft: true,
            dirty: false,
            last_saved: None,
            last_save_at: Instant::now(),
            started: Instant::now(),
            typewriter: false,
            inspiration,
            confirm_sign: false,
            message: None,
            undo: vec![],
            scroll: 0,
        }
    }

    /// Opens an existing poem. The draft tag is hidden while editing and written back on save.
    pub fn open(path: &Path) -> Result<Editor> {
        let raw = fs::read_to_string(path)?.replace("\r\n", "\n");
        let mut lines: Vec<String> = raw.lines().map(str::to_string).collect();
        let mut is_draft = false;
        if let Some(i) = lines
            .iter()
            .skip(1)
            .position(|l| !l.trim().is_empty())
            .map(|i| i + 1)
            && DRAFT_TAGS.contains(&lines[i].trim().to_lowercase().as_str())
        {
            is_draft = true;
            lines.remove(i);
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        let mut e = Editor::new(None);
        e.row = lines.len() - 1;
        e.col = lines[e.row].chars().count();
        e.lines = lines;
        e.path = Some(path.to_path_buf());
        e.is_draft = is_draft;
        Ok(e)
    }

    pub fn title(&self) -> &str {
        self.lines.first().map(|l| l.trim()).unwrap_or("")
    }

    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|l| l.trim().is_empty())
    }

    pub fn is_signed(&self) -> bool {
        self.lines
            .iter()
            .rposition(|l| !l.trim().is_empty())
            .is_some_and(|i| i > 0 && Signature::parse(&self.lines[i]).is_some())
    }

    pub fn word_count(&self) -> usize {
        self.lines
            .iter()
            .skip(1)
            .map(|l| l.split_whitespace().count())
            .sum()
    }

    pub fn line_count(&self) -> usize {
        self.lines
            .iter()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .count()
    }

    fn push_undo(&mut self) {
        self.undo.push((self.lines.clone(), self.row, self.col));
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if let Some((lines, row, col)) = self.undo.pop() {
            self.lines = lines;
            self.row = row;
            self.col = col;
            self.dirty = true;
        }
    }

    fn byte_idx(line: &str, col: usize) -> usize {
        line.char_indices()
            .nth(col)
            .map(|(i, _)| i)
            .unwrap_or(line.len())
    }

    fn line_len(&self, row: usize) -> usize {
        self.lines[row].chars().count()
    }

    fn edited(&mut self) {
        self.dirty = true;
        self.inspiration = None;
        self.message = None;
    }

    pub fn insert_char(&mut self, c: char) {
        // Snapshot at word boundaries so undo isn't one character at a time.
        if c == ' ' || self.undo.is_empty() {
            self.push_undo();
        }
        let line = &mut self.lines[self.row];
        let i = Self::byte_idx(line, self.col);
        line.insert(i, c);
        self.col += 1;
        self.edited();
    }

    pub fn insert_str(&mut self, s: &str) {
        self.push_undo();
        for (n, part) in s.replace("\r\n", "\n").split('\n').enumerate() {
            if n > 0 {
                self.split_line();
            }
            for c in part.chars() {
                let line = &mut self.lines[self.row];
                let i = Self::byte_idx(line, self.col);
                line.insert(i, c);
                self.col += 1;
            }
        }
        self.edited();
    }

    fn split_line(&mut self) {
        let line = &mut self.lines[self.row];
        let i = Self::byte_idx(line, self.col);
        let rest = line.split_off(i);
        self.lines.insert(self.row + 1, rest);
        self.row += 1;
        self.col = 0;
    }

    pub fn enter(&mut self) {
        self.push_undo();
        self.split_line();
        self.edited();
    }

    pub fn backspace(&mut self) {
        if self.col > 0 {
            self.push_undo();
            let line = &mut self.lines[self.row];
            let i = Self::byte_idx(line, self.col - 1);
            line.remove(i);
            self.col -= 1;
            self.edited();
        } else if self.row > 0 {
            self.push_undo();
            let cur = self.lines.remove(self.row);
            self.row -= 1;
            self.col = self.line_len(self.row);
            self.lines[self.row].push_str(&cur);
            self.edited();
        }
    }

    pub fn delete(&mut self) {
        if self.col < self.line_len(self.row) {
            self.push_undo();
            let line = &mut self.lines[self.row];
            let i = Self::byte_idx(line, self.col);
            line.remove(i);
            self.edited();
        } else if self.row + 1 < self.lines.len() {
            self.push_undo();
            let next = self.lines.remove(self.row + 1);
            self.lines[self.row].push_str(&next);
            self.edited();
        }
    }

    pub fn left(&mut self) {
        if self.col > 0 {
            self.col -= 1;
        } else if self.row > 0 {
            self.row -= 1;
            self.col = self.line_len(self.row);
        }
    }

    pub fn right(&mut self) {
        if self.col < self.line_len(self.row) {
            self.col += 1;
        } else if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = 0;
        }
    }

    pub fn up(&mut self, n: usize) {
        self.row = self.row.saturating_sub(n);
        self.col = self.col.min(self.line_len(self.row));
    }

    pub fn down(&mut self, n: usize) {
        self.row = (self.row + n).min(self.lines.len() - 1);
        self.col = self.col.min(self.line_len(self.row));
    }

    pub fn home(&mut self) {
        self.col = 0;
    }

    pub fn end(&mut self) {
        self.col = self.line_len(self.row);
    }

    /// The file text: the draft tag goes right after the title.
    pub fn content(&self) -> String {
        let mut lines = self.lines.clone();
        while lines.last().is_some_and(|l| l.trim().is_empty()) {
            lines.pop();
        }
        if self.is_draft {
            lines.insert(1.min(lines.len()), "#taslak".into());
        }
        let mut s = lines.join("\n");
        s.push('\n');
        s
    }

    /// Saves the poem. A draft's file follows its title; once signed the name stays fixed.
    pub fn save(&mut self, cfg: &Config) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }
        let dir = cfg.poems_dir();
        fs::create_dir_all(&dir)?;
        let wanted = sanitize(self.title());
        let path = match &self.path {
            None => unique(&dir, &wanted, None),
            Some(p)
                if self.is_draft
                    && p.file_name()
                        .map(|n| n.to_string_lossy() != wanted)
                        .unwrap_or(true) =>
            {
                let new = unique(&dir, &wanted, Some(p));
                if new != *p {
                    let old_name = p.file_name().unwrap().to_string_lossy().to_string();
                    let new_name = new.file_name().unwrap().to_string_lossy().to_string();
                    if p.exists() {
                        fs::rename(p, &new)?;
                    }
                    history::rename(&old_name, &new_name);
                }
                new
            }
            Some(p) => p.clone(),
        };
        let text = self.content();
        let tmp = dir.join(format!(
            ".{}.tmp",
            path.file_name().unwrap().to_string_lossy()
        ));
        fs::write(&tmp, &text)?;
        fs::rename(&tmp, &path)?;
        let _ = history::snapshot(
            &path.file_name().unwrap().to_string_lossy(),
            &text,
            cfg.history_keep,
        );
        self.path = Some(path);
        self.dirty = false;
        self.last_saved = Some(Local::now());
        self.last_save_at = Instant::now();
        Ok(())
    }

    pub fn autosave_due(&self) -> bool {
        self.dirty && self.last_save_at.elapsed() >= AUTOSAVE
    }

    /// Removes the draft tag and appends the signature with the current time.
    pub fn sign(&mut self, cfg: &Config) -> Result<()> {
        self.push_undo();
        while self.lines.last().is_some_and(|l| l.trim().is_empty()) {
            self.lines.pop();
        }
        self.lines.push(String::new());
        self.lines
            .push(Signature::format(&cfg.author, Local::now().naive_local()));
        self.is_draft = false;
        self.row = self.lines.len() - 1;
        self.col = self.line_len(self.row);
        self.dirty = true;
        self.save(cfg)
    }

    /// Wraps the text into visual rows of at most `width` columns.
    /// Returns (logical row, start char, text, is_continuation) per visual row,
    /// and the cursor's visual (row, column).
    pub fn layout(&self, width: usize) -> (Vec<VisualRow>, (usize, usize)) {
        let width = width.max(8);
        let mut rows = vec![];
        let mut cursor = (0, 0);
        for (li, line) in self.lines.iter().enumerate() {
            let chunks = wrap(line, width);
            for (ci, (start, text)) in chunks.iter().enumerate() {
                let len = text.chars().count();
                let last = ci + 1 == chunks.len();
                if li == self.row && self.col >= *start && (self.col < start + len || last) {
                    let x: usize = text
                        .chars()
                        .take(self.col - start)
                        .map(|c| c.width().unwrap_or(0))
                        .sum();
                    cursor = (rows.len(), x);
                }
                rows.push((li, *start, text.clone(), ci > 0));
            }
        }
        (rows, cursor)
    }
}

/// Soft wrap at spaces where possible; returns (start char index, text) chunks.
pub fn wrap(line: &str, width: usize) -> Vec<(usize, String)> {
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return vec![(0, String::new())];
    }
    let mut out = vec![];
    let mut start = 0;
    while start < chars.len() {
        let mut w = 0;
        let mut end = start;
        while end < chars.len() && w + chars[end].width().unwrap_or(0) <= width {
            w += chars[end].width().unwrap_or(0);
            end += 1;
        }
        if end < chars.len() {
            if let Some(sp) = (start + 1..end).rev().find(|&i| chars[i] == ' ') {
                end = sp + 1;
            }
            if end == start {
                end = start + 1;
            }
        }
        out.push((start, chars[start..end].iter().collect()));
        start = end;
    }
    out
}

/// A file name from a title: `/` replaced, Turkish letters kept.
pub fn sanitize(title: &str) -> String {
    let s: String = title
        .trim()
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect();
    let s = s.trim_start_matches(['.', '_']).trim().to_string();
    if s.is_empty() {
        "Adsız".into()
    } else {
        s.chars().take(80).collect()
    }
}

fn unique(dir: &Path, name: &str, current: Option<&PathBuf>) -> PathBuf {
    let mut candidate = dir.join(name);
    let mut n = 2;
    while candidate.exists() && Some(&candidate) != current {
        candidate = dir.join(format!("{name} ({n})"));
        n += 1;
    }
    candidate
}

/// A random line from the user's own poems, for the blank page.
pub fn inspiration(poems: &[Poem]) -> Option<String> {
    use rand::seq::IndexedRandom;
    let lines: Vec<&String> = poems
        .iter()
        .flat_map(|p| p.body.iter())
        .filter(|l| {
            let n = l.trim().chars().count();
            (12..=80).contains(&n)
        })
        .collect();
    lines.choose(&mut rand::rng()).map(|l| l.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping() {
        assert_eq!(
            wrap("abc def ghi", 7),
            vec![(0, "abc ".into()), (4, "def ghi".into())]
        );
        assert_eq!(
            wrap("abcdefghij", 4),
            vec![(0, "abcd".into()), (4, "efgh".into()), (8, "ij".into())]
        );
        assert_eq!(wrap("", 4), vec![(0, String::new())]);
    }

    #[test]
    fn names() {
        assert_eq!(sanitize("Kum/Fener"), "Kum-Fener");
        assert_eq!(sanitize("  "), "Adsız");
        assert_eq!(sanitize("_gizli"), "gizli");
    }

    #[test]
    fn editing_and_content() {
        let mut e = Editor::new(None);
        for c in "Başlık".chars() {
            e.insert_char(c);
        }
        e.enter();
        e.enter();
        e.insert_str("    girintili\nikinci");
        assert_eq!(e.content(), "Başlık\n#taslak\n\n    girintili\nikinci\n");
        e.backspace();
        assert_eq!(e.lines.last().unwrap(), "ikinc");
        e.undo();
        assert!(e.title() == "Başlık");
    }

    #[test]
    fn cursor_layout() {
        let mut e = Editor::new(None);
        e.insert_str("abc def ghi");
        // "abc def " fills 8 columns, "ghi" wraps; the cursor ends after "ghi".
        let (rows, cur) = e.layout(8);
        assert_eq!(rows.len(), 2);
        assert_eq!(cur, (1, 3));
    }
}
