use crate::lang::Lang;
use crate::poem::{Epigraph, Poem, Status};
use chrono::{Datelike, NaiveDate, NaiveTime};
use std::collections::BTreeMap;

/// One weekly volume: the published poems of one Monday–Sunday ISO week.
#[derive(Debug, Clone)]
pub struct Week {
    /// Effective date of the first poem; names the volume.
    pub first_date: NaiveDate,
    /// Oldest first.
    pub poems: Vec<Poem>,
    /// "Cilt N", counted from the first week of the series.
    pub number: usize,
    pub epigraph: Option<Epigraph>,
    pub file_name: String,
    /// ISO week, "2026-W41"; the key for a custom name.
    pub key: String,
    /// A name the user gave the week, replacing "5 Ekim Haftası".
    pub name: Option<String>,
}

impl Week {
    /// The custom name if there is one, else "5 Ekim Haftası" / "Week of 5 October".
    pub fn label(&self, lang: Lang) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| lang.week_label(self.first_date))
    }
}

/// Groups published poems into weeks, oldest week first.
pub fn group(
    poems: &[Poem],
    epigraphs: &[Epigraph],
    author: &str,
    cutoff: NaiveTime,
    series: &str,
) -> Vec<Week> {
    let mut by_week: BTreeMap<(i32, u32), Vec<Poem>> = BTreeMap::new();
    for p in poems {
        if p.status(author) != Status::Published {
            continue;
        }
        let sig = p.signature.as_ref().unwrap();
        let iso = sig.effective_date(cutoff).iso_week();
        by_week
            .entry((iso.year(), iso.week()))
            .or_default()
            .push(p.clone());
    }

    let mut weeks: Vec<Week> = by_week
        .into_iter()
        .enumerate()
        .map(|(i, ((year, week), mut poems))| {
            poems.sort_by_key(|p| {
                let s = p.signature.as_ref().unwrap();
                (s.sort_key(), p.file_name.clone())
            });
            // Named after the earliest day of the week's poems, including
            // approximate ones, even though those are listed last.
            let first_date = poems
                .iter()
                .map(|p| p.signature.as_ref().unwrap().effective_date(cutoff))
                .min()
                .unwrap();
            let epigraph = epigraphs
                .iter()
                .filter(|e| {
                    let w = e.date.iso_week();
                    (w.year(), w.week()) == (year, week)
                })
                .max_by_key(|e| e.date)
                .cloned();
            Week {
                first_date,
                poems,
                number: i + 1,
                epigraph,
                file_name: String::new(),
                key: format!("{year}-W{week:02}"),
                name: None,
            }
        })
        .collect();

    // contra_noir_DD_MM_week.pdf; the year is added only if two weeks would collide.
    let prefix = slug(series);
    let base = |w: &Week| {
        format!(
            "{}_{:02}_{:02}_week",
            prefix,
            w.first_date.day(),
            w.first_date.month()
        )
    };
    let names: Vec<String> = weeks.iter().map(base).collect();
    for (i, w) in weeks.iter_mut().enumerate() {
        let clash = names.iter().filter(|n| **n == names[i]).count() > 1;
        w.file_name = if clash {
            format!("{}_{}.pdf", names[i], w.first_date.year())
        } else {
            format!("{}.pdf", names[i])
        };
    }
    weeks
}

pub fn book_file_name(series: &str) -> String {
    format!("{}_all.pdf", slug(series))
}

/// "Contra Noir" -> "contra_noir", folding Turkish letters to ASCII.
pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        let c = match c {
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ı' | 'I' | 'İ' | 'i' => 'i',
            'ö' | 'Ö' => 'o',
            'ş' | 'Ş' => 's',
            'ü' | 'Ü' => 'u',
            'â' | 'Â' => 'a',
            'î' | 'Î' => 'i',
            'û' | 'Û' => 'u',
            c => c.to_ascii_lowercase(),
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    let out = out.trim_end_matches('_').to_string();
    if out.is_empty() { "poems".into() } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn poem(name: &str, sig: &str) -> Poem {
        Poem::parse(Path::new(name), &format!("{name}\n\nmısra\n{sig}\n"))
    }

    #[test]
    fn grouping_and_names() {
        let cutoff = NaiveTime::from_hms_opt(5, 0, 0).unwrap();
        let poems = vec![
            poem("Liman", "- Deniz Yalın. 13.10.2026 - 22.58"),
            poem("Kum", "- Deniz Yalın. 08.10.2026 - 13.38"),
            poem("Rüzgârda", "- Deniz Yalın. 09.10.2026 - 00.17"),
            poem("Gece", "- Deniz Yalın. 12.10.2026 - 00.30"), // Monday night -> Sunday
            poem("B10", "B00"),
        ];
        let weeks = group(&poems, &[], "Deniz Yalın", cutoff, "Contra Noir");
        assert_eq!(weeks.len(), 2);
        assert_eq!(weeks[0].file_name, "contra_noir_08_10_week.pdf");
        assert_eq!(weeks[0].number, 1);
        let titles: Vec<_> = weeks[0].poems.iter().map(|p| p.title.as_str()).collect();
        assert_eq!(titles, vec!["Kum", "Rüzgârda", "Gece"]);
        assert_eq!(weeks[1].file_name, "contra_noir_13_10_week.pdf");
        assert_eq!(weeks[1].number, 2);
        assert_eq!(weeks[1].key, "2026-W42");
        assert_eq!(weeks[1].label(Lang::Tr), "13 Ekim Haftası");

        // An approximate date goes to its week, after the exactly dated poems.
        let mut more: Vec<Poem> = poems
            .iter()
            .filter(|p| p.title != "Kum")
            .cloned()
            .collect();
        more.push(poem("Yaklaşık", "-Deniz Yalın ~08.10.2026"));
        let weeks = group(&more, &[], "Deniz Yalın", cutoff, "Contra Noir");
        let titles: Vec<_> = weeks[0].poems.iter().map(|p| p.title.as_str()).collect();
        assert_eq!(titles, vec!["Rüzgârda", "Gece", "Yaklaşık"]);
        assert_eq!(weeks[0].file_name, "contra_noir_08_10_week.pdf");
    }

    #[test]
    fn new_year_week() {
        let cutoff = NaiveTime::from_hms_opt(5, 0, 0).unwrap();
        // 31.12.2026 (Thu) and 02.01.2027 (Sat) are both ISO week 2026-W53.
        let poems = vec![
            poem("A", "- S. 31.12.2026 - 20.00"),
            poem("B", "- S. 02.01.2027 - 20.00"),
        ];
        let weeks = group(&poems, &[], "S", cutoff, "Contra Noir");
        assert_eq!(weeks.len(), 1);
        assert_eq!(
            weeks[0].first_date,
            NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()
        );
    }

    #[test]
    fn slugs() {
        assert_eq!(slug("Contra Noir"), "contra_noir");
        assert_eq!(slug("Gece Şiirleri!"), "gece_siirleri");
    }
}
