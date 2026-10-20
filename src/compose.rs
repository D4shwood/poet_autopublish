//! Writing a poem file: the rules behind the writing screen, shared by every
//! app that edits poems (the terminal writer and the phone).
//!
//! A poem is held as lines: line 0 is the title, then the body, then the
//! signature once signed. The draft tag is hidden while editing and goes back
//! right after the title when the file is written.

use crate::poem::{DRAFT_TAGS, Poem, Signature};
use chrono::NaiveDateTime;

/// A poem file opened for editing: its lines without the draft tag.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub lines: Vec<String>,
    pub is_draft: bool,
}

/// Splits a file into editable lines, taking out the draft tag.
pub fn open(raw: &str) -> Draft {
    let raw = raw.replace("\r\n", "\n");
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
    Draft { lines, is_draft }
}

/// The file text: the draft tag goes right after the title.
pub fn content(lines: &[String], is_draft: bool) -> String {
    let mut lines = lines.to_vec();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    if is_draft {
        lines.insert(1.min(lines.len()), "#taslak".into());
    }
    let mut s = lines.join("\n");
    s.push('\n');
    s
}

/// Signs the poem: trailing blank lines go, then a blank line and the signature.
pub fn sign(lines: &mut Vec<String>, author: &str, at: NaiveDateTime) {
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    lines.push(String::new());
    lines.push(Signature::format(author, at));
}

pub fn title(lines: &[String]) -> &str {
    lines.first().map(|l| l.trim()).unwrap_or("")
}

pub fn is_empty(lines: &[String]) -> bool {
    lines.iter().all(|l| l.trim().is_empty())
}

/// Whether the last non-blank line (not the title) is a dated signature.
pub fn is_signed(lines: &[String]) -> bool {
    lines
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .is_some_and(|i| i > 0 && Signature::parse(&lines[i]).is_some())
}

pub fn word_count(lines: &[String]) -> usize {
    lines
        .iter()
        .skip(1)
        .map(|l| l.split_whitespace().count())
        .sum()
}

/// Non-blank lines of the body.
pub fn line_count(lines: &[String]) -> usize {
    lines
        .iter()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .count()
}

/// A file name from a title: `/` replaced, Turkish letters kept. On Windows
/// the characters and names it forbids are replaced too.
pub fn sanitize(title: &str) -> String {
    sanitize_for(title, cfg!(windows))
}

/// Names Windows keeps for devices, whatever the extension.
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub fn sanitize_for(title: &str, windows: bool) -> String {
    let forbidden =
        |c: char| c == '/' || c == '\\' || c.is_control() || (windows && "<>:\"|?*".contains(c));
    let s: String = title
        .trim()
        .chars()
        .map(|c| if forbidden(c) { '-' } else { c })
        .collect();
    let mut s: String = s
        .trim_start_matches(['.', '_'])
        .trim()
        .chars()
        .take(80)
        .collect();
    if windows {
        // Windows drops trailing dots and spaces, so "Son." and "Son" would collide.
        s = s.trim_end_matches(['.', ' ']).to_string();
        let stem = s.split('.').next().unwrap_or("").to_ascii_uppercase();
        if RESERVED.contains(&stem.as_str()) {
            s.push('_');
        }
    }
    if s.is_empty() { "Adsız".into() } else { s }
}

/// `name`, or `name (2)`, `name (3)`… — the first one `taken` says is free.
pub fn unique_name(name: &str, taken: impl Fn(&str) -> bool) -> String {
    let mut candidate = name.to_string();
    let mut n = 2;
    while taken(&candidate) {
        candidate = format!("{name} ({n})");
        n += 1;
    }
    candidate
}

/// Lines from the user's own poems that suit the blank page (12–80 letters).
pub fn inspiration_lines(poems: &[Poem]) -> Vec<String> {
    poems
        .iter()
        .flat_map(|p| p.body.iter())
        .filter(|l| (12..=80).contains(&l.trim().chars().count()))
        .map(|l| l.trim().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn lines(s: &[&str]) -> Vec<String> {
        s.iter().map(|l| l.to_string()).collect()
    }

    #[test]
    fn draft_tag_round_trip() {
        let d = open("Liman\n#taslak\n\nbir mısra\n");
        assert!(d.is_draft);
        assert_eq!(d.lines, lines(&["Liman", "", "bir mısra"]));
        assert_eq!(content(&d.lines, true), "Liman\n#taslak\n\nbir mısra\n");
        assert_eq!(content(&d.lines, false), "Liman\n\nbir mısra\n");
    }

    #[test]
    fn signing() {
        let mut l = lines(&["Liman", "", "bir mısra", "", ""]);
        assert!(!is_signed(&l));
        let at = NaiveDate::from_ymd_opt(2026, 10, 5)
            .unwrap()
            .and_hms_opt(21, 30, 0)
            .unwrap();
        sign(&mut l, "Deniz Yalın", at);
        assert_eq!(l.last().unwrap(), "- Deniz Yalın. 05.10.2026 - 21.30");
        assert_eq!(l[l.len() - 2], "");
        assert!(is_signed(&l));
        assert_eq!((word_count(&l), line_count(&l)), (8, 2));
    }

    #[test]
    fn names() {
        assert_eq!(sanitize("Kum/Fener"), "Kum-Fener");
        assert_eq!(sanitize(" ._ "), "Adsız");
        // Windows' forbidden characters, trailing dots and device names.
        assert_eq!(
            sanitize_for("Neden? Niçin: \"biz\"", true),
            "Neden- Niçin- -biz-"
        );
        assert_eq!(sanitize_for("Neden?", false), "Neden?");
        assert_eq!(sanitize_for("Hamd olsun. Son.", true), "Hamd olsun. Son");
        assert_eq!(sanitize_for("con", true), "con_");
        assert_eq!(sanitize_for("Konser", true), "Konser");
        let taken = ["Liman", "Liman (2)"];
        assert_eq!(unique_name("Liman", |n| taken.contains(&n)), "Liman (3)");
        assert_eq!(unique_name("Kum", |n| taken.contains(&n)), "Kum");
    }
}
