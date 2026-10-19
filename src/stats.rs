use crate::lang::Lang;
use crate::poem::Poem;
use crate::volume::Week;
use chrono::{Duration, NaiveDate, Timelike};
use std::collections::BTreeSet;

pub struct Stats {
    pub poems: usize,
    pub lines: usize,
    pub words: usize,
    /// (week label, poem count), oldest first
    pub per_week: Vec<(String, u64)>,
    /// Consecutive days with a signed poem, ending today or yesterday.
    pub streak: usize,
    pub longest_streak: usize,
    /// Signed poems per hour of the day.
    pub by_hour: [u64; 24],
}

pub fn compute(weeks: &[Week], lang: Lang, today: NaiveDate) -> Stats {
    let poems: Vec<&Poem> = weeks.iter().flat_map(|w| w.poems.iter()).collect();
    let mut by_hour = [0u64; 24];
    let mut days = BTreeSet::new();
    for p in &poems {
        let sig = p.signature.as_ref().unwrap();
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
        per_week: weeks
            .iter()
            .map(|w| (lang.date_short(w.first_date), w.poems.len() as u64))
            .collect(),
        streak,
        longest_streak: longest,
        by_hour,
    }
}
