use crate::lang::Lang;
use crate::poem::Poem;
use crate::volume::Week;
use chrono::{Datelike, Duration, NaiveDate, Timelike};
use std::collections::{BTreeMap, BTreeSet};

pub struct Stats {
    pub poems: usize,
    pub lines: usize,
    pub words: usize,
    /// (label, poem count), oldest first: weeks for the series, years for a collection.
    pub per_group: Vec<(String, u64)>,
    /// Consecutive days with a signed poem, ending today or yesterday.
    pub streak: usize,
    pub longest_streak: usize,
    /// Signed poems per hour of the day.
    pub by_hour: [u64; 24],
}

/// Poems per week of the series, oldest first.
pub fn per_week(weeks: &[Week], lang: Lang) -> Vec<(String, u64)> {
    weeks
        .iter()
        .map(|w| (lang.date_short(w.first_date), w.poems.len() as u64))
        .collect()
}

/// Poems per year, oldest first; a collection has no weeks.
pub fn per_year(poems: &[&Poem]) -> Vec<(String, u64)> {
    let mut years: BTreeMap<i32, u64> = BTreeMap::new();
    for p in poems {
        if let Some(sig) = &p.signature {
            *years.entry(sig.date.year()).or_default() += 1;
        }
    }
    years.into_iter().map(|(y, n)| (y.to_string(), n)).collect()
}

/// Statistics over published (signed) poems.
pub fn compute(poems: &[&Poem], per_group: Vec<(String, u64)>, today: NaiveDate) -> Stats {
    let mut by_hour = [0u64; 24];
    let mut days = BTreeSet::new();
    for p in poems {
        let Some(sig) = &p.signature else { continue };
        // An approximate day can't count toward a streak.
        if !sig.approx {
            days.insert(sig.date);
        }
        if let Some(t) = sig.time {
            by_hour[t.hour() as usize] += 1;
        }
    }

    let mut longest = 0;
    let mut run = 0;
    let mut prev: Option<NaiveDate> = None;
    for d in &days {
        run = match prev {
            Some(p) if *d == p + Duration::days(1) => run + 1,
            _ => 1,
        };
        longest = longest.max(run);
        prev = Some(*d);
    }
    let mut streak = 0;
    let mut day = if days.contains(&today) {
        today
    } else {
        today - Duration::days(1)
    };
    while days.contains(&day) {
        streak += 1;
        day -= Duration::days(1);
    }

    Stats {
        poems: poems.len(),
        lines: poems.iter().map(|p| p.line_count()).sum(),
        words: poems.iter().map(|p| p.word_count()).sum(),
        per_group,
        streak,
        longest_streak: longest,
        by_hour,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn years() {
        let a = Poem::parse(Path::new("a"), "A\n\nx\n- S. 01.01.2019\n");
        let b = Poem::parse(Path::new("b"), "B\n\nx\n- S. 05.06.2019\n");
        let c = Poem::parse(Path::new("c"), "C\n\nx\n- S. 02.02.2017\n");
        assert_eq!(
            per_year(&[&a, &b, &c]),
            vec![("2017".to_string(), 1), ("2019".to_string(), 2)]
        );
    }
}
