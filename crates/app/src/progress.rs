//! `heimspiel-app progress`: the loading window above ES-DE. Reads [`Message`]s as JSON lines from
//! standard input and closes when the input ends; after a failure it stays until the user
//! dismisses it or it times out.

use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use heimspiel_core::progress::Message;
use serde_json::json;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// The page navigates here to close the window.
const CLOSE_URL: &str = "heimspiel://close";

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let page = Arc::new(Page::default());
            let handle = app.handle().clone();
            let loaded = page.clone();
            let window =
                WebviewWindowBuilder::new(app, "progress", WebviewUrl::App("progress.html".into()))
                    .title("heimspiel")
                    .fullscreen(true)
                    .decorations(false)
                    .always_on_top(true)
                    .focused(true)
                    .on_navigation(move |url| {
                        if url.as_str() == CLOSE_URL {
                            handle.exit(0);
                        }
                        url.scheme() != "heimspiel"
                    })
                    .on_page_load(move |window, payload| {
                        if payload.event() == PageLoadEvent::Finished {
                            loaded.loaded(&window);
                        }
                    })
                    .build()?;
            let handle = app.handle().clone();
            thread::spawn(move || read_input(&handle, &window, &page));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the heimspiel loading window");
}

fn read_input(handle: &AppHandle, window: &WebviewWindow, page: &Page) {
    let mut failed = false;
    for line in io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let Ok(message) = serde_json::from_str::<Message>(&line) else {
            eprintln!("heimspiel-app: ignoring {line}");
            continue;
        };
        failed |= matches!(message, Message::Fail { .. });
        page.run(window, script(message));
    }
    if !failed {
        handle.exit(0);
    }
}

/// Scripts for the page, held back until it has loaded.
struct Page {
    /// `None` once the page has loaded.
    pending: Mutex<Option<Vec<String>>>,
}

impl Default for Page {
    fn default() -> Page {
        Page {
            pending: Mutex::new(Some(Vec::new())),
        }
    }
}

impl Page {
    fn run(&self, window: &WebviewWindow, script: String) {
        let mut pending = self.pending.lock().unwrap();
        match pending.as_mut() {
            Some(scripts) => scripts.push(script),
            None => {
                let _ = window.eval(script);
            }
        }
    }

    fn loaded(&self, window: &WebviewWindow) {
        if let Some(scripts) = self.pending.lock().unwrap().take() {
            for script in scripts {
                let _ = window.eval(script);
            }
        }
    }
}

/// The call to the page's `show` that displays `message`; a cover goes in as a data URL.
fn script(message: Message) -> String {
    let value = match message {
        Message::Start {
            game,
            cover,
            action,
        } => json!({
            "type": "start",
            "game": game,
            "action": action,
            "cover": cover.as_deref().and_then(data_url),
        }),
        other => serde_json::to_value(other).expect("messages serialise"),
    };
    format!("show({value})")
}

fn data_url(image: &Path) -> Option<String> {
    let mime = match image.extension()?.to_str()?.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => return None,
    };
    let bytes = fs::read(image).ok()?;
    Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
}
