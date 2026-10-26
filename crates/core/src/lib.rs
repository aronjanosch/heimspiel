//! The heimspiel core: the Library, the Sources that fill it and the Outputs it is written into.
//!
//! Terms follow `GLOSSARY.md` at the repository root. The CLI and the app only call into this crate.

pub mod cache;
pub mod catalog;
pub mod config;
pub mod desktop;
pub mod dump;
pub mod fetch;
pub mod hook;
pub mod launch;
pub mod library;
pub mod n64;
pub mod output;
pub mod port;
mod process;
pub mod progress;
pub mod proton;
pub mod provision;
pub mod quiver;
pub mod romm;
pub mod source;
pub mod start;
pub mod sync;
