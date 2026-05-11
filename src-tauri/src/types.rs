use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;

// ── State ──────────────────────────────────────────────────────────────

pub struct CancelState(pub Arc<AtomicBool>);
pub struct YtDlpPath(pub Mutex<String>);

// ── Data Types ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub duration: Option<u64>,
    pub thumbnail: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlaylistResult {
    pub title: String,
    pub videos: Vec<VideoInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct DownloadSettings {
    pub playlist_url: String,
    pub cookie_file: String,
    pub output_dir: String,
    pub quality: String,
    pub format: String,
    pub proxy: Option<String>,
    pub include_comments: bool,
    pub auto_tag: bool,
    pub selected_indices: Vec<usize>,
    pub single_video: bool,
    pub inject_metadata: bool,
    pub update_mode: bool,
    pub export_comments: Option<String>,
    pub download_subs: bool,
    pub sub_langs: Option<String>,
    pub auto_subs: bool,
    pub write_info_json: bool,
    pub flat_output: bool,
    pub no_watermark: bool,
    pub max_concurrent: usize,
    pub is_tiktok: bool,
}

#[derive(Serialize)]
pub struct CommentExport {
    pub video: String,
    pub video_id: String,
    pub comments: Vec<CommentEntry>,
}

#[derive(Serialize)]
pub struct CommentEntry {
    pub author: String,
    pub text: String,
    pub date: String,
    pub likes: u64,
    pub is_creator: bool,
    pub parent: String,
}
