//! The "İmzalar" screen: authors, their pen names and how many poems each
//! signature carries, plus signatures that belong to no one yet.

use super::app::{App, AuthorStep, Confirm, Mode};
use super::ui::{ACCENT, BG, FAINT, FG, MUTED, WARN, panel};
use crate::authors::{Author, norm};
use crate::build;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::collections::BTreeMap;

#[derive(Clone, PartialEq)]
pub enum Row {
    Author(usize),
    Pen(usize, usize),
    /// A signature no author claims yet.
    Unknown(String),
    NewAuthor,
}

/// Poems per signature (normalised), with the spelling first seen.
fn counts(app: &App) -> BTreeMap<String, (String, usize)> {
    let mut out: BTreeMap<String, (String, usize)> = BTreeMap::new();
    for p in app.lib.scan.all_poems() {
        if let Some(sig) = &p.signature {
            let e = out
                .entry(norm(&sig.author))
                .or_insert_with(|| (sig.author.clone(), 0));
            e.1 += 1;
        }
    }
    out
}

fn count_of(counts: &BTreeMap<String, (String, usize)>, name: &str) -> usize {
    counts.get(&norm(name)).map(|(_, n)| *n).unwrap_or(0)
}

/// The main author first, then the others, then unclaimed signatures.
pub fn rows(app: &App) -> Vec<Row> {
    let authors = &app.lib.authors;
    let mut order: Vec<usize> = vec![authors.main];
    order.extend((0..authors.list.len()).filter(|i| *i != authors.main));
    let mut rows = vec![];
    for i in order {
        rows.push(Row::Author(i));
        for j in 0..authors.list[i].pen_names.len() {
            rows.push(Row::Pen(i, j));
        }
    }
    for (_, (shown, _)) in counts(app) {
        if authors.find(&shown).is_none() {
            rows.push(Row::Unknown(shown));
        }
    }
    rows.push(Row::NewAuthor);
    rows
}

/// Saves the table and rebuilds: who counts as "you" decides what is published.
fn commit(app: &mut App, what: String) {
    match app.lib.authors.save() {
        Ok(()) => {
            build::log(&what);
            app.refresh();
            app.start_build(false);
        }
        Err(e) => app.say(format!("{e:#}")),
    }
}

fn text_key(input: &mut String, k: &KeyEvent) -> bool {
    match k.code {
        KeyCode::Backspace => {
            input.pop();
            true
        }
        KeyCode::Char(c)
            if !k.modifiers.contains(KeyModifiers::CONTROL)
                || k.modifiers.contains(KeyModifiers::ALT) =>
        {
            input.push(c);
            true
        }
        _ => false,
    }
}

pub fn key(app: &mut App, sel: usize, k: KeyEvent) -> Mode {
    let rows = rows(app);
    let sel = sel.min(rows.len() - 1);
    let author_of = |r: &Row| match r {
        Row::Author(i) | Row::Pen(i, _) => Some(*i),
        _ => None,
    };
    match k.code {
        KeyCode::Esc | KeyCode::Char('q') => Mode::Main,
        KeyCode::Up | KeyCode::Char('k') => Mode::Signatures {
            sel: sel.saturating_sub(1),
        },
        KeyCode::Down | KeyCode::Char('j') => Mode::Signatures {
            sel: (sel + 1).min(rows.len() - 1),
        },
        KeyCode::Enter => match &rows[sel] {
            Row::NewAuthor => Mode::AuthorPrompt {
                step: AuthorStep::Name,
                input: String::new(),
                back: sel,
            },
            Row::Unknown(name) => Mode::AssignPen {
                name: name.clone(),
                sel: 0,
                back: sel,
            },
            r => match author_of(r) {
                Some(author) => Mode::AuthorPrompt {
                    step: AuthorStep::AddPen { author },
                    input: String::new(),
                    back: sel,
                },
                None => Mode::Signatures { sel },
            },
        },
        KeyCode::Char('p') => match author_of(&rows[sel]) {
            Some(author) => Mode::AuthorPrompt {
                step: AuthorStep::AddPen { author },
                input: String::new(),
                back: sel,
            },
            None => Mode::Signatures { sel },
        },
        KeyCode::Char('x') => match rows[sel] {
            Row::Pen(i, j) => {
                let pen = app.lib.authors.list[i].pen_names.remove(j);
                commit(app, format!("removed pen name: {pen}"));
                Mode::Signatures { sel }
            }
            Row::Author(i) if i != app.lib.authors.main => Mode::Confirm(Confirm::RemoveAuthor(i)),
            _ => Mode::Signatures { sel },
        },
        _ => Mode::Signatures { sel },
    }
}

pub fn remove_author(app: &mut App, i: usize) {
    if i != app.lib.authors.main && i < app.lib.authors.list.len() {
        let gone = app.lib.authors.list.remove(i);
        if i < app.lib.authors.main {
            app.lib.authors.main -= 1;
        }
        commit(app, format!("removed author: {}", gone.label()));
    }
}

pub fn key_prompt(
    app: &mut App,
    step: AuthorStep,
    mut input: String,
    back: usize,
    k: KeyEvent,
) -> Mode {
    match k.code {
        KeyCode::Esc => Mode::Signatures { sel: back },
        KeyCode::Enter => match step {
            // A new author: first the real name, then a first pen name. Either may be empty.
            AuthorStep::Name => Mode::AuthorPrompt {
                step: AuthorStep::FirstPen {
                    name: input.trim().to_string(),
                },
                input: String::new(),
                back,
            },
            AuthorStep::FirstPen { name } => {
                let pen = input.trim().to_string();
                if name.is_empty() && pen.is_empty() {
                    return Mode::Signatures { sel: back };
                }
                let taken = |s: &str| !s.is_empty() && app.lib.authors.find(s).is_some();
                if taken(&name) || taken(&pen) {
                    app.say(app.t("name_taken"));
                    return Mode::Signatures { sel: back };
                }
                let author = Author {
                    name,
                    pen_names: if pen.is_empty() { vec![] } else { vec![pen] },
                };
                let label = author.label();
                app.lib.authors.list.push(author);
                commit(app, format!("new author: {label}"));
                Mode::Signatures { sel: back }
            }
            AuthorStep::AddPen { author } => {
                let pen = input.trim().to_string();
                if !pen.is_empty() {
                    if app.lib.authors.add_pen_name(author, &pen) {
                        commit(app, format!("new pen name: {pen}"));
                    } else {
                        app.say(app.t("name_taken"));
                    }
                }
                Mode::Signatures { sel: back }
            }
        },
        _ => {
            text_key(&mut input, &k);
            Mode::AuthorPrompt { step, input, back }
        }
    }
}

/// Choosing whose pen name an unclaimed signature is.
pub fn key_assign(app: &mut App, name: String, sel: usize, back: usize, k: KeyEvent) -> Mode {
    let n = app.lib.authors.list.len();
    match k.code {
        KeyCode::Esc => Mode::Signatures { sel: back },
        KeyCode::Up | KeyCode::Char('k') => Mode::AssignPen {
            name,
            sel: sel.saturating_sub(1),
            back,
        },
        KeyCode::Down | KeyCode::Char('j') => Mode::AssignPen {
            name,
            sel: (sel + 1).min(n - 1),
            back,
        },
        KeyCode::Enter => {
            if app.lib.authors.add_pen_name(sel, &name) {
                commit(
                    app,
                    format!("pen name {name} → {}", app.lib.authors.list[sel].label()),
                );
            }
            Mode::Signatures { sel: back }
        }
        _ => Mode::AssignPen { name, sel, back },
    }
}

pub fn draw(f: &mut Frame, area: Rect, app: &App, sel: usize) {
    let counts = counts(app);
    let authors = &app.lib.authors;

    if let Mode::AssignPen { name, sel, .. } = &app.mode {
        let mut lines = vec![
            Line::from(Span::styled(
                format!(" \u{201c}{name}\u{201d} {}", app.t("assign_to")),
                Style::default().fg(ACCENT),
            )),
            Line::from(""),
        ];
        for (i, a) in authors.list.iter().enumerate() {
            let style = if i == *sel {
                Style::default().fg(BG).bg(ACCENT)
            } else {
                Style::default().fg(FG)
            };
            lines.push(Line::from(Span::styled(
                format!("  {}  ", a.label()),
                style,
            )));
        }
        f.render_widget(
            Paragraph::new(lines).block(panel(app.t("signatures").trim_start_matches("◇ "), true)),
            area,
        );
        return;
    }

    let rows = rows(app);
    let mut lines = vec![];
    let mut unknown_header = false;
    for (i, row) in rows.iter().enumerate() {
        let selected = i == sel;
        let (text, n, note, mut style) = match row {
            Row::Author(a) => {
                let author = &authors.list[*a];
                let total = count_of(&counts, &author.name)
                    + author
                        .pen_names
                        .iter()
                        .map(|p| count_of(&counts, p))
                        .sum::<usize>();
                let note = if *a == authors.main {
                    app.t("main_author")
                } else {
                    app.t("tracked")
                };
                let name = if author.name.trim().is_empty() {
                    app.t("no_real_name").to_string()
                } else {
                    author.name.clone()
                };
                (
                    format!(" {name}"),
                    Some(total),
                    note,
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                )
            }
            Row::Pen(a, j) => {
                let pen = &authors.list[*a].pen_names[*j];
                (
                    format!("     {pen}"),
                    Some(count_of(&counts, pen)),
                    "",
                    Style::default().fg(FG),
                )
            }
            Row::Unknown(name) => {
                if !unknown_header {
                    unknown_header = true;
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        format!(" {}", app.t("unknown_signatures")),
                        Style::default().fg(FAINT),
                    )));
                }
                (
                    format!("     {name}"),
                    Some(count_of(&counts, name)),
                    app.t("assign_hint"),
                    Style::default().fg(WARN),
                )
            }
            Row::NewAuthor => {
                lines.push(Line::from(""));
                (
                    format!(" {}", app.t("new_author")),
                    None,
                    "",
                    Style::default().fg(FG),
                )
            }
        };
        if selected {
            style = style.fg(BG).bg(ACCENT);
        }
        let mut spans = vec![Span::styled(format!("{text:<34}"), style)];
        if let Some(n) = n {
            spans.push(Span::styled(
                format!(" {n:>3}  "),
                Style::default().fg(MUTED),
            ));
        }
        if !note.is_empty() {
            spans.push(Span::styled(note, Style::default().fg(FAINT)));
        }
        lines.push(Line::from(spans));
    }
    f.render_widget(
        Paragraph::new(lines).block(panel(app.t("signatures").trim_start_matches("◇ "), true)),
        area,
    );
}
