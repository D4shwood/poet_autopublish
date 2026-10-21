use crate::lang::Lang;
use crate::poem::Poem;
use crate::variants::{LineDiff, Seg};
use crate::volume::{Collection, Week};
use chrono::{NaiveDate, NaiveDateTime};
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Copy)]
pub struct Ctx<'a> {
    pub template: &'a str,
    pub series: &'a str,
    /// The name on covers: the main author's real name, or their pen names.
    pub author: &'a str,
    /// The main author's real name; a footer signed otherwise shows its name.
    pub real_name: &'a str,
    pub lang: Lang,
    /// file name -> latest revision date
    pub revisions: &'a HashMap<String, NaiveDate>,
}

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\textbackslash{}"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '$' => out.push_str("\\$"),
            '&' => out.push_str("\\&"),
            '#' => out.push_str("\\#"),
            '%' => out.push_str("\\%"),
            '_' => out.push_str("\\_"),
            '~' => out.push_str("\\textasciitilde{}"),
            '^' => out.push_str("\\textasciicircum{}"),
            '\t' => out.push_str("    "),
            c => out.push(c),
        }
    }
    out
}

/// Leading whitespace width; a tab counts as 4 spaces.
fn lead_width(line: &str) -> usize {
    line.chars()
        .take_while(|c| c.is_whitespace())
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

/// Indent level per line. The distinct indent widths of a poem are ranked,
/// so 4/8 spaces and 6/12 spaces both become steps 1 and 2.
pub fn indent_levels(body: &[String]) -> Vec<usize> {
    let widths: BTreeSet<usize> = body
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| lead_width(l))
        .filter(|w| *w > 0)
        .collect();
    let rank: Vec<usize> = widths.into_iter().collect();
    body.iter()
        .map(|l| {
            let w = lead_width(l);
            if w == 0 || l.trim().is_empty() {
                0
            } else {
                rank.iter().position(|r| *r == w).unwrap() + 1
            }
        })
        .collect()
}

/// Body lines as `\pl` commands, with a `\stanza` before each stanza.
pub fn body_tex(body: &[String]) -> String {
    let levels = indent_levels(body);
    let mut stanzas: Vec<Vec<(usize, &str)>> = vec![vec![]];
    for (line, lvl) in body.iter().zip(levels) {
        if line.trim().is_empty() {
            if !stanzas.last().unwrap().is_empty() {
                stanzas.push(vec![]);
            }
        } else {
            stanzas.last_mut().unwrap().push((lvl, line.trim()));
        }
    }
    let mut out = String::new();
    for (i, st) in stanzas.iter().filter(|s| !s.is_empty()).enumerate() {
        let keep = st.len().min(8);
        if i == 0 {
            out.push_str(&format!("\\needspace{{{keep}\\baselineskip}}\n"));
        } else {
            out.push_str(&format!("\\stanza{{{keep}}}\n"));
        }
        for (lvl, text) in st {
            out.push_str(&format!("\\pl{{{}em}}{{{}}}\n", lvl * 2, escape(text)));
        }
    }
    out
}

pub fn footer_text(p: &Poem, ctx: &Ctx) -> String {
    let Some(sig) = &p.signature else {
        return String::new();
    };
    let mut parts: Vec<String> = vec![];
    // A pen name is shown; the real name isn't, as it is on the cover.
    if crate::authors::norm(&sig.author) != crate::authors::norm(ctx.real_name) {
        parts.push(sig.author.clone());
    }
    // An approximate date only claims its week: "Ekim 2026 · 2. hafta".
    // An undated signature shows no date at all.
    if let Some(date) = sig.date {
        parts.push(if sig.approx {
            ctx.lang.month_week(date)
        } else {
            ctx.lang.date_long(date)
        });
    }
    if let Some(t) = sig.time {
        parts.push(t.format("%H.%M").to_string());
    }
    if let Some(r) = ctx.revisions.get(&p.file_name) {
        parts.push(format!("{} {}", ctx.lang.t("rev"), ctx.lang.date_short(*r)));
    }
    parts.join(" · ")
}

/// "ilk yayın: Mürekkep #0", from the text after the signature's date.
pub fn publication_text(p: &Poem, ctx: &Ctx) -> Option<String> {
    let publication = p.signature.as_ref()?.publication.as_ref()?;
    Some(format!("{} {publication}", ctx.lang.t("first_published")))
}

fn poem_page(p: &Poem, ctx: &Ctx, toc: bool) -> String {
    let mut out = String::from("\\newpage\n");
    if toc {
        out.push_str(&format!(
            "\\addcontentsline{{toc}}{{subsection}}{{{}}}\n",
            escape(&p.title)
        ));
    }
    out.push_str(&format!("\\poemtitle{{{}}}\n", escape(&p.title)));
    out.push_str(&body_tex(&p.body));
    // The publication goes on a smaller line under the date.
    let mut footer = escape(&footer_text(p, ctx));
    if let Some(publication) = publication_text(p, ctx) {
        footer.push_str(&format!(
            "\\\\[2pt]{{\\footnotesize\\itshape {}}}",
            escape(&publication)
        ));
    }
    out.push_str(&format!("\\poemfooter{{{footer}}}\n"));
    out
}

/// The date of the newest content in a set of poems (signature or revision).
pub fn compiled_date<'p>(poems: impl Iterator<Item = &'p Poem>, ctx: &Ctx) -> Option<NaiveDate> {
    poems
        .filter_map(|p| {
            let sig = p.signature.as_ref()?.date;
            match (ctx.revisions.get(&p.file_name), sig) {
                (Some(r), Some(s)) => Some((*r).max(s)),
                (Some(r), None) => Some(*r),
                (None, s) => s,
            }
        })
        .max()
}

fn epigraph_tex(w: &Week) -> String {
    match &w.epigraph {
        Some(e) => e
            .lines
            .iter()
            .map(|l| escape(l.trim()))
            .collect::<Vec<_>>()
            .join("\\\\\n"),
        None => String::new(),
    }
}

fn week_cover(w: &Week, ctx: &Ctx) -> String {
    format!(
        "\\cover{{{}}}{{{}}}{{{}}}{{{} · {}}}{{{}}}\n",
        escape(ctx.series),
        escape(&w.label(ctx.lang)),
        escape(ctx.author),
        ctx.lang.volume(w.number),
        ctx.lang.poem_count(w.poems.len()),
        epigraph_tex(w),
    )
}

/// "Contra Noir · Cilt 2 · derlendi 19 Ekim 2026 · Liberation Serif · poetautopublish"
pub fn colophon_text(label: &str, date: Option<NaiveDate>, ctx: &Ctx) -> String {
    let mut parts = vec![ctx.series.to_string()];
    if !label.is_empty() {
        parts.push(label.to_string());
    }
    if let Some(d) = date {
        parts.push(format!(
            "{} {}",
            ctx.lang.t("compiled"),
            ctx.lang.date_long(d)
        ));
    }
    parts.push("Liberation Serif".into());
    parts.push("poetautopublish".into());
    parts.join(" · ")
}

fn colophon(label: &str, date: Option<NaiveDate>, ctx: &Ctx) -> String {
    format!(
        "\\colophon{{{}}}\n",
        escape(&colophon_text(label, date, ctx))
    )
}

fn fill(ctx: &Ctx, title: &str, body: &str) -> String {
    // PDF metadata is written as a UTF-16BE hex string with a byte-order mark.
    let meta = |s: &str| {
        let mut hex = String::from("FEFF");
        for u in s.encode_utf16() {
            hex.push_str(&format!("{u:04X}"));
        }
        hex
    };
    ctx.template
        .replace("%%TITLE%%", &meta(title))
        .replace("%%AUTHOR%%", &meta(ctx.author))
        .replace("%%SUBJECT%%", &meta(ctx.series))
        .replace("%%CONTENTS%%", ctx.lang.t("contents"))
        .replace("%%BODY%%", body)
}

pub fn week_doc(w: &Week, ctx: &Ctx) -> String {
    let mut body = week_cover(w, ctx);
    for p in &w.poems {
        body.push_str(&poem_page(p, ctx, false));
    }
    body.push_str(&colophon(
        &ctx.lang.volume(w.number),
        compiled_date(w.poems.iter(), ctx),
        ctx,
    ));
    let title = format!("{} — {}", ctx.series, w.label(ctx.lang));
    fill(ctx, &title, &body)
}

/// The cumulative book: newest week first, with a table of contents.
/// One poem's path from its first draft, for the book's variants chapter.
pub struct Variant<'a> {
    pub poem: &'a Poem,
    /// When the first draft was saved.
    pub first: Option<NaiveDateTime>,
    /// How many different versions there were.
    pub versions: usize,
    pub diff: Vec<LineDiff>,
}

/// Struck-out and underlined text without extra packages, defined here so a
/// user's own template needs nothing new.
const VARIANT_MACROS: &str = "\\makeatletter
\\newlength{\\poet@varlen}
\\providecommand{\\gone}[1]{{\\color{muted}\\settowidth{\\poet@varlen}{#1}\\rlap{\\rule[0.5ex]{\\poet@varlen}{0.5pt}}#1}}
\\providecommand{\\added}[1]{{\\settowidth{\\poet@varlen}{#1}\\rlap{\\rule[-0.45ex]{\\poet@varlen}{0.4pt}}#1}}
\\makeatother
";

/// A run of text in a macro, its outer spaces kept outside so TeX keeps them.
fn marked(cmd: &str, t: &str) -> String {
    let core = t.trim();
    if core.is_empty() {
        return if t.is_empty() {
            String::new()
        } else {
            " ".into()
        };
    }
    let lead = if t.starts_with(char::is_whitespace) {
        " "
    } else {
        ""
    };
    let trail = if t.ends_with(char::is_whitespace) {
        " "
    } else {
        ""
    };
    format!("{lead}\\{cmd}{{{}}}{trail}", escape(core))
}

fn variant_line(line: &LineDiff) -> String {
    line.iter()
        .enumerate()
        .map(|(i, s)| {
            // The comparison isn't about indentation: lines start flush.
            let lstrip = |t: &str| {
                if i == 0 {
                    t.trim_start().to_string()
                } else {
                    t.to_string()
                }
            };
            match s {
                Seg::Same(t) => escape(&lstrip(t)),
                Seg::Gone(t) => marked("gone", &lstrip(t)),
                Seg::New(t) => marked("added", &lstrip(t)),
            }
        })
        .collect()
}

/// "Varyantlar": each poem from its first draft to its final form, removed
/// words struck out and new ones underlined.
pub fn variants_chapter(variants: &[Variant], ctx: &Ctx) -> String {
    if variants.is_empty() {
        return String::new();
    }
    let mut out = String::from(VARIANT_MACROS);
    out.push_str(&format!(
        "\\newpage\n\\addcontentsline{{toc}}{{section}}{{{}}}\n",
        ctx.lang.t("variants")
    ));
    for v in variants {
        out.push_str("\\newpage\n");
        out.push_str(&format!(
            "\\addcontentsline{{toc}}{{subsection}}{{{}}}\n",
            escape(&v.poem.title)
        ));
        out.push_str(&format!("\\poemtitle{{{}}}\n", escape(&v.poem.title)));
        // The title line itself is the heading; the rest are the poem's lines.
        let mut started = false;
        for line in v.diff.iter().skip(1) {
            let text: String = line
                .iter()
                .map(|s| match s {
                    Seg::Same(t) | Seg::Gone(t) | Seg::New(t) => t.as_str(),
                })
                .collect();
            if text.trim().is_empty() {
                if started {
                    out.push_str("\\stanza{1}\n");
                }
                continue;
            }
            started = true;
            out.push_str(&format!("\\pl{{0em}}{{{}}}\n", variant_line(line)));
        }
        let first = v
            .first
            .map(|t| format!("{} {}", ctx.lang.date_long(t.date()), t.format("%H.%M")))
            .unwrap_or_default();
        out.push_str(&format!(
            "\\poemfooter{{{} {} · {} {}}}\n",
            ctx.lang.t("first_draft"),
            escape(&first),
            v.versions,
            ctx.lang.t("version")
        ));
    }
    out
}

pub fn book_doc(weeks: &[Week], variants: &[Variant], ctx: &Ctx) -> String {
    let total: usize = weeks.iter().map(|w| w.poems.len()).sum();
    let mut body = format!(
        "\\cover{{{}}}{{{}}}{{{}}}{{{}}}{{}}\n",
        escape(ctx.series),
        ctx.lang.t("all_poems"),
        escape(ctx.author),
        ctx.lang.poem_count(total),
    );
    body.push_str("\\tableofcontents\n");
    for w in weeks.iter().rev() {
        body.push_str("\\newpage\n");
        body.push_str(&format!(
            "\\addcontentsline{{toc}}{{section}}{{{} — {}}}\n",
            ctx.lang.volume(w.number),
            escape(&w.label(ctx.lang))
        ));
        body.push_str(&week_cover(w, ctx));
        for p in &w.poems {
            body.push_str(&poem_page(p, ctx, true));
        }
    }
    body.push_str(&variants_chapter(variants, ctx));
    let date = compiled_date(weeks.iter().flat_map(|w| w.poems.iter()), ctx);
    body.push_str(&colophon(ctx.lang.t("all_poems"), date, ctx));
    let title = format!("{} — {}", ctx.series, ctx.lang.t("all_poems"));
    fill(ctx, &title, &body)
}

/// A collection outside the weekly series: the folder name is the title,
/// and the colophon and PDF subject name the collection instead of the series.
pub fn collection_doc(c: &Collection, ctx: &Ctx) -> String {
    let ctx = Ctx {
        series: &c.name,
        ..*ctx
    };
    let mut body = format!(
        "\\cover{{{}}}{{{}}}{{}}{{{}}}{{}}\n",
        escape(&c.name),
        escape(ctx.author),
        ctx.lang.poem_count(c.poems.len()),
    );
    if c.poems.len() > 1 {
        body.push_str("\\tableofcontents\n");
    }
    for p in &c.poems {
        body.push_str(&poem_page(p, &ctx, true));
    }
    body.push_str(&colophon("", compiled_date(c.poems.iter(), &ctx), &ctx));
    fill(&ctx, &c.name, &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn escaping() {
        assert_eq!(
            escape("50% & #1 {x} a_b ~ ^ \\"),
            "50\\% \\& \\#1 \\{x\\} a\\_b \\textasciitilde{} \\textasciicircum{} \\textbackslash{}"
        );
        assert_eq!(escape("na'hür; İş"), "na'hür; İş");
    }

    #[test]
    fn indentation_is_ranked() {
        let body = v(&["a", "      b", "            c", "d"]);
        assert_eq!(indent_levels(&body), vec![0, 1, 2, 0]);
        let body = v(&["a", "    b", "        c", "\tt"]);
        assert_eq!(indent_levels(&body), vec![0, 1, 2, 1]);
    }

    #[test]
    fn variant_lines() {
        let d = crate::variants::diff(
            "Liman\n\nkıyıda bekleyen martılar\n",
            "Liman\n\nkıyıda susan martılar & %\n",
        );
        assert_eq!(
            variant_line(&d[2]),
            "kıyıda \\gone{bekleyen}\\added{susan} martılar \\added{\\& \\%}"
        );
    }

    #[test]
    fn stanzas() {
        let tex = body_tex(&v(&["a", "b", "", "", "    c"]));
        assert_eq!(
            tex,
            "\\needspace{2\\baselineskip}\n\\pl{0em}{a}\n\\pl{0em}{b}\n\\stanza{1}\n\\pl{2em}{c}\n"
        );
    }
}
