//! The shared core: poem files, signatures, authors, weeks and stats.
//! The terminal app is built on top of it, and the phone app can use it alone.

pub mod authors;
pub mod compose;
pub mod config;
pub mod history;
pub mod lang;
pub mod poem;
pub mod prosody;
pub mod search;
pub mod shared;
pub mod stats;
pub mod volume;
