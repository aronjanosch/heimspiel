//! The heimspiel core: the Library, the Sources that fill it and the Outputs it is written into.
//!
//! Terms follow `GLOSSARY.md` at the repository root. The CLI and the app only call into this crate.

pub mod config;
pub mod desktop;
pub mod library;
pub mod output;
pub mod romm;
pub mod source;
pub mod sync;
