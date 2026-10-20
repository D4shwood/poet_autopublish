//! Authors and their pen names, kept in `<poems folder>/.poet/authors.toml`.
//!
//! The main author (the config's `author`) publishes under their name or any of
//! their pen names. Poems signed by other authors are tracked but never published.

use crate::shared;
use anyhow::Result;
use serde::{Deserialize, Serialize};
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
