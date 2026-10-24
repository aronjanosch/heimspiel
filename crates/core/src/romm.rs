//! A small synchronous RomM client (ADR 0010): one keep-alive connection, retries with backoff on
//! busy servers, a minimum interval between media downloads, and a token that never leaves the
//! `Authorization` header.

use std::cell::Cell;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow, bail};
use serde::Deserialize;
use ureq::Agent;
use ureq::http::Response;

const API_BODY_TIMEOUT: Duration = Duration::from_secs(60);

pub struct Romm {
    base: String,
    token: String,
    agent: Agent,
    page_size: u32,
    retries: u32,
    backoff: Duration,
    media_interval: Duration,
    last_media: Cell<Option<Instant>>,
}

/// Leaves the token out.
impl fmt::Debug for Romm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Romm")
            .field("base", &self.base)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Deserialize)]
pub struct Platform {
    pub id: u64,
    pub slug: String,
}

#[derive(Debug, Deserialize)]
pub struct Rom {
    pub id: u64,
    pub fs_name: String,
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    /// Server path of the cover, e.g. `/assets/romm/resources/roms/52/7/cover/big.png?ts=…`.
    #[serde(default)]
    pub path_cover_large: Option<String>,
    #[serde(default)]
    pub metadatum: Option<RomMetadata>,
    #[serde(default)]
    pub igdb_id: Option<u64>,
    #[serde(default)]
    pub regions: Vec<String>,
    /// Empty for the first release.
    #[serde(default)]
    pub revision: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RomMetadata {
    pub genres: Vec<String>,
    pub player_count: Option<String>,
    /// Milliseconds since the Unix epoch.
    pub first_release_date: Option<i64>,
    /// Between 0 and 100.
    pub average_rating: Option<f64>,
}

/// The fields of `/api/roms/{id}` needed to fetch a game.
#[derive(Debug, Deserialize)]
pub struct RomDetail {
    pub id: u64,
    pub fs_name: String,
    pub name: String,
    pub fs_size_bytes: u64,
    #[serde(default)]
    pub has_multiple_files: bool,
}

#[derive(Deserialize)]
struct RomPage {
    items: Vec<Rom>,
    total: u64,
}

impl Romm {
    pub fn new(base_url: &str, token: &str) -> Romm {
        // No overall timeout: a disc image may take many minutes. API answers get one per request.
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_recv_response(Some(Duration::from_secs(60)))
            .build()
            .into();
        Romm {
            base: base_url.trim_end_matches('/').to_owned(),
            token: token.to_owned(),
            agent,
            page_size: 100,
            retries: 4,
            backoff: Duration::from_millis(500),
            media_interval: Duration::from_millis(200),
            last_media: Cell::new(None),
        }
    }

    pub fn page_size(mut self, size: u32) -> Romm {
        self.page_size = size;
        self
    }

    /// Wait before the first retry; doubles with every further one.
    pub fn backoff(mut self, backoff: Duration) -> Romm {
        self.backoff = backoff;
        self
    }

    /// Minimum time between two media downloads, so a sync does not flood the server or the DNS.
    pub fn media_interval(mut self, interval: Duration) -> Romm {
        self.media_interval = interval;
        self
    }

    pub fn platforms(&self) -> anyhow::Result<Vec<Platform>> {
        self.get_json("/api/platforms")
    }

    /// All ROMs of one platform, fetched page by page.
    pub fn roms(&self, platform_id: u64) -> anyhow::Result<Vec<Rom>> {
        let mut roms = Vec::new();
        loop {
            let page: RomPage = self.get_json(&format!(
                "/api/roms?platform_ids={platform_id}&order_by=name&order_dir=asc\
                 &with_char_index=false&with_filter_values=false&limit={}&offset={}",
                self.page_size,
                roms.len()
            ))?;
            let empty = page.items.is_empty();
            roms.extend(page.items);
            if empty || roms.len() as u64 >= page.total {
                return Ok(roms);
            }
        }
    }

    pub fn rom(&self, id: u64) -> anyhow::Result<RomDetail> {
        self.get_json(&format!("/api/roms/{id}"))
    }

    /// Downloads a game file to `dest`.
    pub fn download_rom(&self, id: u64, fs_name: &str, dest: &Path) -> anyhow::Result<()> {
        let path = format!("/api/roms/{id}/content/{}", percent_encode(fs_name));
        let mut response = self.get(&path, None)?;
        let mut file =
            fs::File::create(dest).with_context(|| format!("creating {}", dest.display()))?;
        io::copy(&mut response.body_mut().as_reader(), &mut file)
            .with_context(|| format!("downloading {fs_name}"))?;
        Ok(())
    }

    /// Downloads an image (cover, screenshot) by its server path to `dest`. Fails on anything
    /// that is not an image, such as the web frontend's index page.
    pub fn download_media(&self, path: &str, dest: &Path) -> anyhow::Result<()> {
        if let Some(last) = self.last_media.get() {
            let wait = self.media_interval.saturating_sub(last.elapsed());
            thread::sleep(wait);
        }
        self.last_media.set(Some(Instant::now()));
        // The `?ts=` query only busts caches, and RomM puts unencoded spaces into it.
        let path = path.split('?').next().unwrap_or(path).replace(' ', "%20");
        let mut response = self.get(&path, Some(API_BODY_TIMEOUT))?;
        let tmp = dest.with_extension("part");
        let mut file =
            fs::File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
        io::copy(&mut response.body_mut().as_reader(), &mut file)
            .with_context(|| format!("downloading {path}"))?;
        drop(file);
        if !is_image(&tmp) {
            let _ = fs::remove_file(&tmp);
            bail!("{path} is not an image");
        }
        fs::rename(&tmp, dest).with_context(|| format!("replacing {}", dest.display()))?;
        Ok(())
    }

    fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let mut response = self.get(path, Some(API_BODY_TIMEOUT))?;
        let text = response
            .body_mut()
            .with_config()
            .limit(64 * 1024 * 1024)
            .read_to_string()
            .with_context(|| format!("reading {path}"))?;
        serde_json::from_str(&text).with_context(|| format!("parsing the answer to {path}"))
    }

    /// GET with retries on transport errors, 429 and 5xx; other errors fail at once.
    /// `body_timeout` limits the time to read the whole answer.
    fn get(
        &self,
        path: &str,
        body_timeout: Option<Duration>,
    ) -> anyhow::Result<Response<ureq::Body>> {
        let url = format!("{}{path}", self.base);
        let mut last_error = anyhow!("no request made");
        for attempt in 0..=self.retries {
            if attempt > 0 {
                thread::sleep(self.backoff * 2u32.pow(attempt - 1));
            }
            match self
                .agent
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.token))
                .config()
                .timeout_recv_body(body_timeout)
                .build()
                .call()
            {
                Ok(response) if response.status().is_success() => return Ok(response),
                Ok(response) => {
                    let status = response.status();
                    let error = anyhow!("RomM answered {status} for {path}");
                    if status.as_u16() != 429 && !status.is_server_error() {
                        bail!(error);
                    }
                    last_error = error;
                }
                Err(e) => last_error = anyhow!(e).context(format!("requesting {path} from RomM")),
            }
        }
        Err(last_error)
    }
}

/// Checks the signature of PNG, JPEG, WebP and GIF files.
fn is_image(path: &Path) -> bool {
    use std::io::Read;
    let mut head = [0u8; 12];
    let Ok(n) = fs::File::open(path).and_then(|mut f| f.read(&mut head)) else {
        return false;
    };
    let head = &head[..n];
    head.starts_with(b"\x89PNG\r\n\x1a\n")
        || head.starts_with(&[0xFF, 0xD8, 0xFF])
        || (head.starts_with(b"RIFF") && head.get(8..12) == Some(b"WEBP"))
        || head.starts_with(b"GIF8")
}

/// Encodes everything but RFC 3986's unreserved characters, for a file name in a URL path.
fn percent_encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            b => format!("%{b:02X}"),
        })
        .collect()
}
