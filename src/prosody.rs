//! Turkish verse craft: syllable meter (hece ölçüsü) and rhyme (kafiye, redif).
//!
//! Turkish syllables are counted reliably by vowels: one vowel, one syllable.
//! A line "fits" a meter when its syllables add up and every pause (durak)
//! falls between two words.

/// Vowels, including the circumflexed ones and both capital i's.
const VOWELS: &str = "aeıioöuüâîûAEIİOÖUÜÂÎÛ";

pub fn syllables(word: &str) -> usize {
    word.chars().filter(|c| VOWELS.contains(*c)).count()
}

/// A line's syllable count, and the counts at each word boundary: the places a
/// pause can fall.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub total: usize,
    pub breaks: Vec<usize>,
}

pub fn line_shape(line: &str) -> Shape {
    let mut total = 0;
    let mut breaks = vec![];
    for w in line.split_whitespace() {
        let n = syllables(w);
        if n > 0 {
            total += n;
            breaks.push(total);
        }
    }
    Shape { total, breaks }
}

/// A meter: its syllables and how they are grouped ("7+7" is [7, 7]).
#[derive(Debug, Clone, PartialEq)]
pub struct Meter {
    pub total: usize,
    pub parts: Vec<usize>,
    /// Lines that fit it, out of the lines that have words.
    pub fits: usize,
    pub of: usize,
}

impl Meter {
    /// "7+7", or "11" when the pauses don't line up.
    pub fn label(&self) -> String {
        self.parts
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join("+")
    }

    pub fn fits(&self, line: &str) -> bool {
        fits(&line_shape(line), self.total, &self.parts)
    }
}

/// The meters of Turkish syllabic verse, most common grouping first.
const METERS: &[&[usize]] = &[
    &[7, 7],
    &[6, 5],
    &[4, 4, 3],
    &[4, 4],
    &[5, 3],
    &[4, 3],
    &[3, 4],
    &[8, 8],
    &[4, 4, 4, 3],
    &[8, 7],
    &[6, 6],
    &[5, 5],
    &[3, 3],
];

fn fits(shape: &Shape, total: usize, parts: &[usize]) -> bool {
    if shape.total != total {
        return false;
    }
    let mut at = 0;
    parts[..parts.len().saturating_sub(1)].iter().all(|p| {
        at += p;
        shape.breaks.contains(&at)
    })
}

/// The meter most lines keep, if at least half of them do; None is free verse.
/// When the syllables agree but the pauses don't, the meter has no grouping.
pub fn meter<S: AsRef<str>>(lines: &[S]) -> Option<Meter> {
    let shapes: Vec<Shape> = lines
        .iter()
        .map(|l| line_shape(l.as_ref()))
        .filter(|s| s.total > 0)
        .collect();
    let of = shapes.len();
    if of < 2 {
        return None;
    }
    let count =
        |total: usize, parts: &[usize]| shapes.iter().filter(|s| fits(s, total, parts)).count();
    let best = METERS
        .iter()
        .map(|parts| {
            let total = parts.iter().sum();
            (count(total, parts), total, parts.to_vec())
        })
        // Most fitting lines; on a tie the earlier (more common) grouping.
        .fold(None::<(usize, usize, Vec<usize>)>, |best, c| match best {
            Some(b) if b.0 >= c.0 => Some(b),
            _ => Some(c),
        });
    if let Some((n, total, parts)) = best
        && n * 2 >= of
        && n > 0
    {
        return Some(Meter {
            total,
            parts,
            fits: n,
            of,
        });
    }
    // The same count without a common pause: "11" rather than "6+5".
    let mut totals: Vec<usize> = shapes.iter().map(|s| s.total).collect();
    totals.sort_unstable();
    let (total, n) = totals
        .chunk_by(|a, b| a == b)
        .map(|g| (g[0], g.len()))
        .max_by_key(|(t, n)| (*n, std::cmp::Reverse(*t)))?;
    (n * 2 >= of && n > 1).then(|| Meter {
        total,
        parts: vec![total],
        fits: n,
        of,
    })
}

/// How a line rhymes with others in the poem.
#[derive(Debug, Clone, PartialEq)]
pub struct Rhyme {
    /// The group, `a`, `b`, `c`… in order of appearance.
    pub letter: char,
    /// The line ends in the same word(s) as the others: a redif.
    pub redif: bool,
}

/// Lowercase, Turkish-style (I → ı, İ → i), letters only.
fn fold(word: &str) -> String {
    word.chars()
        .flat_map(|c| match c {
            'I' => vec!['ı'],
            'İ' => vec!['i'],
            c => c.to_lowercase().collect(),
        })
        .filter(|c| c.is_alphabetic())
        .collect()
}

fn common_suffix(a: &str, b: &str) -> usize {
    a.chars()
        .rev()
        .zip(b.chars().rev())
        .take_while(|(x, y)| x == y)
        .count()
}

/// The last word, and the word before it (for the rhyme under a redif).
fn ending(line: &str) -> Option<(String, String)> {
    let mut words = line
        .split_whitespace()
        .map(fold)
        .filter(|w| !w.is_empty())
        .rev();
    let last = words.next()?;
    Some((last, words.next().unwrap_or_default()))
}

/// Rhymes are endings shared by two or more lines: the same last word is a
/// redif, two or more matching final letters a kafiye. Lines that rhyme with
/// nothing get None.
pub fn rhymes<S: AsRef<str>>(lines: &[S]) -> Vec<Option<Rhyme>> {
    let ends: Vec<Option<String>> = lines
        .iter()
        .map(|l| ending(l.as_ref()).map(|e| e.0))
        .collect();
    // Each line's group, named by its first line.
    let mut group: Vec<Option<usize>> = vec![None; lines.len()];
    let mut redif = vec![false; lines.len()];
    // First the redifs: the same last word on two or more lines.
    for i in 0..ends.len() {
        let Some(w) = &ends[i] else { continue };
        if let Some(j) = (0..i).find(|&j| ends[j].as_ref() == Some(w)) {
            let g = group[j].unwrap_or(j);
            group[j] = Some(g);
            group[i] = Some(g);
            redif[i] = true;
            redif[j] = true;
        }
    }
    // Then the other lines, by a shared ending of two or more letters.
    for i in 0..ends.len() {
        let Some(w) = &ends[i] else { continue };
        if redif[i] {
            continue;
        }
        let earlier = (0..i)
            .find(|&j| !redif[j] && ends[j].as_ref().is_some_and(|o| common_suffix(w, o) >= 2));
        if let Some(j) = earlier {
            let g = group[j].unwrap_or(j);
            group[j] = Some(g);
            group[i] = Some(g);
        }
    }
    // Letters in order of first appearance.
    let mut letters: Vec<(usize, char)> = vec![];
    group
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let g = (*g)?;
            let letter = match letters.iter().find(|(id, _)| *id == g) {
                Some((_, c)) => *c,
                None => {
                    let c = (b'a' + (letters.len() % 26) as u8) as char;
                    letters.push((g, c));
                    c
                }
            };
            Some(Rhyme {
                letter,
                redif: redif[i],
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counting() {
        assert_eq!(syllables("rüzgâr"), 2);
        assert_eq!(syllables("Ankara'dan"), 4);
        assert_eq!(line_shape("IŞIK İNER").total, 4);
        assert_eq!(
            line_shape("Gece, ağır — bir rüzgâr!").breaks,
            vec![2, 4, 5, 7]
        );
    }

    #[test]
    fn seven_plus_seven() {
        let poem = [
            "Gece ağır bir rüzgâr eser uzak kıyıdan",
            "Bir kuş konar dallara eser serin rüzgârlar",
            "Deniz yine sessizdi",
        ];
        let m = meter(&poem).unwrap();
        assert_eq!((m.label().as_str(), m.fits, m.of), ("7+7", 2, 3));
        assert!(!m.fits(poem[2]));
    }

    #[test]
    fn six_plus_five() {
        let poem = [
            "Akşam iner yavaş köyün üstüne",
            "Kuşlar susar bütün dağlar uyurken",
        ];
        assert_eq!(meter(&poem).unwrap().label(), "6+5");
    }

    #[test]
    fn free_verse() {
        let poem = [
            "Kısa",
            "bu satır biraz daha uzun olmalı",
            "orta boy bir satır",
        ];
        assert_eq!(meter(&poem), None);
    }

    #[test]
    fn rhyme_and_redif() {
        let poem = [
            "Akşam iner yollarda",
            "Bir ses kalır kollarda",
            "Rüzgâr gelir bana",
            "Bir kuş gider bana",
            "Yalnız bir satır",
        ];
        // A word repeated at the end is a redif even after a looser rhyme.
        let later = ["Kapıda bekler rüzgâr", "Bir avuç su ver", "İzin ver"];
        let r = rhymes(&later);
        assert_eq!(r[1].as_ref().map(|r| r.redif), Some(true));
        assert_eq!(
            r[1].as_ref().map(|r| r.letter),
            r[2].as_ref().map(|r| r.letter)
        );
        let marks: Vec<Option<(char, bool)>> = rhymes(&poem)
            .into_iter()
            .map(|r| r.map(|r| (r.letter, r.redif)))
            .collect();
        assert_eq!(
            marks,
            vec![
                Some(('a', false)),
                Some(('a', false)),
                Some(('b', true)),
                Some(('b', true)),
                None
            ]
        );
    }
}
