//! Helpers shared by the integration tests: a small fake YouTube API and a self-cleaning temp dir.
#![allow(dead_code)] // each test file uses a different subset

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub const CHANNEL_ID: &str = "UC_TECH_UNTANGLED";
pub const API_KEY: &str = "AIzaSyTestKey_0123456789";
pub const CHANNEL_ETAG: &str = "\"etag-yt-v1\"";

#[derive(Debug, Clone)]
pub struct Request {
    pub path: String,
    pub query: String,
    /// Header names are lowercased.
    pub headers: HashMap<String, String>,
}

pub struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Response {
    pub fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: body.to_string(),
        }
    }

    pub fn empty(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: String::new(),
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

/// A local HTTP server that answers every request with whatever the handler returns, and
/// remembers what it was asked.
pub struct MockServer {
    /// What the telemetry engine takes as its YouTube base URL.
    pub base_url: String,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl MockServer {
    pub async fn start(handler: impl Fn(&Request) -> Response + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local port for the mock server");
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let handler = Arc::new(handler);

        let log = requests.clone();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let (log, handler) = (log.clone(), handler.clone());
                tokio::spawn(async move {
                    let mut stream = stream;
                    let Some(request) = read_request(&mut stream).await else {
                        return;
                    };
                    let response = handler(&request);
                    log.lock().unwrap().push(request);
                    let _ = stream.write_all(&encode(&response)).await;
                });
            }
        });

        Self {
            base_url: format!("http://127.0.0.1:{port}/youtube"),
            requests,
        }
    }

    /// The standard fake: a channel with two uploads, ETag support included.
    pub async fn youtube() -> Self {
        Self::start(youtube_handler).await
    }

    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }

    /// How many requests went to an endpoint such as `channels`.
    pub fn hits(&self, endpoint: &str) -> usize {
        let path = format!("/youtube/{endpoint}");
        self.requests().iter().filter(|r| r.path == path).count()
    }
}

pub fn youtube_handler(request: &Request) -> Response {
    match request.path.as_str() {
        "/youtube/channels" => {
            if request.headers.get("if-none-match").map(String::as_str) == Some(CHANNEL_ETAG) {
                return Response::empty(304).header("ETag", CHANNEL_ETAG);
            }
            Response::json(200, &channel_json("Tech Untangled")).header("ETag", CHANNEL_ETAG)
        }
        "/youtube/playlistItems" => Response::json(
            200,
            r#"{"items":[
                {"contentDetails":{"videoId":"vid_latest_1"}},
                {"contentDetails":{"videoId":"vid_older_2"}}
            ]}"#,
        ),
        "/youtube/videos" => Response::json(200, VIDEOS_JSON),
        _ => Response::empty(404),
    }
}

pub fn channel_json(title: &str) -> String {
    serde_json::json!({
        "items": [{
            "snippet": {
                "title": title,
                "customUrl": "@techuntangled",
                "thumbnails": {
                    "default": { "url": "https://evil.example.com/tracker.png" },
                    "high": { "url": "https://yt3.ggpht.com/avatar=s800" }
                }
            },
            "statistics": {
                "subscriberCount": "142500",
                "videoCount": "94",
                "viewCount": "8910200"
            },
            "contentDetails": { "relatedPlaylists": { "uploads": "UU_TECH_UNTANGLED" } }
        }]
    })
    .to_string()
}

const VIDEOS_JSON: &str = r#"{"items":[
    {
        "snippet": {
            "title": "Building a Local Rust Notch Widget",
            "publishedAt": "2026-10-01T09:30:00Z",
            "thumbnails": {
                "default": { "url": "https://evil.example.com/tracker.png" },
                "medium": { "url": "https://i.ytimg.com/vi/vid_latest_1/mqdefault.jpg" }
            }
        },
        "statistics": { "viewCount": "52300", "likeCount": "4100", "commentCount": "380" }
    },
    {
        "snippet": { "title": "Tauri v2 Deep Dive" },
        "statistics": { "viewCount": "89000", "likeCount": "6200", "commentCount": "520" }
    }
]}"#;

async fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 2048];
    while !data.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 || data.len() > 64 * 1024 {
            return None;
        }
        data.extend_from_slice(&chunk[..read]);
    }
    let text = String::from_utf8_lossy(&data);
    let mut lines = text.lines();
    let target = lines.next()?.split_whitespace().nth(1)?;
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let headers = lines
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect();
    Some(Request {
        path: path.to_string(),
        query: query.to_string(),
        headers,
    })
}

fn encode(response: &Response) -> Vec<u8> {
    let mut head = format!(
        "HTTP/1.1 {} Mock\r\nContent-Length: {}\r\nConnection: close\r\n",
        response.status,
        response.body.len()
    );
    for (name, value) in &response.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    [head.as_bytes(), response.body.as_bytes()].concat()
}

/// A directory under the system temp dir that is deleted when it goes out of scope.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(label: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("growth-notch-test-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
