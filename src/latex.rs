use crate::lang::Lang;
use crate::poem::Poem;
use crate::volume::Week;
use chrono::NaiveDate;
use std::collections::{BTreeSet, HashMap};

pub struct Ctx<'a> {
    pub template: &'a str,
    pub series: &'a str,
    pub author: &'a str,
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
    // An approximate date only claims its week: "Ekim 2026 · 2. hafta".
    let mut s = if sig.approx {
        ctx.lang.month_week(sig.date)
    } else {
        ctx.lang.date_long(sig.date)
    };
    if let Some(t) = sig.time {
        s.push_str(&format!(" · {}", t.format("%H.%M")));
    }
    if let Some(r) = ctx.revisions.get(&p.file_name) {
        s.push_str(&format!(
            " · {} {}",
            ctx.lang.t("rev"),
            ctx.lang.date_short(*r)
        ));
    }
    s
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
    out.push_str(&format!(
        "\\poemfooter{{{}}}\n",
        escape(&footer_text(p, ctx))
    ));
    out
}

/// The date of the newest content in a set of poems (signature or revision).
pub fn compiled_date<'p>(poems: impl Iterator<Item = &'p Poem>, ctx: &Ctx) -> Option<NaiveDate> {
    poems
        .filter_map(|p| {
            let sig = p.signature.as_ref()?.date;
            Some(match ctx.revisions.get(&p.file_name) {
                Some(r) if *r > sig => *r,
                _ => sig,
            })
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
    let mut parts = vec![ctx.series.to_string(), label.to_string()];
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
pub fn book_doc(weeks: &[Week], ctx: &Ctx) -> String {
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
    let date = compiled_date(weeks.iter().flat_map(|w| w.poems.iter()), ctx);
    body.push_str(&colophon(ctx.lang.t("all_poems"), date, ctx));
    let title = format!("{} — {}", ctx.series, ctx.lang.t("all_poems"));
    fill(ctx, &title, &body)
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
    fn stanzas() {
        let tex = body_tex(&v(&["a", "b", "", "", "    c"]));
        assert_eq!(
            tex,
            "\\needspace{2\\baselineskip}\n\\pl{0em}{a}\n\\pl{0em}{b}\n\\stanza{1}\n\\pl{2em}{c}\n"
        );
    }
}
