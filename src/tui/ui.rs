use super::app::{App, Confirm, Item, Mode, Watching};
use super::writer::Editor;
use crate::poem::Status;
use crate::stats;
use chrono::{Datelike, Local};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui_image::StatefulImage;
use std::fs;
use std::sync::atomic::Ordering;

pub const BG: Color = Color::Rgb(0x15, 0x15, 0x15);
pub const FG: Color = Color::Rgb(0xe6, 0xe6, 0xe6);
pub const MUTED: Color = Color::Rgb(0x88, 0x88, 0x88);
pub const FAINT: Color = Color::Rgb(0x4a, 0x4a, 0x4a);
pub const ACCENT: Color = Color::Rgb(0xd8, 0xcf, 0xc0);
pub const WARN: Color = Color::Rgb(0xc8, 0x8a, 0x5a);

fn base() -> Style {
    Style::default().fg(FG).bg(BG)
}

fn panel(title: &str, focused: bool) -> Block<'_> {
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
    if let Mode::Writer(ed) = &mut app.mode {
        draw_writer(f, area, ed, app.lib.lang);
        return;
    }

    let [header, body, log, keys] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(7),
        Constraint::Length(1),
    ])
    .areas(area);
    draw_header(f, header, app);

    match &app.mode {
        Mode::Stats => draw_stats(f, body, app),
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
    if !matches!(app.mode, Mode::Log { .. }) {
        draw_log(f, log, app, 0, false);
    }

    let hint = match &app.mode {
        Mode::Confirm(Confirm::Force) => app.t("confirm_force").to_string(),
        Mode::Confirm(Confirm::Trash(p)) => {
            format!(
                "{} — {}",
                p.file_name().unwrap_or_default().to_string_lossy(),
                app.t("confirm_trash")
            )
        }
        Mode::WatcherMenu => format!("{}: {}", app.t("watcher"), app.t("watcher_menu")),
        Mode::Search { .. } => "⏎ · ↑↓ · Esc".into(),
        Mode::History { .. } | Mode::Stats | Mode::Log { .. } => "↑↓ · Esc".into(),
        Mode::Preview => "← → · Esc".into(),
        _ => match &app.flash {
            Some((m, _)) => m.clone(),
            None => app.t("keys_main").into(),
        },
    };
    let style = if matches!(app.mode, Mode::Confirm(_) | Mode::WatcherMenu) {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(MUTED)
    };
    f.render_widget(
        Paragraph::new(Span::styled(format!(" {hint}"), style)),
        keys,
    );
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
            app.cfg.series.clone(),
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
            format!(
                "  {}  ({})",
                app.lang().week_label(w.first_date),
                w.poems.len()
            )
        }
        Item::All => format!("  {}  ({})", app.t("all_poems"), app.count(item)),
        Item::Unsigned => format!("{}  ({})", app.t("unsigned"), app.count(item)),
        Item::Drafts => format!("{}  ({})", app.t("drafts"), app.count(item)),
        Item::Stats => app.t("stats").into(),
    }
}

fn draw_left(f: &mut Frame, area: Rect, app: &App) {
    let focused =
        !app.focus_right && matches!(app.mode, Mode::Main | Mode::Confirm(_) | Mode::WatcherMenu);
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
        Mode::Preview => {
            let title = match &app.preview {
                Some(p) => format!(
                    "{} · {}/{}",
                    p.pdf.file_name().unwrap_or_default().to_string_lossy(),
                    p.page,
                    p.pages
                ),
                None => String::new(),
            };
            let block = panel(&title, true);
            let inner = block.inner(area);
            f.render_widget(block, area);
            if let Some(p) = &mut app.preview {
                f.render_stateful_widget(StatefulImage::default(), inner, &mut p.image);
            }
        }
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
                Some(s) => {
                    let mut w = s.date.format("%d.%m").to_string();
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
            let tag = match p.status(&app.cfg.author) {
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
    let s = stats::compute(&app.lib.weeks, app.lang(), Local::now().date_naive());
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
    let max = s.per_week.iter().map(|(_, n)| *n).max().unwrap_or(1).max(1);
    let bar_w = weeks.width.saturating_sub(18) as u64;
    let rows = weeks.height.saturating_sub(2) as usize;
    let lines: Vec<Line> = s
        .per_week
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
        Paragraph::new(lines).block(panel(app.t("per_week"), false)),
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

fn draw_writer(f: &mut Frame, area: Rect, ed: &mut Editor, lang: crate::lang::Lang) {
    let [page, status] = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(area);
    let width = (page.width.saturating_sub(8) as usize).min(76);
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

    let cy = text_area.y as usize + top_pad + cur_row - first;
    if cy < (text_area.y + text_area.height) as usize {
        f.set_cursor_position(Position::new(text_area.x + cur_col as u16, cy as u16));
    }

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
    let tw = if ed.typewriter { " · ⌨" } else { "" };
    let msg = if ed.confirm_sign {
        lang.t("confirm_sign").to_string()
    } else {
        ed.message.clone().unwrap_or_default()
    };
    let line = Line::from(vec![
        Span::styled(
            format!(
                " {} {} · {} {} · {} {} · {} · {} · {}{}",
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
            ),
            Style::default().fg(FAINT),
        ),
        Span::styled(format!("   {msg}"), Style::default().fg(ACCENT)),
    ]);
    f.render_widget(Clear, status);
    f.render_widget(Paragraph::new(line).style(base()), status);
    let keys = Line::from(Span::styled(
        format!("{} ", lang.t("writer_keys")),
        Style::default().fg(FAINT),
    ))
    .right_aligned();
    if !ed.confirm_sign && ed.message.is_none() {
        f.render_widget(Paragraph::new(keys), status);
    }
}
