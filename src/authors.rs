//! Authors and their pen names, kept in `<poems folder>/.poet/authors.toml`.
//!
//! The main author (the config's `author`) publishes under their name or any of
//! their pen names. Poems signed by other authors are tracked but never published.

use crate::poem::Poem;
use crate::shared;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Author {
    /// The real name; may be empty when only pen names are known.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pen_names: Vec<String>,
}

impl Author {
    /// The real name if there is one, else the pen names.
    pub fn label(&self) -> String {
        if !self.name.trim().is_empty() {
            self.name.trim().to_string()
        } else {
            self.pen_names.join(" / ")
        }
    }

    pub fn signs_as(&self, signature: &str) -> bool {
        let s = norm(signature);
        (!self.name.trim().is_empty() && norm(&self.name) == s)
            || self.pen_names.iter().any(|p| norm(p) == s)
    }
}

#[derive(Debug, Clone)]
pub struct Authors {
    pub list: Vec<Author>,
    /// Index of the main author in `list`.
    pub main: usize,
}

/// Names compare ignoring case, spacing and dots: "K.M. Ozan" = "k.m ozan",
/// "Ece T." = "Ece T".
pub fn norm(s: &str) -> String {
    // Circumflexes are spelling variants too: "Âli" = "Ali".
    s.replace('.', " ")
        .replace(['â', 'Â'], "a")
        .replace(['î', 'Î'], "i")
        .replace(['û', 'Û'], "u")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

impl Authors {
    /// Builds the table, making sure the main author (`main_name`) is in it.
    pub fn new(mut list: Vec<Author>, main_name: &str) -> Authors {
        let main = match list.iter().position(|a| a.signs_as(main_name)) {
            Some(i) => i,
            None => {
                list.insert(
                    0,
                    Author {
                        name: main_name.trim().to_string(),
                        pen_names: vec![],
                    },
                );
                0
            }
        };
        Authors { list, main }
    }

    pub fn load(poems_dir: &Path, main_name: &str) -> Authors {
        Authors::new(shared::load_authors(poems_dir), main_name)
    }

    pub fn save(&self, poems_dir: &Path) -> Result<()> {
        shared::save_authors(poems_dir, &self.list)
    }

    /// Which author a signature belongs to.
    pub fn find(&self, signature: &str) -> Option<usize> {
        self.list.iter().position(|a| a.signs_as(signature))
    }

    /// Signed by the main author under their name or a pen name.
    pub fn is_main(&self, signature: &str) -> bool {
        self.find(signature) == Some(self.main)
    }

    pub fn main_author(&self) -> &Author {
        &self.list[self.main]
    }

    /// Adds a pen name to an author, unless some author already signs that way.
    pub fn add_pen_name(&mut self, author: usize, pen: &str) -> bool {
        let pen = pen.trim();
        if pen.is_empty() || self.find(pen).is_some() {
            return false;
        }
        self.list[author].pen_names.push(pen.to_string());
        true
    }

    pub fn remove_pen_name(&mut self, author: usize, pen: usize) -> Option<String> {
        let pens = &mut self.list.get_mut(author)?.pen_names;
        (pen < pens.len()).then(|| pens.remove(pen))
    }

    /// Adds another author with a real name, a first pen name, or both.
    /// Fails when both are empty or either is already someone's signature.
    pub fn add_author(&mut self, name: &str, pen: &str) -> bool {
        let (name, pen) = (name.trim(), pen.trim());
        let taken = |s: &str| !s.is_empty() && self.find(s).is_some();
        if (name.is_empty() && pen.is_empty()) || taken(name) || taken(pen) {
            return false;
        }
        self.list.push(Author {
            name: name.to_string(),
            pen_names: if pen.is_empty() {
                vec![]
            } else {
                vec![pen.to_string()]
            },
        });
        true
    }

    /// Removes another author; the main author stays.
    pub fn remove_author(&mut self, i: usize) -> Option<Author> {
        if i == self.main || i >= self.list.len() {
            return None;
        }
        if i < self.main {
            self.main -= 1;
        }
        Some(self.list.remove(i))
    }

    /// Author indexes, the main author first.
    pub fn order(&self) -> Vec<usize> {
        let mut order = vec![self.main];
        order.extend((0..self.list.len()).filter(|i| *i != self.main));
        order
    }

    /// Signatures no author claims, in the spelling first seen.
    pub fn unclaimed(&self, counts: &SignatureCounts) -> Vec<String> {
        counts
            .values()
            .filter(|(shown, _)| self.find(shown).is_none())
            .map(|(shown, _)| shown.clone())
            .collect()
    }
}

/// Poems per signature, keyed by the normalised name, with the spelling first seen.
pub type SignatureCounts = BTreeMap<String, (String, usize)>;

pub fn signature_counts<'a>(poems: impl IntoIterator<Item = &'a Poem>) -> SignatureCounts {
    let mut out = SignatureCounts::new();
    for p in poems {
        if let Some(sig) = &p.signature {
            let e = out
                .entry(norm(&sig.author))
                .or_insert_with(|| (sig.author.clone(), 0));
            e.1 += 1;
        }
    }
    out
}

pub fn count_of(counts: &SignatureCounts, name: &str) -> usize {
    counts.get(&norm(name)).map(|(_, n)| *n).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Authors {
        Authors::new(
            vec![
                Author {
                    name: "Deniz Yalın".into(),
                    pen_names: vec!["K.M Ozan".into()],
                },
                Author {
                    name: "".into(),
                    pen_names: vec!["Gölge".into(), "B.".into()],
                },
            ],
            "Deniz Yalın",
        )
    }

    #[test]
    fn editing() {
        let mut a = table();
        assert!(!a.add_author("", ""));
        assert!(!a.add_author("Yeni", "k.m. ozan"), "pen name already taken");
        assert!(a.add_author("Ece Tan", ""));
        assert_eq!(a.remove_author(a.main), None, "the main author stays");
        assert_eq!(a.remove_pen_name(1, 0).as_deref(), Some("Gölge"));
        assert_eq!(a.remove_author(1).unwrap().pen_names, vec!["B."]);
        assert_eq!(a.list[a.main].name, "Deniz Yalın");
        assert_eq!(a.order(), vec![0, 1]);
    }

    #[test]
    fn signatures_map_to_authors() {
        let a = table();
        assert!(a.is_main("deniz  yalın"));
        assert!(a.is_main("K.M ozan"));
        assert!(a.is_main("K.M. Ozan"));
        assert_eq!(norm("Ece T."), norm("Ece T"));
        assert_eq!(norm("Âdem Nur"), norm("Adem Nur"));
        assert_eq!(a.find("Gölge"), Some(1));
        assert!(!a.is_main("Gölge"));
        assert_eq!(a.find("Biri"), None);
        assert_eq!(a.list[1].label(), "Gölge / B.");
    }

    #[test]
    fn main_author_is_added_when_missing() {
        let a = Authors::new(vec![], "Deniz Yalın");
        assert_eq!(a.main, 0);
        assert!(a.is_main("Deniz Yalın"));
    }

    #[test]
    fn pen_names_are_unique() {
        let mut a = table();
        assert!(a.add_pen_name(0, "Yeni Rumuz"));
        assert!(!a.add_pen_name(1, "yeni rumuz"));
        assert!(!a.add_pen_name(1, "  "));
    }
}
