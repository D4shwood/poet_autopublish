//! A text proof of the PDF pages, drawn in the terminal: the same covers, poem
//! pages, contents and colophon as the PDF, laid out as plain text on a dark page.

use crate::lang::Lang;
use crate::latex::{self, Ctx};
use crate::poem::Poem;
use crate::volume::Week;

pub enum Page {
    Cover {
        series: String,
        subtitle: String,
        author: String,
        meta: String,
        epigraph: Vec<String>,
    },
    /// (text, is a week heading, page number in this preview)
    Contents(Vec<(String, bool, usize)>),
    Poem {
        title: String,
        /// (indent level, text); an empty text is a stanza break.
        lines: Vec<(usize, String)>,
        footer: String,
    },
    Colophon(String),
}

pub struct Preview {
    pub title: String,
    pub pages: Vec<Page>,
    /// 0-based
    pub page: usize,
    /// Lines scrolled within a long page.
    pub scroll: usize,
}

fn poem_page(p: &Poem, ctx: &Ctx) -> Page {
    let levels = latex::indent_levels(&p.body);
    let mut lines: Vec<(usize, String)> = vec![];
    for (line, lvl) in p.body.iter().zip(levels) {
        if line.trim().is_empty() {
            if lines.last().is_some_and(|(_, t)| !t.is_empty()) {
                lines.push((0, String::new()));
            }
        } else {
            lines.push((lvl, line.trim().to_string()));
        }
    }
    let footer = if p.signature.is_some() {
        latex::footer_text(p, ctx)
    } else if p.draft {
        ctx.lang.t("draft").to_string()
    } else {
        ctx.lang
            .t("unsigned")
            .trim_start_matches("⚠ ")
            .to_lowercase()
    };
    Page::Poem {
        title: p.title.clone(),
        lines,
        footer,
    }
}

fn week_cover(w: &Week, ctx: &Ctx) -> Page {
    Page::Cover {
        series: ctx.series.to_string(),
        subtitle: w.label(ctx.lang),
        author: ctx.author.to_string(),
        meta: format!(
            "{} · {}",
            ctx.lang.volume(w.number),
            ctx.lang.poem_count(w.poems.len())
        ),
        epigraph: w
            .epigraph
            .as_ref()
            .map(|e| e.lines.clone())
            .unwrap_or_default(),
    }
}

impl Preview {
    /// One weekly volume, opened at the page of the poem file `start` if given.
    pub fn week(w: &Week, ctx: &Ctx, start: Option<&str>) -> Preview {
        let mut pages = vec![week_cover(w, ctx)];
        let mut page = 0;
        for p in &w.poems {
            if Some(p.file_name.as_str()) == start {
                page = pages.len();
            }
            pages.push(poem_page(p, ctx));
        }
        pages.push(Page::Colophon(latex::colophon_text(
            &ctx.lang.volume(w.number),
            latex::compiled_date(w.poems.iter(), ctx),
            ctx,
        )));
        Preview {
            title: format!("{} — {}", ctx.series, w.label(ctx.lang)),
            pages,
            page,
            scroll: 0,
        }
    }

    /// The cumulative book: cover, contents, then the weeks newest first.
    pub fn book(weeks: &[Week], ctx: &Ctx) -> Preview {
        let total: usize = weeks.iter().map(|w| w.poems.len()).sum();
        let mut contents = vec![];
        let mut body = vec![];
        // Pages 1 and 2 are the cover and the contents.
        for w in weeks.iter().rev() {
            contents.push((
                format!("{} — {}", ctx.lang.volume(w.number), w.label(ctx.lang)),
                true,
                body.len() + 3,
            ));
            body.push(week_cover(w, ctx));
            for p in &w.poems {
                contents.push((p.title.clone(), false, body.len() + 3));
                body.push(poem_page(p, ctx));
            }
        }
        let mut pages = vec![
            Page::Cover {
                series: ctx.series.to_string(),
                subtitle: ctx.lang.t("all_poems").to_string(),
                author: ctx.author.to_string(),
                meta: ctx.lang.poem_count(total),
                epigraph: vec![],
            },
            Page::Contents(contents),
        ];
        pages.extend(body);
        let date = latex::compiled_date(weeks.iter().flat_map(|w| w.poems.iter()), ctx);
        pages.push(Page::Colophon(latex::colophon_text(
            ctx.lang.t("all_poems"),
            date,
            ctx,
        )));
        Preview {
            title: format!("{} — {}", ctx.series, ctx.lang.t("all_poems")),
            pages,
            page: 0,
            scroll: 0,
        }
    }

    /// A poem that isn't in any volume yet (a draft or an unsigned poem).
    pub fn single(p: &Poem, ctx: &Ctx) -> Preview {
        Preview {
            title: p.title.clone(),
            pages: vec![poem_page(p, ctx)],
            page: 0,
            scroll: 0,
        }
    }

    pub fn turn(&mut self, delta: i32) {
        let last = self.pages.len().saturating_sub(1) as i32;
        self.page = (self.page as i32 + delta).clamp(0, last) as usize;
        self.scroll = 0;
    }

    pub fn status(&self, lang: Lang) -> String {
        let word = match lang {
            Lang::Tr => "sayfa",
            Lang::En => "page",
        };
        format!(
            "{} · {word} {}/{}",
            self.title,
            self.page + 1,
            self.pages.len()
        )
    }
}
