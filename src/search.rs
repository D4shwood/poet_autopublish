use crate::history;
use crate::poem::Poem;
use chrono::NaiveDateTime;
use std::collections::HashSet;
use std::fs;

#[derive(Debug, Clone)]
pub struct Hit {
    pub file_name: String,
    pub title: String,
    pub line: String,
    /// Set when the match is only in an older version of the poem.
    pub snapshot: Option<NaiveDateTime>,
}

fn fold(s: &str) -> String {
    // Turkish-aware enough for searching: İ/I fold like i/ı would to a reader.
    s.chars()
        .flat_map(|c| match c {
            'İ' => vec!['i'],
            'I' => vec!['ı'],
            c => c.to_lowercase().collect(),
        })
        .collect()
}

/// Case-insensitive search over titles and lines of every poem, then their history.
pub fn search(poems: &[Poem], query: &str) -> Vec<Hit> {
    let q = fold(query.trim());
    if q.is_empty() {
        return vec![];
    }
    let mut hits = vec![];
    let mut seen: HashSet<(String, String)> = HashSet::new();
    for p in poems {
        for line in std::iter::once(&p.title).chain(p.body.iter()) {
            if fold(line).contains(&q)
                && seen.insert((p.file_name.clone(), line.trim().to_string()))
            {
                hits.push(Hit {
                    file_name: p.file_name.clone(),
                    title: p.title.clone(),
                    line: line.trim().to_string(),
                    snapshot: None,
                });
            }
        }
    }
    for p in poems {
        for snap in history::list(&p.file_name).iter().rev() {
            let Ok(text) = fs::read_to_string(&snap.path) else {
                continue;
            };
            for line in text.lines() {
                if fold(line).contains(&q)
                    && seen.insert((p.file_name.clone(), line.trim().to_string()))
                {
                    hits.push(Hit {
                        file_name: p.file_name.clone(),
                        title: p.title.clone(),
                        line: line.trim().to_string(),
                        snapshot: Some(snap.at),
                    });
                }
            }
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn turkish_case() {
        let p = Poem::parse(Path::new("a"), "İshaneler\n\nIşık ve ışık\n");
        assert_eq!(search(std::slice::from_ref(&p), "ishane").len(), 1);
        assert_eq!(search(std::slice::from_ref(&p), "ışık").len(), 1);
    }
}
