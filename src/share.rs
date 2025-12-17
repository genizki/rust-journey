use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

pub enum WorkerMessage {
    Data(SearchResponse),
    Progress(u32),
    Error(String),
    Done(usize),
}

// Tokio worker for async operations
pub struct TokioWorker {
    pub tx: mpsc::Sender<WorkerMessage>,
    pub rx: mpsc::Receiver<WorkerMessage>,
}

impl Default for TokioWorker {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel(100); // Buffer size of 100
        Self { tx, rx }
    }
}

// Panel size structure for UI layout
pub struct PanelSize {
    pub side_width: f32,
    pub _central_width: f32,
}

// App state management
#[derive(Default)]
pub enum AppState {
    #[default]
    App,
    Settings,
    Warning,
    Test,
}

// Settings state
#[derive(Serialize, Deserialize)]
pub struct SettingsState {
    pub max_results: i8,
    pub first_run: bool,
    pub download_path: String,
    pub personal_yt_api: String,
}

impl SettingsState {
    pub fn default() -> Self {
        Self {
            max_results: 10,
            first_run: true,
            download_path: "".to_string(),
            personal_yt_api: "".to_string(),
        }
    }
}

impl Default for SettingsState {
    fn default() -> Self {
        Self::default()
    }
}

// UI Constants
pub const WIDTH: f32 = 120.0;
pub const HEIGHT: f32 = 120.0;

// Platform-specific paths
#[cfg(target_os = "macos")]
pub const DOWNLOAD_PATH: &str = "~/Downloads";

#[cfg(target_os = "windows")]
pub const DOWNLOAD_PATH: &str = "%USERPROFILE%\\Downloads";

#[cfg(target_os = "windows")]
pub const YT_DLP_BINARY: &str = "./yt_dlp/yt-dlp.exe";

#[cfg(not(target_os = "windows"))]
pub const YT_DLP_BINARY: &str = "./yt_dlp/yt-dlp_macos";

pub struct SearchResponseMeta {
    pub is_enabled: bool,
    pub download_progress: usize,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SearchResponse {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub etag: String,
    #[serde(rename = "nextPageToken", default)]
    pub next_page_token: String,
    #[serde(rename = "regionCode", default)]
    pub region_code: String,
    #[serde(rename = "pageInfo", default)]
    pub page_info: Option<PageInfo>,
    #[serde(default)]
    pub items: Vec<SearchItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchItem {
    pub kind: String,
    pub etag: String,
    pub id: Id,
    pub snippet: Snippet,
    #[serde(skip)]
    pub is_enabled: bool,
    #[serde(skip)]
    pub video_durration: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PageInfo {
    #[serde(rename = "totalResults")]
    pub total_results: u64,
    #[serde(rename = "resultsPerPage")]
    pub results_per_page: u64,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Id {
    pub kind: String,
    #[serde(rename = "videoId")]
    pub video_id: Option<String>,
    #[serde(rename = "channelId")]
    pub channel_id: Option<String>,
    #[serde(rename = "playlistId")]
    pub playlist_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Snippet {
    #[serde(rename = "publishedAt")]
    pub published_at: String,
    #[serde(rename = "channelId")]
    pub channel_id: String,
    pub title: String,
    pub description: String,
    pub thumbnails: Thumbnails,
    #[serde(rename = "channelTitle")]
    pub channel_title: String,
    #[serde(rename = "liveBroadcastContent")]
    pub live_broadcast_content: String,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Thumbnails {
    pub default: Option<ThumbnailData>,
    pub medium: Option<ThumbnailData>,
    pub high: Option<ThumbnailData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThumbnailData {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Default)]
pub struct SearchDuration {
    items: Vec<SearchDurationItem>,
}

#[derive(Default)]
pub struct SearchDurationItem {
    video_id: String,
    video_durration: String,
}
