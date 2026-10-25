// Each test crate uses only part of this helper.
#![allow(dead_code)]

//! A tiny HTTP/1.1 server for tests: answers from a handler and records every request.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone)]
pub struct Request {
    /// Path including the query string.
    pub path: String,
    pub authorization: Option<String>,
    pub range: Option<String>,
}

pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Response {
    pub fn json(body: &str) -> Response {
        Response {
            status: 200,
            body: body.as_bytes().to_vec(),
        }
    }

    pub fn status(status: u16) -> Response {
        Response {
            status,
            body: Vec::new(),
        }
    }
}

pub struct FakeServer {
    pub url: String,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl FakeServer {
    pub fn start(handler: impl Fn(&Request) -> Response + Send + Sync + 'static) -> FakeServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let handler = Arc::new(handler);
        let recorded = requests.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                let (handler, recorded) = (handler.clone(), recorded.clone());
                thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut stream = stream;
                    // Keep-alive: serve requests until the client closes the connection.
                    loop {
                        let mut line = String::new();
                        if reader.read_line(&mut line).unwrap_or(0) == 0 {
                            return;
                        }
                        let path = line.split_whitespace().nth(1).unwrap_or("").to_owned();
                        let (mut authorization, mut range) = (None, None);
                        loop {
                            let mut header = String::new();
                            reader.read_line(&mut header).unwrap();
                            let header = header.trim_end();
                            if header.is_empty() {
                                break;
                            }
                            if let Some((name, value)) = header.split_once(':') {
                                let value = Some(value.trim().to_owned());
                                if name.eq_ignore_ascii_case("authorization") {
                                    authorization = value;
                                } else if name.eq_ignore_ascii_case("range") {
                                    range = value;
                                }
                            }
                        }
                        let request = Request {
                            path,
                            authorization,
                            range,
                        };
                        let response = handler(&request);
                        recorded.lock().unwrap().push(request);
                        let head = format!(
                            "HTTP/1.1 {} X\r\nContent-Length: {}\r\n\r\n",
                            response.status,
                            response.body.len()
                        );
                        if stream.write_all(head.as_bytes()).is_err()
                            || stream.write_all(&response.body).is_err()
                        {
                            return;
                        }
                    }
                });
            }
        });
        FakeServer { url, requests }
    }

    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

/// A PS4 `param.sfo` holding the given text values, as found in a Dump's `sce_sys/`.
pub fn param_sfo(values: &[(&str, &str)]) -> Vec<u8> {
    let mut keys = Vec::new();
    let mut data = Vec::new();
    let mut index = Vec::new();
    for (key, value) in values {
        let len = value.len() as u32 + 1;
        let max = len.next_multiple_of(4);
        index.extend((keys.len() as u16).to_le_bytes());
        index.extend(0x0204u16.to_le_bytes());
        index.extend(len.to_le_bytes());
        index.extend(max.to_le_bytes());
        index.extend((data.len() as u32).to_le_bytes());
        keys.extend(key.bytes().chain([0]));
        data.extend(value.bytes());
        data.resize(data.len() + (max - len + 1) as usize, 0);
    }
    keys.resize(keys.len().next_multiple_of(4), 0);
    let key_table = 20 + index.len() as u32;
    let data_table = key_table + keys.len() as u32;
    let mut sfo = b"\0PSF".to_vec();
    sfo.extend(0x101u32.to_le_bytes());
    sfo.extend(key_table.to_le_bytes());
    sfo.extend(data_table.to_le_bytes());
    sfo.extend((values.len() as u32).to_le_bytes());
    sfo.extend(index);
    sfo.extend(keys);
    sfo.extend(data);
    sfo
}

/// A PS4 Dump folder `<parent>/<title_id>` with `param.sfo` and the given (empty) files.
pub fn ps4_dump(parent: &std::path::Path, title_id: &str, files: &[&str]) -> std::path::PathBuf {
    let dir = parent.join(title_id);
    std::fs::create_dir_all(dir.join("sce_sys")).unwrap();
    std::fs::write(
        dir.join("sce_sys/param.sfo"),
        param_sfo(&[
            ("APP_VER", "01.00"),
            ("TITLE", "Test Collection™"),
            ("TITLE_ID", title_id),
        ]),
    )
    .unwrap();
    for file in files {
        std::fs::write(dir.join(file), b"").unwrap();
    }
    dir
}
