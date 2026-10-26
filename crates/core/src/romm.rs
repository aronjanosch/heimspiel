//! A small synchronous RomM client (ADR 0010): one keep-alive connection, retries with backoff on
//! busy servers, a minimum interval between media downloads, and a token that never leaves the
//! `Authorization` header.

use std::cell::Cell;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow, bail};
use serde::Deserialize;
use ureq::Agent;
use ureq::http::Response;

use crate::start::{self, Start};

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
    /// Whether the game is a folder of files rather than one file.
    #[serde(default)]
    pub has_multiple_files: bool,
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
    /// The game's file, or its folder for games made of several files.
    pub fs_name: String,
    /// The server folder holding `fs_name`, e.g. `roms/dc`.
    pub fs_path: String,
    pub name: String,
    #[serde(default)]
    pub has_multiple_files: bool,
    pub files: Vec<RomFile>,
}

/// One file of a game.
#[derive(Debug, Deserialize)]
pub struct RomFile {
    pub id: u64,
    pub file_name: String,
    /// The server folder holding the file, e.g. `roms/dc/Metal Slug 6`.
    pub file_path: String,
    pub file_size_bytes: u64,
}

impl RomDetail {
    /// Where `file` belongs inside the game's folder, e.g. `track01.iso` or `data/Stage/0.pxa`.
    pub fn relative_path(&self, file: &RomFile) -> anyhow::Result<String> {
        let root = if self.has_multiple_files {
            format!("{}/{}", self.fs_path, self.fs_name)
        } else {
            self.fs_path.clone()
        };
        let dir = file
            .file_path
            .strip_prefix(&root)
            .with_context(|| format!("{} lies outside {root}", file.file_path))?;
        let path = Path::new(dir.trim_start_matches('/')).join(&file.file_name);
        if !path.components().all(|c| matches!(c, Component::Normal(_))) {
            bail!("{} is no plain path", path.display());
        }
        Ok(path.to_string_lossy().into_owned())
    }

    /// What a frontend starts, or `None` when the files do not tell.
    pub fn start(&self) -> Option<Start> {
        if !self.has_multiple_files {
            return Some(Start::File(self.files.first()?.file_name.clone()));
        }
        let top: Vec<&str> = self
            .files
            .iter()
            .filter(|f| self.relative_path(f).is_ok_and(|p| !p.contains('/')))
            .map(|f| f.file_name.as_str())
            .collect();
        start::of(&self.fs_name, &top)
    }
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

    /// Downloads one file of a game to `dest`, continuing a partial download already there.
    /// `progress` receives the size of `dest` as it grows.
    pub fn download_file(
        &self,
        file: &RomFile,
        dest: &Path,
        progress: &dyn Fn(u64),
    ) -> anyhow::Result<()> {
        let path = format!(
            "/api/roms/{}/files/content/{}",
            file.id,
            percent_encode(&file.file_name)
        );
        let have = fs::metadata(dest).map_or(0, |m| m.len());
        let range = (have > 0).then(|| format!("bytes={have}-"));
        let mut response = self.get(&path, None, range.as_deref())?;
        // A server that ignores the range sends the whole file again.
        let resumed = response.status().as_u16() == 206;
        let out = if resumed {
            fs::File::options().append(true).open(dest)
        } else {
            fs::File::create(dest)
        }
        .with_context(|| format!("opening {}", dest.display()))?;
        let mut out = Counting {
            out,
            written: if resumed { have } else { 0 },
            progress,
        };
        io::copy(&mut response.body_mut().as_reader(), &mut out)
            .with_context(|| format!("downloading {}", file.file_name))?;
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
        let mut response = self.get(&path, Some(API_BODY_TIMEOUT), None)?;
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
        let mut response = self.get(path, Some(API_BODY_TIMEOUT), None)?;
        let text = response
            .body_mut()
            .with_config()
            .limit(64 * 1024 * 1024)
            .read_to_string()
            .with_context(|| format!("reading {path}"))?;
        serde_json::from_str(&text).with_context(|| format!("parsing the answer to {path}"))
    }

    /// GET with retries on transport errors, 429 and 5xx; other errors fail at once.
    /// `body_timeout` limits the time to read the whole answer; `range` asks for part of it.
    fn get(
        &self,
        path: &str,
        body_timeout: Option<Duration>,
        range: Option<&str>,
    ) -> anyhow::Result<Response<ureq::Body>> {
        let url = format!("{}{path}", self.base);
        let mut last_error = anyhow!("no request made");
        for attempt in 0..=self.retries {
            if attempt > 0 {
                thread::sleep(self.backoff * 2u32.pow(attempt - 1));
            }
            let mut request = self
                .agent
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.token));
            if let Some(range) = range {
                request = request.header("Range", range);
            }
            match request
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

/// Passes written data on to `out` and reports the running total.
struct Counting<'a> {
    out: fs::File,
    written: u64,
    progress: &'a dyn Fn(u64),
}

impl Write for Counting<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.out.write(buf)?;
        self.written += n as u64;
        (self.progress)(self.written);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
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
