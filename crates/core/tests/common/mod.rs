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
                        let mut authorization = None;
                        loop {
                            let mut header = String::new();
                            reader.read_line(&mut header).unwrap();
                            let header = header.trim_end();
                            if header.is_empty() {
                                break;
                            }
                            if let Some((name, value)) = header.split_once(':')
                                && name.eq_ignore_ascii_case("authorization")
                            {
                                authorization = Some(value.trim().to_owned());
                            }
                        }
                        let request = Request {
                            path,
                            authorization,
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
