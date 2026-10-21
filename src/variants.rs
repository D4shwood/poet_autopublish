//! The draft as an artifact: what changed between two versions of a poem,
//! word by word, and the path from a poem's first draft to its final form.

use crate::compose;
use crate::history;
use chrono::NaiveDateTime;
use std::fs;

/// A run of text on a line: kept, removed, or added.
#[derive(Debug, Clone, PartialEq)]
pub enum Seg {
    Same(String),
    Gone(String),
    New(String),
}

/// One line of a comparison, as segments.
pub type LineDiff = Vec<Seg>;

/// The poem's lines without the draft tag: a draft and its signed version
/// differ in words, not in that tag.
fn lines(text: &str) -> Vec<String> {
    let d = compose::open(text);
    let mut l = d.lines;
    while l.last().is_some_and(|x| x.trim().is_empty()) {
        l.pop();
    }
    l
}

enum Op<'a, T> {
    Same(&'a T),
    Gone(&'a T),
    New(&'a T),
}

/// The longest common subsequence as a list of edits.
fn lcs<'a, T: PartialEq>(a: &'a [T], b: &'a [T]) -> Vec<Op<'a, T>> {
    let (n, m) = (a.len(), b.len());
    let mut t = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            t[i][j] = if a[i] == b[j] {
                t[i + 1][j + 1] + 1
            } else {
                t[i + 1][j].max(t[i][j + 1])
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, vec![]);
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            out.push(Op::Same(&a[i]));
            i += 1;
            j += 1;
        } else if i < n && (j == m || t[i + 1][j] >= t[i][j + 1]) {
            // On a tie the removal comes first, as one reads a correction.
            out.push(Op::Gone(&a[i]));
            i += 1;
        } else {
            out.push(Op::New(&b[j]));
            j += 1;
        }
    }
    out
}

/// Words and the spaces between them, so a line can be rebuilt exactly.
fn tokens(line: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for c in line.chars() {
        match out.last_mut() {
            Some(t) if t.chars().next().unwrap().is_whitespace() == c.is_whitespace() => t.push(c),
            _ => out.push(c.to_string()),
        }
    }
    out
}

/// Appends text to the last segment when it is of the same kind.
fn push(segs: &mut LineDiff, s: Seg) {
    match (segs.last_mut(), s) {
        (Some(Seg::Same(a)), Seg::Same(b))
        | (Some(Seg::Gone(a)), Seg::Gone(b))
        | (Some(Seg::New(a)), Seg::New(b)) => a.push_str(&b),
        (_, s) => segs.push(s),
    }
}

/// A changed line, word by word.
fn words(old: &str, new: &str) -> LineDiff {
    let (a, b) = (tokens(old), tokens(new));
    let mut segs = vec![];
    for op in lcs(&a, &b) {
        push(
            &mut segs,
            match op {
                Op::Same(t) => Seg::Same(t.clone()),
                Op::Gone(t) => Seg::Gone(t.clone()),
                Op::New(t) => Seg::New(t.clone()),
            },
        );
    }
    segs
}

/// Two versions compared: unchanged lines, removed and added lines, and lines
/// that changed shown word by word.
pub fn diff(old: &str, new: &str) -> Vec<LineDiff> {
    let (a, b) = (lines(old), lines(new));
    let mut out = vec![];
    let (mut gone, mut added): (Vec<&String>, Vec<&String>) = (vec![], vec![]);
    // A run of removed lines followed by added ones pairs up line by line.
    let flush = |out: &mut Vec<LineDiff>, gone: &mut Vec<&String>, added: &mut Vec<&String>| {
        let pairs = gone.len().min(added.len());
        for k in 0..pairs {
            out.push(words(gone[k], added[k]));
        }
        for g in &gone[pairs..] {
            out.push(vec![Seg::Gone(g.to_string())]);
        }
        for a in &added[pairs..] {
            out.push(vec![Seg::New(a.to_string())]);
        }
        gone.clear();
        added.clear();
    };
    for op in lcs(&a, &b) {
        match op {
            Op::Same(l) => {
                flush(&mut out, &mut gone, &mut added);
                out.push(vec![Seg::Same(l.clone())]);
            }
            Op::Gone(l) => gone.push(l),
            Op::New(l) => added.push(l),
        }
    }
    flush(&mut out, &mut gone, &mut added);
    out
}

/// Whether a comparison changed anything.
pub fn changed(d: &[LineDiff]) -> bool {
    d.iter().flatten().any(|s| !matches!(s, Seg::Same(_)))
}

/// A poem's versions, oldest first, each with when it was saved; only the
/// ones that differ in words. The current text closes the list.
pub fn versions(file_name: &str, current: &str) -> Vec<(Option<NaiveDateTime>, String)> {
    let mut out: Vec<(Option<NaiveDateTime>, String)> = vec![];
    let snapshots = history::list(file_name)
        .into_iter()
        .filter_map(|s| fs::read_to_string(&s.path).ok().map(|t| (Some(s.at), t)));
    for (at, text) in snapshots.chain(std::iter::once((None, current.to_string()))) {
        if out
            .last()
            .is_none_or(|(_, prev)| lines(prev) != lines(&text))
        {
            out.push((at, text));
        }
    }
    out
}

/// The path from the first draft to the current text, if it ever changed.
pub fn journey(file_name: &str, current: &str) -> Option<Vec<LineDiff>> {
    let v = versions(file_name, current);
    let first = &v.first()?.1;
    (v.len() >= 2).then(|| diff(first, current))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(d: &[LineDiff]) -> Vec<String> {
        d.iter()
            .map(|l| {
                l.iter()
                    .map(|s| match s {
                        Seg::Same(t) => t.clone(),
                        Seg::Gone(t) => format!("[-{t}]"),
                        Seg::New(t) => format!("[+{t}]"),
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_word_changes() {
        let d = diff(
            "Liman\n\nkıyıda bekleyen martılar\n",
            "Liman\n\nkıyıda susan martılar\n",
        );
        assert_eq!(
            show(&d),
            vec!["Liman", "", "kıyıda [-bekleyen][+susan] martılar"]
        );
    }

    #[test]
    fn lines_come_and_go() {
        let d = diff("Liman\n\nbir\niki\nüç\n", "Liman\n\nbir\nüç\ndört\n");
        assert_eq!(
            show(&d),
            vec!["Liman", "", "bir", "[-iki]", "üç", "[+dört]"]
        );
    }

    #[test]
    fn the_draft_tag_and_signing() {
        let draft = "Liman\n#taslak\n\nbir mısra\n";
        let signed = "Liman\n\nbir mısra\n\n- Deniz Yalın. 05.10.2026 - 21.30\n";
        let d = diff(draft, signed);
        // Only the signature is new; the tag isn't a change.
        assert_eq!(
            show(&d),
            vec![
                "Liman",
                "",
                "bir mısra",
                "[+]",
                "[+- Deniz Yalın. 05.10.2026 - 21.30]"
            ]
        );
        assert!(!changed(&diff(draft, "Liman\n\nbir mısra\n")));
    }
}
