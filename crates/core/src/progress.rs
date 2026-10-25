//! Telling the user what a long step does while ES-DE waits for heimspiel, such as a download or
//! the installation of a Port. ES-DE shows only a black screen meanwhile.

use std::cell::RefCell;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

pub trait Progress {
    /// A long step starts: `action`, such as "Downloading", on `game`, with its cover if known.
    fn start(&self, game: &str, cover: Option<&Path>, action: &str);
    /// How much of the step is done.
    fn advance(&self, done_bytes: u64, total_bytes: u64);
    /// The steps are over and the game starts next.
    fn finish(&self);
    /// Shows that the game cannot start, and why, until the user has seen it.
    fn fail(&self, message: &str);
}

/// For callers that show nothing.
impl Progress for () {
    fn start(&self, _: &str, _: Option<&Path>, _: &str) {}
    fn advance(&self, _: u64, _: u64) {}
    fn finish(&self) {}
    fn fail(&self, _: &str) {}
}

/// One line of JSON on the loading window's standard input. The window closes when its input
/// ends, except after a [`Message::Fail`], which stays until the user dismisses it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    Start {
        game: String,
        cover: Option<PathBuf>,
        action: String,
    },
    Advance {
        done_bytes: u64,
        total_bytes: u64,
    },
    Fail {
        message: String,
    },
}

/// A loading window above ES-DE, run as a separate program that reads [`Message`]s; opened on
/// the first message and closed by [`Progress::finish`] or when dropped.
pub struct Window {
    command: Vec<String>,
    open: RefCell<Option<Open>>,
}

struct Open {
    child: Child,
    input: Option<BufWriter<ChildStdin>>,
    last_advance: Option<Instant>,
}

/// Progress updates sent to the window at most this often.
const ADVANCE_INTERVAL: Duration = Duration::from_millis(100);

impl Window {
    /// `command` starts the window program, e.g. `["heimspiel-app", "progress"]`.
    pub fn new(command: Vec<String>) -> Window {
        Window {
            command,
            open: RefCell::new(None),
        }
    }

    fn send(&self, message: &Message) {
        let mut open = self.open.borrow_mut();
        if open.is_none() {
            let child = Command::new(&self.command[0])
                .args(&self.command[1..])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn();
            match child {
                Ok(mut child) => {
                    let input = child.stdin.take().map(BufWriter::new);
                    *open = Some(Open {
                        child,
                        input,
                        last_advance: None,
                    });
                }
                Err(e) => eprintln!("heimspiel: cannot open the loading window: {e}"),
            }
        }
        let Some(Open {
            input: Some(input), ..
        }) = open.as_mut()
        else {
            return;
        };
        // A window the user closed early is no reason to stop the game.
        let line = serde_json::to_string(message).expect("messages serialise");
        let _ = writeln!(input, "{line}").and_then(|()| input.flush());
    }

    /// Ends the window's input and waits until it has closed.
    fn close(&self) {
        if let Some(mut open) = self.open.borrow_mut().take() {
            drop(open.input.take());
            let _ = open.child.wait();
        }
    }
}

impl Progress for Window {
    fn start(&self, game: &str, cover: Option<&Path>, action: &str) {
        self.send(&Message::Start {
            game: game.to_owned(),
            cover: cover.map(Path::to_path_buf),
            action: action.to_owned(),
        });
    }

    fn advance(&self, done_bytes: u64, total_bytes: u64) {
        {
            let mut open = self.open.borrow_mut();
            let Some(open) = open.as_mut() else {
                return;
            };
            let due = open
                .last_advance
                .is_none_or(|last| last.elapsed() >= ADVANCE_INTERVAL);
            if !due && done_bytes < total_bytes {
                return;
            }
            open.last_advance = Some(Instant::now());
        }
        self.send(&Message::Advance {
            done_bytes,
            total_bytes,
        });
    }

    fn finish(&self) {
        self.close();
    }

    fn fail(&self, message: &str) {
        self.send(&Message::Fail {
            message: message.to_owned(),
        });
        self.close();
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.close();
    }
}
