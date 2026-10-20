//! The setup screens: who the user is, where the poems live, what the series is
//! called and whether to rebuild in the background. Shown on the first run, and
//! later from ⚙ Ayarlar with the current answers filled in.

use super::ui::{ACCENT, BG, FAINT, FG, MUTED, WARN};
use crate::authors::{Authors, norm};
use crate::config::{Config, expand};
use crate::install;
use crate::lang::Lang;
use anyhow::Result;
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use std::fs;

const NAME: usize = 0;
const PENS: usize = 1;
const POEMS: usize = 2;
const OUT: usize = 3;
const SERIES: usize = 4;
const SERVICE: usize = 5;
const SUMMARY: usize = 6;
const STEPS: usize = 7;

pub enum Outcome {
    Stay,
    /// Esc on the first screen.
    Cancel,
    /// The answers were confirmed; call `apply`.
    Done,
}

pub struct Setup {
    pub lang: Lang,
    pub first_run: bool,
    step: usize,
    /// name, pen names, poems folder, PDF folder, series
    fields: [String; 5],
    service: bool,
    error: Option<String>,
}

fn t(lang: Lang, tr: &'static str, en: &'static str) -> &'static str {
    match lang {
        Lang::Tr => tr,
        Lang::En => en,
    }
}

impl Setup {
    /// A blank setup with sensible suggestions, for a user starting from zero.
    pub fn first_run() -> Setup {
        let lang = crate::lang::system_lang();
        let (poems, out, series) = match lang {
            Lang::Tr => ("~/Şiirler", "~/Şiirler_PDF", "Haftalık Defter"),
            Lang::En => ("~/poetry", "~/poetry_pdf", "Weekly Notebook"),
        };
        Setup {
            lang,
            first_run: true,
            step: NAME,
            fields: [
                String::new(),
                String::new(),
                poems.into(),
                out.into(),
                series.into(),
            ],
            service: true,
            error: None,
        }
    }

    /// The same screens with the current answers, from ⚙ Ayarlar.
    pub fn from_current(cfg: &Config, authors: &Authors, lang: Lang, service_on: bool) -> Setup {
        let main = authors.main_author();
        Setup {
            lang,
            first_run: false,
            step: NAME,
            fields: [
                if main.name.is_empty() {
                    cfg.author.clone()
                } else {
                    main.name.clone()
                },
                main.pen_names.join(", "),
                cfg.poems_dir.clone(),
                cfg.out_dir.clone(),
                cfg.series.clone(),
            ],
            service: service_on,
            error: None,
        }
    }

    fn pens(&self) -> Vec<String> {
        let mut out: Vec<String> = vec![];
        for p in self.fields[PENS].split(',') {
            let p = p.trim();
            if !p.is_empty()
                && norm(p) != norm(&self.fields[NAME])
                && !out.iter().any(|q| norm(q) == norm(p))
            {
                out.push(p.to_string());
            }
        }
        out
    }

    /// Checks the current screen before moving on.
    fn check(&self) -> Option<&'static str> {
        let l = self.lang;
        let v = self.fields.get(self.step).map(|s| s.trim());
        match self.step {
            NAME if v == Some("") => Some(t(l, "Bir ad yazmalısın.", "Please write a name.")),
            POEMS | SERIES if v == Some("") => {
                Some(t(l, "Boş bırakılamaz.", "This can't be empty."))
            }
            OUT if v == Some("") => Some(t(l, "Boş bırakılamaz.", "This can't be empty.")),
            OUT if expand(self.fields[OUT].trim())
                .starts_with(expand(self.fields[POEMS].trim())) =>
            {
                // Inside the poems folder it would become a "collection" of PDFs.
                Some(t(
                    l,
                    "PDF klasörü şiir klasörünün içinde olamaz.",
                    "The PDF folder can't be inside the poems folder.",
                ))
            }
            _ => None,
        }
    }

    pub fn set_error(&mut self, e: String) {
        self.error = Some(e);
    }

    pub fn paste(&mut self, s: &str) {
        if self.step < SERVICE {
            self.fields[self.step].push_str(s.lines().next().unwrap_or(""));
        }
    }

    pub fn key(&mut self, k: KeyEvent) -> Outcome {
        self.error = None;
        match k.code {
            KeyCode::Esc => {
                if self.step == NAME {
                    return Outcome::Cancel;
                }
                self.step -= 1;
            }
            KeyCode::Enter => {
                if let Some(e) = self.check() {
                    self.error = Some(e.to_string());
                } else if self.step == SUMMARY {
                    return Outcome::Done;
                } else {
                    self.step += 1;
                }
            }
            _ if self.step == SERVICE => match k.code {
                KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::Char(' ') => {
                    self.service = !self.service
                }
                KeyCode::Char(c) if self.lang.yes(c) => self.service = true,
                KeyCode::Char('h' | 'H' | 'n' | 'N') => self.service = false,
                _ => {}
            },
            KeyCode::Backspace if self.step < SERVICE => {
                self.fields[self.step].pop();
            }
            KeyCode::Char(c)
                if self.step < SERVICE
                    && (!k.modifiers.contains(KeyModifiers::CONTROL)
                        || k.modifiers.contains(KeyModifiers::ALT)) =>
            {
                self.fields[self.step].push(c);
            }
            _ => {}
        }
        Outcome::Stay
    }

    /// Writes config.toml and the folder's .poet/ files, creates the folders and sets up (or
    /// stops) the background service. Returns the new config.
    pub fn apply(&self, service_was_on: bool) -> Result<Config> {
        let old = Config::load().ok();
        let old_name = old.as_ref().map(|c| c.author.clone());
        let old_poems = old.as_ref().map(|c| c.poems_dir());
        let mut cfg = old.unwrap_or_default();
        let name = self.fields[NAME].trim().to_string();
        cfg.author = name.clone();
        cfg.poems_dir = self.fields[POEMS].trim().to_string();
        cfg.out_dir = self.fields[OUT].trim().to_string();
        cfg.series = self.fields[SERIES].trim().to_string();
        fs::create_dir_all(cfg.poems_dir())?;
        fs::create_dir_all(cfg.out_dir())?;
        crate::shared::migrate(&cfg.poems_dir(), &crate::config::config_dir())?;
        cfg.save()?;

        // The main author keeps their entry (and other authors theirs); only the
        // name and pen names change. A pen name another author owns is skipped.
        // When the poems folder moved, the authors come along from the old one.
        let from = old_poems
            .filter(|p| crate::shared::dir(p).join(crate::shared::AUTHORS).exists())
            .unwrap_or_else(|| cfg.poems_dir());
        let mut authors = Authors::load(&from, old_name.as_deref().unwrap_or(&name));
        let main = authors.main;
        authors.list[main].name = name;
        authors.list[main].pen_names.clear();
        for pen in self.pens() {
            authors.add_pen_name(main, &pen);
        }
        authors.save(&cfg.poems_dir())?;

        if self.service {
            install::install()?;
        } else if service_was_on {
            install::disable_service()?;
        }
        Ok(cfg)
    }
}

pub fn draw(f: &mut Frame, area: Rect, s: &Setup) {
    let l = s.lang;
    let w = area.width.saturating_sub(4).min(84);
    let h = area.height.saturating_sub(2).min(20);
    let r = Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    };
    f.render_widget(Clear, r);

    let title = if s.first_run {
        t(l, "Hoş geldin", "Welcome")
    } else {
        t(l, "Ayarlar", "Settings")
    };
    let step_word = t(l, "adım", "step");
    let (question, help) = match s.step {
        NAME => (
            t(l, "Adın nedir?", "What's your name?"),
            t(
                l,
                "Şiirlerini imzalarken kullandığın gerçek adın. Bu adla imzalanan şiirlerin yayımlanır.",
                "The real name you sign your poems with. Poems signed with it get published.",
            ),
        ),
        PENS => (
            t(l, "Rumuzların var mı?", "Do you have pen names?"),
            t(
                l,
                "Virgülle ayır, örneğin: K.M Ozan, D.Y. Bu adlarla imzaladıkların da senin sayılır. Boş bırakabilirsin.",
                "Separate them with commas, e.g. K.M Ozan, D.Y. Poems signed with them count as yours too. You can leave this empty.",
            ),
        ),
        POEMS => (
            t(
                l,
                "Şiirlerin hangi klasörde?",
                "Which folder holds your poems?",
            ),
            t(
                l,
                "Her şiir bu klasörde bir metin dosyasıdır; alt klasörler koleksiyon olur. Yoksa oluşturulur.",
                "Each poem is a text file in this folder; subfolders become collections. It is created if missing.",
            ),
        ),
        OUT => (
            t(l, "PDF'ler nereye yazılsın?", "Where should the PDFs go?"),
            t(
                l,
                "Şiir klasörünün dışında bir klasör olmalı. Yoksa oluşturulur.",
                "A folder outside the poems folder. It is created if missing.",
            ),
        ),
        SERIES => (
            t(
                l,
                "Haftalık serinin adı ne olsun?",
                "What should the weekly series be called?",
            ),
            t(
                l,
                "Her hafta bir cilt olur; bu ad kapaklarda büyük harflerle yazar.",
                "Every week becomes a volume; this name is printed large on the covers.",
            ),
        ),
        SERVICE => (
            t(
                l,
                "Arka planda kendiliğinden derlensin mi?",
                "Rebuild automatically in the background?",
            ),
            t(
                l,
                "Evet dersen bir servis kurulur: şiir kaydettikçe PDF'ler güncellenir. Ayrıca `poet` komutu ve menü girdisi eklenir.",
                "If yes, a service is installed: PDFs update whenever you save a poem. The `poet` command and a menu entry are added too.",
            ),
        ),
        _ => (
            t(l, "Hazır. Kaydedilsin mi?", "All set. Save?"),
            t(
                l,
                "Sonra ⚙ Ayarlar'dan değiştirebilirsin.",
                "You can change these later from ⚙ Settings.",
            ),
        ),
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!(" poetautopublish · {title}"),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("   {step_word} {}/{STEPS}", s.step + 1),
                Style::default().fg(FAINT),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!(" {question}"),
            Style::default().fg(FG).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(format!(" {help}"), Style::default().fg(MUTED))),
        Line::from(""),
    ];

    if s.step < SERVICE {
        lines.push(Line::from(vec![
            Span::styled(" › ", Style::default().fg(ACCENT)),
            Span::styled(s.fields[s.step].clone(), Style::default().fg(FG)),
            Span::styled("▏", Style::default().fg(ACCENT)),
        ]));
    } else if s.step == SERVICE {
        let (yes, no) = (t(l, " Evet ", " Yes "), t(l, " Hayır ", " No "));
        let on = Style::default().fg(BG).bg(ACCENT);
        let off = Style::default().fg(MUTED);
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(yes, if s.service { on } else { off }),
            Span::raw("   "),
            Span::styled(no, if s.service { off } else { on }),
        ]));
    } else {
        let pens = s.pens();
        let rows = [
            (t(l, "Ad", "Name"), s.fields[NAME].clone()),
            (
                t(l, "Rumuzlar", "Pen names"),
                if pens.is_empty() {
                    "—".into()
                } else {
                    pens.join(", ")
                },
            ),
            (
                t(l, "Şiir klasörü", "Poems folder"),
                s.fields[POEMS].clone(),
            ),
            (t(l, "PDF klasörü", "PDF folder"), s.fields[OUT].clone()),
            (t(l, "Seri", "Series"), s.fields[SERIES].clone()),
            (
                t(l, "Servis", "Service"),
                t(
                    l,
                    if s.service { "evet" } else { "hayır" },
                    if s.service { "yes" } else { "no" },
                )
                .to_string(),
            ),
        ];
        for (k, v) in rows {
            lines.push(Line::from(vec![
                Span::styled(format!("   {k:<14}"), Style::default().fg(MUTED)),
                Span::styled(v, Style::default().fg(FG)),
            ]));
        }
    }

    if let Some(e) = &s.error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(" {e}"),
            Style::default().fg(WARN),
        )));
    }

    let back = if s.step == NAME {
        if s.first_run {
            t(l, "Esc çık", "Esc quit")
        } else {
            t(l, "Esc vazgeç", "Esc cancel")
        }
    } else {
        t(l, "Esc geri", "Esc back")
    };
    let next = if s.step == SUMMARY {
        t(l, "⏎ kaydet", "⏎ save")
    } else if s.step == SERVICE {
        t(l, "← → seç · ⏎ ileri", "← → choose · ⏎ next")
    } else {
        t(l, "⏎ ileri", "⏎ next")
    };
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(" {next} · {back}"),
        Style::default().fg(FAINT),
    )));

    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(FG).bg(BG))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(ACCENT))
                    .style(Style::default().bg(BG)),
            ),
        r,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pen_names_are_cleaned() {
        let mut s = Setup::first_run();
        s.fields[NAME] = "Deniz Yalın".into();
        s.fields[PENS] = " K.M Ozan , , k.m. ozan, Deniz Yalın, D.Y ".into();
        assert_eq!(s.pens(), vec!["K.M Ozan", "D.Y"]);
    }

    #[test]
    fn pdf_folder_outside_poems() {
        let mut s = Setup::first_run();
        s.step = OUT;
        s.fields[POEMS] = "~/Şiirler".into();
        s.fields[OUT] = "~/Şiirler/pdf".into();
        assert!(s.check().is_some());
        s.fields[OUT] = "~/Şiirler_PDF".into();
        assert!(s.check().is_none());
    }
}
