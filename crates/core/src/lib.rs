//! The heimspiel core: the Library, the Sources that fill it and the Outputs it is written into.
//!
//! Terms follow `GLOSSARY.md` at the repository root. The CLI and the app only call into this crate.

pub mod cache;
pub mod config;
pub mod desktop;
pub mod fetch;
pub mod hook;
pub mod launch;
pub mod library;
pub mod output;
pub mod port;
mod process;
pub mod provision;
pub mod quiver;
pub mod romm;
pub mod source;
pub mod sync;
