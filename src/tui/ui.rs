use super::app::{App, Confirm, Item, Mode, Watching};
#[cfg(feature = "preview")]
use super::preview::Page;
use super::writer::Editor;
#[cfg(feature = "preview")]
use super::writer::wrap;
use crate::authors::norm;
use crate::poem::{Signature, Status};
use crate::prosody;
use crate::stats;
use chrono::{Datelike, Local};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use std::fs;
use std::sync::atomic::Ordering;
use unicode_width::UnicodeWidthStr;

pub const BG: Color = Color::Rgb(0x15, 0x15, 0x15);
pub const FG: Color = Color::Rgb(0xe6, 0xe6, 0xe6);
pub const MUTED: Color = Color::Rgb(0x88, 0x88, 0x88);
pub const FAINT: Color = Color::Rgb(0x4a, 0x4a, 0x4a);
pub const ACCENT: Color = Color::Rgb(0xd8, 0xcf, 0xc0);
pub const WARN: Color = Color::Rgb(0xc8, 0x8a, 0x5a);

fn base() -> Style {
    Style::default().fg(FG).bg(BG)
}

pub(super) fn panel(title: &str, focused: bool) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused { ACCENT } else { FAINT }))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(if focused { ACCENT } else { MUTED }),
        ))
        .style(base())
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    f.render_widget(Block::default().style(base()), area);
    if let Mode::Setup(s) = &app.mode {
        super::setup::draw(f, area, s);
        return;
    }
    if let Mode::Writer(ed) = &mut app.mode {
        draw_writer(f, area, ed, app.lib.lang, app.cfg.prosody);
        return;
    }

    // The key hints wrap onto as many lines as the terminal width needs.
    let (hint, footer_style) = footer(app);
    let footer_lines = wrap_hint(&hint, area.width.saturating_sub(2) as usize);
    let [header, body, log, keys] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(7),
        Constraint::Length(footer_lines.len() as u16),
    ])
    .areas(area);
    draw_header(f, header, app);

    match &app.mode {
        Mode::Stats => draw_stats(f, body, app),
        Mode::Signatures { sel } => super::signatures::draw(f, body, app, *sel),
        Mode::AuthorPrompt { back, .. } | Mode::AssignPen { back, .. } => {
            super::signatures::draw(f, body, app, *back)
        }
        #[cfg(feature = "preview")]
        Mode::Preview => draw_preview(f, body.union(log), app),
        Mode::Log { scroll } => {
            let s = *scroll;
            draw_log(f, body.union(log), app, s, true);
        }
        _ => {
            let [left, right] =
                Layout::horizontal([Constraint::Percentage(32), Constraint::Percentage(68)])
                    .areas(body);
            draw_left(f, left, app);
            draw_right(f, right, app);
        }
    }
    if !full_screen(&app.mode) {
        draw_log(f, log, app, 0, false);
    }

    let lines: Vec<Line> = footer_lines
        .into_iter()
        .map(|l| Line::from(Span::styled(format!(" {l}"), footer_style)))
        .collect();
    f.render_widget(Paragraph::new(lines), keys);
}

/// The bottom line's text: key hints, a prompt, or a passing message.
fn footer(app: &App) -> (String, Style) {
    let hint = match &app.mode {
        Mode::Confirm(Confirm::Force) => app.t("confirm_force").to_string(),
        Mode::Confirm(Confirm::RemoveAuthor(_)) => app.t("confirm_remove_author").to_string(),
        Mode::Signatures { .. } => app.t("sig_keys").to_string(),
        Mode::AssignPen { .. } => "⏎ · ↑↓ · Esc".into(),
        Mode::AuthorPrompt { step, input, .. } => {
            let label = match step {
                super::app::AuthorStep::Name => app.t("author_name"),
                super::app::AuthorStep::FirstPen { .. } => app.t("first_pen"),
                super::app::AuthorStep::AddPen { .. } => app.t("add_pen"),
            };
            format!("{label}: {input}▏   {}", app.t("prompt_keys"))
        }
        Mode::Confirm(Confirm::Trash(p)) => {
            format!(
                "{} — {}",
                p.file_name().unwrap_or_default().to_string_lossy(),
                app.t("confirm_trash")
            )
        }
        Mode::WatcherMenu => format!("{}: {}", app.t("watcher"), app.t("watcher_menu")),
        Mode::Rename { input, .. } => {
            format!("{}: {input}▏   {}", app.t("rename"), app.t("rename_keys"))
        }
        Mode::NewCollection { input } => format!(
            "{}: {input}▏   {}",
            app.t("collection_name"),
            app.t("collection_keys")
        ),
        Mode::Search { .. } => "⏎ · ↑↓ · Esc".into(),
        Mode::History { .. } | Mode::Stats | Mode::Log { .. } => "↑↓ · Esc".into(),
        #[cfg(feature = "preview")]
        Mode::Preview => app.t("preview_keys").into(),
        _ => match &app.flash {
            Some((m, _)) => m.clone(),
            // `r` renames weeks, so it only belongs to the series.
            None if app.space.is_some() => app
                .t("keys_main")
                .split(" · ")
                .filter(|k| !k.starts_with("r "))
                .collect::<Vec<_>>()
                .join(" · "),
            None => app.t("keys_main").into(),
        },
    };
    let hint = if app.quitting {
        app.t("quitting").to_string()
    } else {
        hint
    };
    let style = if app.quitting
        || matches!(
            app.mode,
            Mode::Confirm(_)
                | Mode::WatcherMenu
                | Mode::Rename { .. }
                | Mode::NewCollection { .. }
                | Mode::AuthorPrompt { .. }
        ) {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(MUTED)
    };
    (hint, style)
}

/// Splits hints at " · " (or spaces) into lines that fit `width` columns.
fn wrap_hint(hint: &str, width: usize) -> Vec<String> {
    let sep = if hint.contains(" · ") { " · " } else { " " };
    let mut lines: Vec<String> = vec![];
    let mut cur = String::new();
    for part in hint.split(sep) {
        let candidate = if cur.is_empty() {
            part.to_string()
        } else {
            format!("{cur}{sep}{part}")
        };
        if candidate.width() > width && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
            cur = part.to_string();
        } else {
            cur = candidate;
        }
    }
    lines.push(cur);
    lines
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let watch = match app.watching {
        Watching::Service => app.t("watch_service"),
        Watching::Tui => app.t("watch_tui"),
        Watching::Off => app.t("watch_off"),
    };
    let building = if app.building.load(Ordering::SeqCst) {
        format!("  {}", app.t("building"))
    } else {
        String::new()
    };
    let left = Line::from(vec![
        Span::styled(" poetautopublish", Style::default().fg(MUTED)),
        Span::styled(" — ", Style::default().fg(FAINT)),
        Span::styled(
            app.space_name(),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(left), area);
    let right = Line::from(vec![
        Span::styled(building, Style::default().fg(ACCENT)),
        Span::styled(format!("  {watch} "), Style::default().fg(MUTED)),
    ])
    .right_aligned();
    f.render_widget(Paragraph::new(right), area);
}

fn item_label(app: &App, item: Item) -> String {
    match item {
        Item::NewPoem => app.t("new_poem").into(),
        Item::ContinueDraft => app.t("continue_draft").into(),
        Item::Week(i) => {
            let w = &app.lib.weeks[i];
            format!("  {}  ({})", w.label(app.lang()), w.poems.len())
        }
        Item::All => format!("  {}  ({})", app.t("all_poems"), app.count(item)),
        Item::Collection(i) => {
            format!("  {}  ({})", app.lib.scan.collections[i].0, app.count(item))
        }
        Item::NewCollection => format!("  {}", app.t("new_collection")),
        Item::Book => format!("  {}  ({})", app.t("all_poems"), app.count(item)),
        Item::Publications => format!("  {}  ({})", app.t("publications"), app.count(item)),
        Item::Series => format!("  ◆ {}  ({})", app.cfg.series, app.count(item)),
        Item::Unsigned => format!("{}  ({})", app.t("unsigned"), app.count(item)),
        Item::Drafts => format!("{}  ({})", app.t("drafts"), app.count(item)),
        Item::Stats => app.t("stats").into(),
        Item::Signatures => app.t("signatures").into(),
        Item::Settings => app.t("settings").into(),
        Item::Quit => app.t("quit_item").into(),
    }
}

fn draw_left(f: &mut Frame, area: Rect, app: &App) {
    let focused = !app.focus_right
        && matches!(
            app.mode,
            Mode::Main
                | Mode::Confirm(_)
                | Mode::WatcherMenu
                | Mode::Rename { .. }
                | Mode::NewCollection { .. }
        );
    let mut lines = vec![];
    for (i, item) in app.items.iter().enumerate() {
        // A faint rule between the actions, the volumes and the rest.
        if matches!(item, Item::Week(_) | Item::All)
            && i > 0
            && matches!(app.items[i - 1], Item::ContinueDraft)
        {
            lines.push(Line::from(Span::styled(
                format!(" {}", app.t("weeks")),
                Style::default().fg(FAINT),
            )));
        }
        // Inside a collection the same group reads "Other spaces": the series and
        // the other collections you can switch to.
        if matches!(
            item,
            Item::Collection(_) | Item::NewCollection | Item::Series
        ) && i > 0
            && !matches!(app.items[i - 1], Item::Collection(_) | Item::Series)
        {
            let title = if app.space.is_some() {
                app.t("other_spaces")
            } else {
                app.t("collections")
            };
            lines.push(Line::from(Span::styled(
                format!(" {title}"),
                Style::default().fg(FAINT),
            )));
        }
        if *item == Item::Quit {
            lines.push(Line::from(""));
        }
        if *item == Item::Unsigned {
            lines.push(Line::from(""));
        }
        let selected = i == app.left;
        let mut style = match item {
            Item::NewPoem => Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            Item::Unsigned if app.count(*item) > 0 => Style::default().fg(WARN),
            _ => Style::default().fg(FG),
        };
        if selected {
            style = style.bg(if focused {
                Color::Rgb(0x2c, 0x2a, 0x26)
            } else {
                Color::Rgb(0x22, 0x22, 0x22)
            });
        }
        let marker = if selected { "›" } else { " " };
        lines.push(Line::from(Span::styled(
            format!("{marker} {}", item_label(app, *item)),
            style,
        )));
    }
    f.render_widget(
        Paragraph::new(lines).block(panel(app.t("poems"), focused)),
        area,
    );
}

fn draw_right(f: &mut Frame, area: Rect, app: &mut App) {
    match &app.mode {
        Mode::Search { query, hits, sel } => {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("/ ", Style::default().fg(ACCENT)),
                    Span::styled(query.clone(), Style::default().fg(FG)),
                    Span::styled("▏", Style::default().fg(ACCENT)),
                ]),
                Line::from(""),
            ];
            let visible = area.height.saturating_sub(4) as usize;
            let start = sel.saturating_sub(visible.saturating_sub(1));
            for (i, h) in hits.iter().enumerate().skip(start).take(visible) {
                let style = if i == *sel {
                    Style::default().fg(BG).bg(ACCENT)
                } else {
                    Style::default().fg(FG)
                };
                let when = h
                    .snapshot
                    .map(|t| format!("  ⟲ {}", t.format("%d.%m %H:%M")))
                    .unwrap_or_default();
                lines.push(Line::from(vec![
                    Span::styled(format!("{:<22.22}", h.title), Style::default().fg(ACCENT)),
                    Span::styled(format!(" {}", h.line), style),
                    Span::styled(when, Style::default().fg(MUTED)),
                ]));
            }
            f.render_widget(
                Paragraph::new(lines).block(panel(app.t("search"), true)),
                area,
            );
        }
        Mode::History { title, snaps, sel } => {
            let [list, view] =
                Layout::horizontal([Constraint::Length(22), Constraint::Min(10)]).areas(area);
            let lines: Vec<Line> = if snaps.is_empty() {
                vec![Line::from(Span::styled(
                    app.t("no_history"),
                    Style::default().fg(MUTED),
                ))]
            } else {
                snaps
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        let style = if i == *sel {
                            Style::default().fg(BG).bg(ACCENT)
                        } else {
                            Style::default().fg(FG)
                        };
                        Line::from(Span::styled(
                            s.at.format(" %d.%m.%Y %H:%M ").to_string(),
                            style,
                        ))
                    })
                    .collect()
            };
            f.render_widget(
                Paragraph::new(lines).block(panel(app.t("history"), true)),
                list,
            );
            let text = snaps
                .get(*sel)
                .and_then(|s| fs::read_to_string(&s.path).ok())
                .unwrap_or_default();
            f.render_widget(
                Paragraph::new(text)
                    .style(base())
                    .wrap(Wrap { trim: false })
                    .block(panel(title, false)),
                view,
            );
        }
        _ if app.current_item() == Item::Publications => draw_publications(f, area, app),
        _ => draw_poem_list(f, area, app),
    }
}

fn draw_poem_list(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus_right;
    let poems = app.right_poems();
    let cutoff = app.cfg.night_cutoff();
    let title = item_label(app, app.current_item()).trim().to_string();
    if poems.is_empty() {
        let hint = match app.current_item() {
            Item::Stats => "⏎".to_string(),
            Item::NewPoem => format!("⏎  {}", app.t("new_poem")),
            Item::NewCollection => format!("⏎  {}", app.t("new_collection")),
            Item::Collection(_) => app.t("new_in_collection").to_string(),
            _ => app.t("empty").to_string(),
        };
        f.render_widget(
            Paragraph::new(Span::styled(format!(" {hint}"), Style::default().fg(MUTED)))
                .block(panel(&title, focused)),
            area,
        );
        return;
    }
    let lines: Vec<Line> = poems
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let selected = focused && i == app.right;
            let when = match &p.signature {
                Some(s) if s.date.is_none() => String::new(),
                Some(s) => {
                    let mut w = s
                        .date
                        .map(|d| d.format("%d.%m").to_string())
                        .unwrap_or_default();
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
                None => String::new(),
            };
            let tag = match p.status(&app.lib.authors) {
                Status::Draft => {
                    format!("{} · {} {}", app.t("draft"), p.line_count(), app.t("lines"))
                }
                Status::Unsigned => format!("{} {}", p.line_count(), app.t("lines")),
                Status::Foreign(a) => format!("{}: {a}", app.t("foreign")),
                Status::Published => format!("{} {}", p.line_count(), app.t("lines")),
            };
            let style = if selected {
                Style::default().fg(BG).bg(ACCENT)
            } else {
                Style::default().fg(FG)
            };
            Line::from(vec![
                Span::styled(format!(" {:<34}", truncate(&p.title, 34)), style),
                Span::styled(format!(" {when:<14}"), Style::default().fg(MUTED)),
                Span::styled(tag, Style::default().fg(FAINT)),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines).block(panel(&title, focused)), area);
}

/// The space's published poems as a tree: journal → issue / edition → poem.
fn draw_publications(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus_right;
    let poems = app.right_poems();
    let title = format!(
        "{} · {}",
        app.space_name(),
        app.t("publications").trim_start_matches("❖ ")
    );
    if poems.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(" {}", app.t("no_publications")),
                Style::default().fg(MUTED),
            ))
            .block(panel(&title, focused)),
            area,
        );
        return;
    }
    let mut lines: Vec<Line> = vec![];
    let mut last_name: Option<String> = None;
    let mut last_issue: Option<Option<String>> = None;
    for (i, p) in poems.iter().enumerate() {
        let sig = p.signature.as_ref().unwrap();
        let pubn = sig.publication().unwrap();
        let issue = pubn.issue_label();
        if last_name.as_deref() != Some(pubn.name.as_str()) {
            if last_name.is_some() {
                lines.push(Line::from(""));
            }
            lines.push(Line::from(Span::styled(
                format!(" {}", pubn.name),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )));
            last_name = Some(pubn.name.clone());
            last_issue = None;
        }
        if last_issue.as_ref() != Some(&issue) {
            if let Some(label) = &issue {
                lines.push(Line::from(Span::styled(
                    format!("    {label}"),
                    Style::default().fg(MUTED),
                )));
            }
            last_issue = Some(issue.clone());
        }
        let indent = if issue.is_some() { "       " } else { "    " };
        let style = if focused && i == app.right {
            Style::default().fg(BG).bg(ACCENT)
        } else {
            Style::default().fg(FG)
        };
        let mut when = sig
            .date
            .map(|d| d.format("%d.%m.%Y").to_string())
            .unwrap_or_default();
        if sig.approx {
            when.insert(0, '~');
        }
        lines.push(Line::from(vec![
            Span::styled(format!("{indent}{:<30}", truncate(&p.title, 30)), style),
            // A fixed-width date column, so undated poems keep the names aligned.
            Span::styled(format!(" {when:<12}"), Style::default().fg(FAINT)),
            // The signature as printed, except your real name (it's on the cover):
            // your pen names show, and so do other authors.
            Span::styled(
                if norm(&sig.author) == norm(&app.lib.authors.main_author().name) {
                    String::new()
                } else {
                    format!(" {}", sig.author)
                },
                Style::default().fg(MUTED),
            ),
        ]));
    }
    f.render_widget(Paragraph::new(lines).block(panel(&title, focused)), area);
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n - 1).collect();
        t.push('…');
        t
    }
}

fn draw_log(f: &mut Frame, area: Rect, app: &App, scroll: usize, focused: bool) {
    let h = area.height.saturating_sub(2) as usize;
    let n = app.log_tail.len();
    let end = n.saturating_sub(scroll);
    let start = end.saturating_sub(h);
    let lines: Vec<Line> = app.log_tail[start..end]
        .iter()
        .map(|l| {
            let color = if l.contains("ERROR") {
                WARN
            } else if l.contains("built") {
                ACCENT
            } else {
                MUTED
            };
            Line::from(Span::styled(l.clone(), Style::default().fg(color)))
        })
        .collect();
    f.render_widget(
        Paragraph::new(lines).block(panel(app.t("log"), focused)),
        area,
    );
}

fn draw_stats(f: &mut Frame, area: Rect, app: &App) {
    // The current space's numbers: weeks for the series, years for a collection.
    let published = app.space_published();
    let (groups, group_title) = match app.space {
        None => (
            stats::per_week(&app.lib.weeks, app.lang()),
            app.t("per_week"),
        ),
        Some(_) => (stats::per_year(&published), app.t("per_year")),
    };
    let s = stats::compute(&published, groups, Local::now().date_naive());
    let [top, charts] = Layout::vertical([Constraint::Length(5), Constraint::Min(5)]).areas(area);
    let summary = vec![
        Line::from(vec![
            Span::styled(format!(" {}: ", app.t("total")), Style::default().fg(MUTED)),
            Span::styled(app.lang().poem_count(s.poems), Style::default().fg(ACCENT)),
            Span::styled(
                format!(
                    " · {} {} · {} {}",
                    s.lines,
                    app.t("lines"),
                    s.words,
                    app.t("words")
                ),
                Style::default().fg(FG),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" {}: ", app.t("streak")),
                Style::default().fg(MUTED),
            ),
            Span::styled(format!("{}", s.streak), Style::default().fg(ACCENT)),
            Span::styled(
                format!("  (max {})", s.longest_streak),
                Style::default().fg(MUTED),
            ),
        ]),
    ];
    f.render_widget(
        Paragraph::new(summary).block(panel(app.t("stats").trim_start_matches("▤ "), true)),
        top,
    );

    let [weeks, hours] =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).areas(charts);
    let max = s
        .per_group
        .iter()
        .map(|(_, n)| *n)
        .max()
        .unwrap_or(1)
        .max(1);
    let bar_w = weeks.width.saturating_sub(18) as u64;
    let rows = weeks.height.saturating_sub(2) as usize;
    let lines: Vec<Line> = s
        .per_group
        .iter()
        .rev()
        .take(rows)
        .map(|(label, n)| {
            let len = (n * bar_w).div_ceil(max) as usize;
            Line::from(vec![
                Span::styled(format!(" {label:>9} "), Style::default().fg(MUTED)),
                Span::styled("█".repeat(len), Style::default().fg(ACCENT)),
                Span::styled(format!(" {n}"), Style::default().fg(FG)),
            ])
        })
        .collect();
    f.render_widget(
        Paragraph::new(lines).block(panel(group_title, false)),
        weeks,
    );

    // Hours of the day as a vertical column chart.
    let inner_h = hours.height.saturating_sub(3) as u64;
    let hmax = s.by_hour.iter().copied().max().unwrap_or(1).max(1);
    let col_w = ((hours.width.saturating_sub(2)) / 24).max(1) as usize;
    let mut grid: Vec<Line> = vec![];
    for level in (1..=inner_h).rev() {
        let spans: Vec<Span> = s
            .by_hour
            .iter()
            .map(|&n| {
                let filled = (n * inner_h).div_ceil(hmax) >= level && n > 0;
                Span::styled(
                    format!("{:<w$}", if filled { "█" } else { " " }, w = col_w),
                    Style::default().fg(ACCENT),
                )
            })
            .collect();
        grid.push(Line::from(spans));
    }
    let axis: Vec<Span> = (0..24)
        .map(|h| {
            let label = if h % 6 == 0 {
                format!("{h:02}")
            } else {
                String::new()
            };
            Span::styled(
                format!("{label:<w$}", w = col_w),
                Style::default().fg(MUTED),
            )
        })
        .collect();
    grid.push(Line::from(axis));
    f.render_widget(
        Paragraph::new(grid).block(panel(app.t("by_hour"), false)),
        hours,
    );
    let _ = Local::now().year();
}

/// Cells beside the text for the syllable count and rhyme ("⚠14 a+r").
const MARGIN: u16 = 8;

/// Syllables and rhymes for the body lines, keyed by line index; the signature
/// and blank lines get none. Also the meter most lines keep.
fn verse_marks(
    ed: &Editor,
) -> (
    std::collections::HashMap<usize, (String, bool)>,
    Option<prosody::Meter>,
) {
    let verse: Vec<usize> = (1..ed.lines.len())
        .filter(|&i| !ed.lines[i].trim().is_empty() && Signature::parse(&ed.lines[i]).is_none())
        .collect();
    let texts: Vec<&str> = verse.iter().map(|&i| ed.lines[i].as_str()).collect();
    let meter = prosody::meter(&texts);
    let rhymes = prosody::rhymes(&texts);
    let marks = verse
        .iter()
        .enumerate()
        .map(|(k, &i)| {
            let n = prosody::line_shape(texts[k]).total;
            let off = meter.as_ref().is_some_and(|m| !m.fits(texts[k]));
            let rhyme = rhymes[k]
                .as_ref()
                .map(|r| format!("{}{}", r.letter, if r.redif { "+r" } else { "" }))
                .unwrap_or_default();
            (
                i,
                (
                    format!("{}{n:>2} {rhyme}", if off { "⚠" } else { " " }),
                    off,
                ),
            )
        })
        .collect();
    (marks, meter)
}

fn draw_writer(f: &mut Frame, area: Rect, ed: &mut Editor, lang: crate::lang::Lang, prosody: bool) {
    let (marks, meter) = if prosody {
        verse_marks(ed)
    } else {
        Default::default()
    };
    let mins = ed.started.elapsed().as_secs() / 60;
    let saved = match (&ed.last_saved, ed.dirty) {
        (Some(t), false) => format!("{} {}", lang.t("saved"), t.format("%H.%M")),
        (Some(t), true) => format!("{} {} *", lang.t("saved"), t.format("%H.%M")),
        (None, _) => lang.t("unsaved").to_string(),
    };
    let state = if ed.is_signed() {
        lang.t("signed")
    } else {
        lang.t("draft")
    };
    let mut tw = if ed.typewriter { " · ⌨" } else { "" }.to_string();
    if prosody {
        tw.push_str(&match &meter {
            Some(m) => format!(
                " · {} {} {}/{}",
                lang.t("syllables"),
                m.label(),
                m.fits,
                m.of
            ),
            None => format!(" · {}", lang.t("free_verse")),
        });
    }
    let msg = if ed.confirm_sign {
        lang.t("confirm_sign").to_string()
    } else {
        ed.message.clone().unwrap_or_default()
    };
    let info = format!(
        "{} {} · {} {} · {} {} · {} · {} · {}{}",
        ed.line_count(),
        lang.t("lines"),
        ed.word_count(),
        lang.t("words"),
        mins,
        lang.t("min"),
        Local::now().format("%H.%M"),
        saved,
        state,
        tw
    );
    let keys = lang.t("writer_keys").to_string();
    let show_keys = !ed.confirm_sign && ed.message.is_none();
    let avail = area.width.saturating_sub(2) as usize;
    let used = info.width() + if msg.is_empty() { 0 } else { msg.width() + 3 };
    let one_line = used + if show_keys { keys.width() + 4 } else { 0 } <= avail;
    let info_lines = wrap_hint(&info, avail);
    let status_h = if one_line {
        1
    } else {
        info_lines.len()
            + usize::from(!msg.is_empty())
            + if show_keys {
                wrap_hint(&keys, avail).len()
            } else {
                0
            }
    };
    let [page, status] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(status_h as u16)]).areas(area);
    // With the meter on, the text leaves room on its right for the marks.
    let room = if prosody { 2 * (MARGIN + 1) } else { 0 };
    let width = (page.width.saturating_sub(8 + room) as usize).min(76);
    let left_x = page.x + (page.width.saturating_sub(width as u16)) / 2;
    let text_area = Rect {
        x: left_x,
        y: page.y + 1,
        width: width as u16,
        height: page.height.saturating_sub(2),
    };
    let h = text_area.height as usize;

    let (rows, (cur_row, cur_col)) = ed.layout(width);
    // Typewriter mode keeps the cursor on the middle row; otherwise scroll only when needed.
    let (top_pad, first) = if ed.typewriter {
        let mid = h / 2;
        if cur_row >= mid {
            (0, cur_row - mid)
        } else {
            (mid - cur_row, 0)
        }
    } else {
        if cur_row < ed.scroll {
            ed.scroll = cur_row;
        } else if cur_row >= ed.scroll + h {
            ed.scroll = cur_row + 1 - h;
        }
        (0, ed.scroll)
    };

    let mut lines: Vec<Line> = vec![Line::from(""); top_pad];
    for (li, _start, text, cont) in rows.iter().skip(first).take(h.saturating_sub(top_pad)) {
        let style = if *li == 0 {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(FG)
        };
        let _ = cont;
        lines.push(Line::from(Span::styled(text.clone(), style)));
    }
    if ed.is_empty()
        && let Some(insp) = &ed.inspiration
    {
        lines = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                insp.clone(),
                Style::default().fg(FAINT).add_modifier(Modifier::ITALIC),
            )),
        ];
    }
    f.render_widget(Paragraph::new(lines).style(base()), text_area);

    // ↪ in the left margin marks soft-wrapped continuation rows.
    for (i, (_, _, _, cont)) in rows
        .iter()
        .skip(first)
        .take(h.saturating_sub(top_pad))
        .enumerate()
    {
        if *cont && left_x >= 2 {
            let r = Rect {
                x: left_x - 2,
                y: text_area.y + (top_pad + i) as u16,
                width: 1,
                height: 1,
            };
            f.render_widget(
                Paragraph::new(Span::styled("↪", Style::default().fg(FAINT))),
                r,
            );
        }
    }

    // The syllable count and rhyme beside each line's first row.
    if prosody {
        for (i, (li, _, _, cont)) in rows
            .iter()
            .skip(first)
            .take(h.saturating_sub(top_pad))
            .enumerate()
        {
            let Some((mark, off)) = marks.get(li).filter(|_| !*cont) else {
                continue;
            };
            let r = Rect {
                x: text_area.x + text_area.width + 1,
                y: text_area.y + (top_pad + i) as u16,
                width: MARGIN,
                height: 1,
            };
            if r.x + r.width <= page.x + page.width {
                let color = if *off { WARN } else { FAINT };
                f.render_widget(
                    Paragraph::new(Span::styled(mark.clone(), Style::default().fg(color))),
                    r,
                );
            }
        }
    }

    let cy = text_area.y as usize + top_pad + cur_row - first;
    if cy < (text_area.y + text_area.height) as usize {
        f.set_cursor_position(Position::new(text_area.x + cur_col as u16, cy as u16));
    }

    // One line when it all fits; otherwise the status and the keys wrap below each other.
    let faint = Style::default().fg(FAINT);
    let mut lines: Vec<Line> = vec![];
    if one_line {
        let mut spans = vec![Span::styled(format!(" {info}"), faint)];
        if !msg.is_empty() {
            spans.push(Span::styled(
                format!("   {msg}"),
                Style::default().fg(ACCENT),
            ));
        }
        lines.push(Line::from(spans));
    } else {
        for l in info_lines {
            lines.push(Line::from(Span::styled(format!(" {l}"), faint)));
        }
        if !msg.is_empty() {
            lines.push(Line::from(Span::styled(
                format!(" {msg}"),
                Style::default().fg(ACCENT),
            )));
        }
    }
    f.render_widget(Clear, status);
    f.render_widget(Paragraph::new(lines).style(base()), status);
    if show_keys {
        let keys_lines: Vec<Line> = if one_line {
            vec![Line::from(Span::styled(format!("{keys} "), faint)).right_aligned()]
        } else {
            wrap_hint(&keys, avail)
                .into_iter()
                .map(|l| Line::from(Span::styled(format!(" {l}"), faint)))
                .collect()
        };
        let y = if one_line {
            status.y
        } else {
            status.y + status.height - keys_lines.len() as u16
        };
        let r = Rect {
            y,
            height: keys_lines.len() as u16,
            ..status
        };
        f.render_widget(Paragraph::new(keys_lines), r);
    }
}

/// Screens that take the log panel's space too.
fn full_screen(mode: &Mode) -> bool {
    match mode {
        Mode::Log { .. } => true,
        #[cfg(feature = "preview")]
        Mode::Preview => true,
        _ => false,
    }
}

#[cfg(feature = "preview")]
/// The text proof: a dark "sheet of paper" with the page laid out like the PDF.
fn draw_preview(f: &mut Frame, area: Rect, app: &mut App) {
    let lang = app.lang();
    let Some(pv) = &mut app.preview else { return };
    let width = area.width.saturating_sub(4).min(68);
    let paper = Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y,
        width,
        height: area.height,
    };
    let status = pv.status(lang);
    let block = panel(&status, true);
    let inner = block.inner(paper);
    f.render_widget(block, paper);
    // Page margins inside the sheet.
    let content = Rect {
        x: inner.x + 4,
        y: inner.y + 1,
        width: inner.width.saturating_sub(7),
        height: inner.height.saturating_sub(3),
    };
    let w = content.width as usize;
    let page_no = pv.page + 1;
    let muted = Style::default().fg(MUTED);
    let accent = Style::default().fg(ACCENT);

    match &pv.pages[pv.page] {
        Page::Cover {
            series,
            subtitle,
            author,
            meta,
            epigraph,
        } => {
            let mut lines = vec![Line::from(""); (content.height / 3) as usize];
            lines.push(Line::from(Span::styled(
                series.clone(),
                accent.add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                subtitle.clone(),
                Style::default().fg(FG),
            )));
            lines.push(Line::from(Span::styled(
                author.clone(),
                Style::default().fg(FG),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(meta.clone(), muted)));
            if !epigraph.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(""));
                for l in epigraph {
                    lines.push(Line::from(Span::styled(
                        l.trim().to_string(),
                        muted.add_modifier(Modifier::ITALIC),
                    )));
                }
            }
            f.render_widget(Paragraph::new(lines).style(base()), content);
        }
        Page::Colophon(text) => {
            let mut lines = vec![Line::from(""); (content.height / 2) as usize];
            for chunk in wrap(text, w) {
                lines.push(Line::from(Span::styled(chunk.1.trim().to_string(), muted)).centered());
            }
            f.render_widget(Paragraph::new(lines).style(base()), content);
        }
        Page::Contents(entries) => {
            let mut lines = vec![
                Line::from(Span::styled(
                    lang.t("contents"),
                    Style::default().fg(FG).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
            ];
            for (text, heading, page) in entries {
                let indent = if *heading { 0 } else { 2 };
                let num = page.to_string();
                let room = w.saturating_sub(indent + num.chars().count() + 2);
                let label: String = text.chars().take(room).collect();
                let dots =
                    w.saturating_sub(indent + label.chars().count() + num.chars().count() + 2);
                let leader = if *heading {
                    " ".repeat(dots)
                } else {
                    " .".repeat(dots / 2) + &" ".repeat(dots % 2)
                };
                if *heading && lines.len() > 2 {
                    lines.push(Line::from(""));
                }
                let style = if *heading {
                    accent
                } else {
                    Style::default().fg(FG)
                };
                lines.push(Line::from(vec![
                    Span::raw(" ".repeat(indent)),
                    Span::styled(label, style),
                    Span::styled(format!(" {leader} "), Style::default().fg(FAINT)),
                    Span::styled(num, muted),
                ]));
            }
            let max = lines.len().saturating_sub(content.height as usize);
            pv.scroll = pv.scroll.min(max);
            f.render_widget(
                Paragraph::new(lines)
                    .style(base())
                    .scroll((pv.scroll as u16, 0)),
                content,
            );
            page_number(f, inner, page_no);
        }
        Page::Poem {
            title,
            lines: body,
            footer,
        } => {
            let mut lines = vec![
                Line::from(Span::styled(
                    title.clone(),
                    accent.add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled("* * *", accent)),
                Line::from(""),
            ];
            for (lvl, text) in body {
                if text.is_empty() {
                    lines.push(Line::from(""));
                    continue;
                }
                let indent = lvl * 4;
                // Wrapped lines hang under the start of the line, like in the PDF.
                for (i, (_, chunk)) in wrap(text, w.saturating_sub(indent + 2).max(10))
                    .into_iter()
                    .enumerate()
                {
                    let pad = if i == 0 { indent } else { indent + 2 };
                    lines.push(Line::from(Span::styled(
                        format!("{}{}", " ".repeat(pad), chunk.trim_end()),
                        Style::default().fg(FG),
                    )));
                }
            }
            let max = lines.len().saturating_sub(content.height as usize);
            pv.scroll = pv.scroll.min(max);
            f.render_widget(
                Paragraph::new(lines)
                    .style(base())
                    .scroll((pv.scroll as u16, 0)),
                content,
            );
            if pv.scroll < max {
                let r = Rect {
                    x: content.x + content.width.saturating_sub(1),
                    y: content.y + content.height.saturating_sub(1),
                    width: 1,
                    height: 1,
                };
                f.render_widget(
                    Paragraph::new(Span::styled("↓", Style::default().fg(ACCENT))),
                    r,
                );
            }
            let foot = Rect {
                x: content.x,
                y: inner.y + inner.height.saturating_sub(2),
                width: content.width,
                height: 1,
            };
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(footer.clone(), muted)).right_aligned()),
                foot,
            );
            page_number(f, inner, page_no);
        }
    }
}

#[cfg(feature = "preview")]
fn page_number(f: &mut Frame, inner: Rect, n: usize) {
    let r = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1),
        width: inner.width,
        height: 1,
    };
    f.render_widget(
        Paragraph::new(
            Line::from(Span::styled(n.to_string(), Style::default().fg(FAINT))).centered(),
        ),
        r,
    );
}

#[cfg(test)]
mod tests {
    use super::wrap_hint;

    #[test]
    fn hints_wrap_at_separators() {
        let lines = wrap_hint("n new · ⏎ open · / search · q quit", 18);
        assert_eq!(lines, vec!["n new · ⏎ open", "/ search · q quit"]);
        assert_eq!(wrap_hint("↑↓ · Esc", 80), vec!["↑↓ · Esc"]);
    }
}
