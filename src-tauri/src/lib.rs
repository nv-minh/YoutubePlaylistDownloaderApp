use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use tauri::{Emitter, State};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

// ── Environment ────────────────────────────────────────────────────────

// ── Pre-compiled Regexes ────────────────────────────────────────────────

static RE_SANITIZE_FOLDER: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"[<>:"/\\|?*]"#).unwrap()
});
static RE_TITLE_METADATA: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?i)\s*[\(\[{].*?(official|video|mv|music|lyric|audio|4k|hd|1080p).*?[\)\]}]"#).unwrap()
});
static RE_PLAYLIST_ID: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"[?&]list=([a-zA-Z0-9_-]+)").unwrap()
});
static RE_PLAIN_ID: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap()
});
static RE_PROGRESS: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\[download\]\s+([\d.]+)%").unwrap()
});

// ── Environment ────────────────────────────────────────────────────────

fn setup_env(cmd: &mut Command) {
    let sep = if cfg!(target_os = "windows") { ";" } else { ":" };
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();

    let mut paths: Vec<String> = vec![];

    if cfg!(target_os = "windows") {
        // Windows: common install locations
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let program_files = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into());
        let program_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".into());

        if !local_app_data.is_empty() {
            paths.push(format!("{}\\Programs\\Python", local_app_data));
            paths.push(format!("{}\\Programs\\Python\\Python311", local_app_data));
            paths.push(format!("{}\\Programs\\Python\\Python310", local_app_data));
            paths.push(format!("{}\\Programs\\Python\\Python39", local_app_data));
        }
        paths.push(format!("{}\\Python311", program_files));
        paths.push(format!("{}\\Python310", program_files));
        paths.push(format!("{}\\Python39", program_files));
        paths.push(format!("{}\\Python311", program_files_x86));
        // App data dir for yt-dlp.exe downloaded by our app
        let app_data = std::env::var("APPDATA").unwrap_or_default();
        if !app_data.is_empty() {
            paths.push(format!("{}\\yt-playlist-downloader", app_data));
        }
    } else {
        // macOS / Linux
        let mut nvm_paths: Vec<String> = vec![];
        let nvm_dir = PathBuf::from(format!("{}/.nvm/versions/node", home));
        if let Ok(entries) = fs::read_dir(&nvm_dir) {
            for entry in entries.flatten() {
                if entry.path().join("bin/node").exists() {
                    nvm_paths.push(entry.path().join("bin").to_string_lossy().to_string());
                }
            }
        }
        if nvm_paths.is_empty() {
            nvm_paths.push(format!("{}/.nvm/versions/node/v20.14.0/bin", home));
        }
        paths.extend(nvm_paths);
        paths.push(format!("{}/.cargo/bin", home));
        paths.push("/usr/local/bin".into());
        paths.push("/opt/homebrew/bin".into());
        paths.push("/usr/bin".into());
    }

    if let Ok(existing) = std::env::var("PATH") {
        for p in existing.split(sep) {
            let s = p.to_string();
            if !paths.contains(&s) {
                paths.push(s);
            }
        }
    }
    cmd.env("PATH", paths.join(sep));
    if cfg!(target_os = "windows") {
        cmd.env("PYTHONUTF8", "1");
    }
}

fn new_cmd(program: &str) -> Command {
    let mut cmd = Command::new(program);
    setup_env(&mut cmd);
    cmd
}

// ── HTML Generation ────────────────────────────────────────────────────

fn load_comments_for_video(video_dir: &PathBuf) -> Vec<serde_json::Value> {
    if !video_dir.is_dir() {
        return vec![];
    }
    if let Ok(entries) = fs::read_dir(video_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".info.json") {
                if let Ok(data) = fs::read_to_string(entry.path()) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
                        if let Some(comments) = json.get("comments").and_then(|c| c.as_array()) {
                            return comments.clone();
                        }
                    }
                }
            }
        }
    }
    vec![]
}

fn render_comment(c: &serde_json::Value) -> String {
    let author = c["author"].as_str().unwrap_or("Anon");
    let text = c["text"].as_str().unwrap_or("");
    let likes = c["like_count"].as_u64().unwrap_or(0);
    let author_id = c["author_id"].as_str().unwrap_or("");
    let is_op = c["author_is_uploader"].as_bool().unwrap_or(false);
    let op_badge = if is_op { "<span class=\"op-badge\">Creator</span>" } else { "" };

    let avatar = c["author_thumbnail"].as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("https://ui-avatars.com/api/?name={}&background=random&color=fff&size=40", &author[..1.min(author.len())]));

    let date_str = c["timestamp"].as_u64()
        .map(|ts| {
            let secs = ts.min(i64::MAX as u64) as i64;
            chrono::DateTime::from_timestamp(secs, 0)
                .map(|dt| dt.format("%d/%m/%Y %H:%M").to_string())
                .unwrap_or_default()
        })
        .unwrap_or_default();

    format!(
        r#"<div class="comment">
            <img class="avatar" src="{}" alt="" onerror="this.src='https://ui-avatars.com/api/?name={}&background=random&color=fff&size=40'">
            <div class="comment-body">
                <div class="comment-header">
                    <a href="https://www.youtube.com/@{}" target="_blank" class="author">{}</a>
                    {}
                    <span class="date">{}</span>
                </div>
                <p class="comment-text">{}</p>
                <div class="comment-meta">
                    <span class="likes">&#9650; {}</span>
                </div>
            </div>
        </div>"#,
        avatar, &author[..1.min(author.len())], author_id, html_escape(author), op_badge, date_str, html_escape(text), likes
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn build_nested_html(comments: &[serde_json::Value]) -> String {
    let roots: Vec<_> = comments.iter().filter(|c| c["parent"].as_str() == Some("root")).collect();
    let mut replies_map: std::collections::HashMap<String, Vec<&serde_json::Value>> = std::collections::HashMap::new();
    for c in comments {
        if c["parent"].as_str() != Some("root") {
            if let Some(parent) = c["parent"].as_str() {
                replies_map.entry(parent.to_string()).or_default().push(c);
            }
        }
    }

    let mut out = String::new();
    for root in roots {
        out.push_str("<div class=\"comment-thread\">");
        out.push_str(&render_comment(root));
        if let Some(id) = root["id"].as_str() {
            if let Some(replies) = replies_map.get(id) {
                out.push_str("<div class=\"replies\">");
                for reply in replies {
                    out.push_str(&render_comment(reply));
                }
                out.push_str("</div>");
            }
        }
        out.push_str("</div>");
    }
    out
}

fn generate_video_comments_html(video_dir: &PathBuf, video_title: &str, video_id: &str, channel: &str) -> usize {
    let comments = load_comments_for_video(video_dir);
    if comments.is_empty() {
        return 0;
    }

    let thumbnail = format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", video_id);
    let video_url = format!("https://www.youtube.com/watch?v={}", video_id);
    let comments_html = build_nested_html(&comments);
    let count = comments.len();

    let page = format!(r#"<!DOCTYPE html>
<html lang="vi">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - Comments</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0f0f0f; color: #f1f1f1; }}
        .video-banner {{ display: flex; gap: 16px; padding: 20px; background: #1a1a1a; border-bottom: 1px solid #2a2a2a; align-items: flex-start; }}
        .video-banner img {{ width: 160px; height: 90px; object-fit: cover; border-radius: 8px; }}
        .video-banner .info h1 {{ font-size: 18px; margin-bottom: 6px; }}
        .video-banner .info h1 a {{ color: #f1f1f1; text-decoration: none; }}
        .video-banner .info h1 a:hover {{ color: #3ea6ff; }}
        .video-banner .info .meta {{ color: #aaa; font-size: 13px; }}
        .video-banner .info .meta a {{ color: #aaa; text-decoration: none; }}
        .video-banner .info .meta a:hover {{ color: #3ea6ff; }}
        .comments-count {{ padding: 16px 20px; font-size: 16px; font-weight: 600; border-bottom: 1px solid #2a2a2a; }}
        .comments-container {{ padding: 0 20px 20px; }}
        .comment-thread {{ border-bottom: 1px solid #222; padding: 12px 0; }}
        .comment-thread:last-child {{ border-bottom: none; }}
        .comment {{ display: flex; gap: 12px; }}
        .replies {{ margin-left: 52px; padding-left: 12px; border-left: 2px solid #333; margin-top: 8px; display: flex; flex-direction: column; gap: 8px; }}
        .replies .comment {{ padding: 4px 0; }}
        .replies .avatar {{ width: 28px; height: 28px; }}
        .avatar {{ width: 40px; height: 40px; border-radius: 50%; flex-shrink: 0; }}
        .comment-body {{ flex: 1; min-width: 0; }}
        .comment-header {{ display: flex; gap: 10px; align-items: baseline; margin-bottom: 4px; }}
        .author {{ color: #f1f1f1; font-weight: 600; font-size: 13px; text-decoration: none; }}
        .author:hover {{ color: #3ea6ff; }}
        .op-badge {{ background: #3ea6ff; color: #0f0f0f; font-size: 10px; font-weight: 700; padding: 1px 6px; border-radius: 4px; }}
        .date {{ color: #717171; font-size: 12px; }}
        .comment-text {{ font-size: 14px; line-height: 1.5; white-space: pre-wrap; word-wrap: break-word; }}
        .comment-meta {{ margin-top: 4px; }}
        .likes {{ color: #717171; font-size: 12px; }}
        .empty {{ color: #717171; text-align: center; padding: 40px 20px; font-size: 14px; }}
    </style>
</head>
<body>
    <div class="video-banner">
        <a href="{}" target="_blank"><img src="{}" alt=""></a>
        <div class="info">
            <h1><a href="{}" target="_blank">{}</a></h1>
            <p class="meta"><a href="https://www.youtube.com/@{}" target="_blank">{}</a></p>
        </div>
    </div>
    <div class="comments-count">{} comments</div>
    <div class="comments-container">
        {}
    </div>
</body>
</html>"#,
        html_escape(video_title),
        video_url, thumbnail, video_url, html_escape(video_title),
        html_escape(channel), html_escape(channel),
        count,
        if comments_html.is_empty() { "<p class=\"empty\">No comments.</p>".to_string() } else { comments_html }
    );

    let html_path = video_dir.join("comments.html");
    let _ = fs::write(&html_path, page);
    count
}

fn generate_index_html(playlist_title: &str, videos: &[VideoInfo], base_dir: &PathBuf, video_ok: u32, total_comments: u32) {
    let mut rows = String::new();
    for video in videos {
        let folder_name = sanitize_folder_name(&video.title);
        let video_dir = base_dir.join(&folder_name);

        let has_video = video_dir.join(format!("video.{}", "mp4")).exists()
            || video_dir.join("video.webm").exists()
            || video_dir.join("video.mkv").exists();
        let has_comments = video_dir.join("comments.html").exists();

        let dur = video.duration.map(|s| {
            let h = s / 3600;
            let m = (s % 3600) / 60;
            let sec = s % 60;
            if h > 0 { format!("{}:{:02}:{:02}", h, m, sec) } else { format!("{}:{:02}", m, sec) }
        }).unwrap_or_default();

        let comments_link = if has_comments {
            format!("<a class=\"btn-comments\" href=\"{}/comments.html\">View comments</a>", folder_name)
        } else {
            "<span class=\"no-comments-link\">No comments</span>".to_string()
        };

        let video_badge = if has_video {
            "<span class=\"badge badge-green\">&#9654; Video</span>"
        } else {
            "<span class=\"badge badge-orange\">&#9632; No video</span>"
        };

        rows.push_str(&format!(r#"
        <div class="video-row">
            <img class="thumb" src="{}" alt="" loading="lazy">
            <div class="info">
                <h3><a href="https://www.youtube.com/watch?v={}" target="_blank">{}</a></h3>
                <p class="channel">{}</p>
                <div class="badges">
                    {}
                    {}
                </div>
            </div>
            <div class="actions">{}</div>
        </div>"#,
            video.thumbnail, video.id, html_escape(&video.title), html_escape(&video.channel),
            if dur.is_empty() { String::new() } else { format!("<span class=\"badge\">{}</span>", dur) },
            video_badge, comments_link
        ));
    }

    let page = format!(r#"<!DOCTYPE html>
<html lang="vi">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0f0f0f; color: #f1f1f1; }}
        .header {{ text-align: center; padding: 30px 20px; background: linear-gradient(135deg, #ff0000 0%, #cc0000 100%); }}
        .header h1 {{ font-size: 26px; margin-bottom: 8px; }}
        .header .stats {{ color: #ffcccc; font-size: 14px; }}
        .stats span {{ margin: 0 12px; }}
        .list {{ max-width: 900px; margin: 0 auto; padding: 20px; }}
        .video-row {{ display: flex; align-items: center; gap: 16px; padding: 16px; background: #1a1a1a; border-radius: 10px; margin-bottom: 10px; border: 1px solid #2a2a2a; }}
        .thumb {{ width: 120px; height: 68px; object-fit: cover; border-radius: 6px; flex-shrink: 0; }}
        .info {{ flex: 1; min-width: 0; }}
        .info h3 {{ font-size: 14px; margin-bottom: 4px; }}
        .info h3 a {{ color: #f1f1f1; text-decoration: none; }}
        .info h3 a:hover {{ color: #3ea6ff; }}
        .channel {{ color: #aaa; font-size: 12px; margin-bottom: 6px; }}
        .badges {{ display: flex; gap: 6px; flex-wrap: wrap; }}
        .badge {{ background: #2a2a2a; padding: 2px 8px; border-radius: 10px; font-size: 11px; color: #aaa; }}
        .badge-blue {{ color: #3ea6ff; }}
        .badge-green {{ color: #4caf50; }}
        .badge-orange {{ color: #ff9800; }}
        .actions {{ flex-shrink: 0; }}
        .btn-comments {{ display: inline-block; padding: 8px 16px; background: #3ea6ff; color: #0f0f0f; border-radius: 6px; text-decoration: none; font-size: 12px; font-weight: 600; }}
        .btn-comments:hover {{ background: #65b8ff; }}
        .no-comments-link {{ color: #555; font-size: 12px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>&#9654; {}</h1>
        <div class="stats">
            <span>{} videos</span>
            <span>{} downloaded</span>
            <span>{} comments</span>
        </div>
    </div>
    <div class="list">{}</div>
</body>
</html>"#,
        html_escape(playlist_title), html_escape(playlist_title),
        videos.len(), video_ok, total_comments, rows
    );

    let _ = fs::write(base_dir.join("index.html"), page);
}

// ── State ──────────────────────────────────────────────────────────────

pub struct CancelState(pub AtomicBool);
pub struct YtDlpPath(pub Mutex<String>);

// ── Types ──────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoInfo {
    id: String,
    title: String,
    channel: String,
    duration: Option<u64>,
    thumbnail: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlaylistResult {
    title: String,
    videos: Vec<VideoInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DownloadSettings {
    playlist_url: String,
    cookie_file: String,
    output_dir: String,
    quality: String,
    format: String,
    proxy: Option<String>,
    include_comments: bool,
    auto_tag: bool,
    selected_indices: Vec<usize>,
    single_video: bool,
    inject_metadata: bool,
    update_mode: bool,
    export_comments: Option<String>,
    download_subs: bool,
    sub_langs: Option<String>,
    auto_subs: bool,
}

// ── Helpers ────────────────────────────────────────────────────────────

fn yt_dlp_extra() -> Vec<String> {
    vec![
        "--js-runtimes".into(),
        "node".into(),
        "--remote-components".into(),
        "ejs:github".into(),
    ]
}

fn quality_format(quality: &str, format: &str) -> String {
    match format {
        "mp3" | "flac" | "wav" | "ogg" | "m4a" => "ba/b".into(),
        _ => match quality {
            "1080p" => "bv*[height<=1080]+ba/b[height<=1080]/bv*+ba/b".into(),
            "720p" => "bv*[height<=720]+ba/b[height<=720]/bv*+ba/b".into(),
            "480p" => "bv*[height<=480]+ba/b[height<=480]/bv*+ba/b".into(),
            _ => "bv*+ba/b".into(),
        },
    }
}

fn parse_title_metadata(title: &str) -> (String, String, String) {
    // Try "Artist - Title" pattern
    if let Some(pos) = title.find(" - ") {
        let artist = title[..pos].trim().to_string();
        let rest = title[pos + 3..].trim();
        let clean_title = RE_TITLE_METADATA.replace_all(rest, "").trim().to_string();
        let genre = String::new();
        return (artist, clean_title, genre);
    }
    (String::new(), title.to_string(), String::new())
}

fn sanitize_folder_name(name: &str) -> String {
    let cleaned = RE_SANITIZE_FOLDER.replace_all(name, "").to_string();
    // Strip non-ASCII characters (Vietnamese diacritics cause cp1252 errors on Windows)
    let ascii: String = cleaned.chars().filter(|c| c.is_ascii()).collect();
    let trimmed = ascii.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.len() > 200 {
        trimmed[..200].to_string()
    } else {
        trimmed
    }
}

fn extract_playlist_id(url: &str) -> Option<String> {
    if let Some(caps) = RE_PLAYLIST_ID.captures(url) {
        return Some(caps[1].to_string());
    }
    if RE_PLAIN_ID.is_match(url.trim())
    {
        return Some(url.trim().to_string());
    }
    None
}

// ── Metadata Injection ──────────────────────────────────────────────────

fn inject_metadata(
    video_dir: &Path,
    thumbnail_url: &str,
    video_title: &str,
    playlist_title: &str,
    format: &str,
) -> Result<(), String> {
    let thumb_path = video_dir.join("thumb.jpg");
    let ext = format;

    // Download thumbnail
    let curl_output = std::process::Command::new("curl")
        .args(["-sL", thumbnail_url, "-o"])
        .arg(&thumb_path)
        .output()
        .map_err(|e| format!("curl failed: {}", e))?;

    if !curl_output.status.success() || !thumb_path.exists() {
        return Err("Failed to download thumbnail".into());
    }

    let video_path = video_dir.join(format!("video.{}", ext));
    let tagged_path = video_dir.join(format!("video_tagged.{}", ext));
    if !video_path.exists() {
        return Err("Video file not found".into());
    }

    let (artist, clean_title, _) = parse_title_metadata(video_title);
    let is_audio = matches!(ext, "mp3" | "flac" | "wav" | "ogg" | "m4a");

    let mut ffmpeg_cmd = std::process::Command::new("ffmpeg");
    ffmpeg_cmd.arg("-y");
    ffmpeg_cmd.arg("-i").arg(&video_path);
    ffmpeg_cmd.arg("-i").arg(&thumb_path);

    if is_audio {
        ffmpeg_cmd.args(["-map", "0:a", "-map", "1:0", "-c:a", "copy"]);
        if ext == "mp3" {
            ffmpeg_cmd.args(["-c:v", "mjpeg", "-id3v2_version", "3"]);
        } else {
            ffmpeg_cmd.args(["-c:v", "copy"]);
        }
    } else {
        ffmpeg_cmd.args(["-map", "0", "-map", "1:0", "-c", "copy"]);
        ffmpeg_cmd.args(["-disposition:v:1", "attached_pic"]);
    }

    if !clean_title.is_empty() {
        ffmpeg_cmd.args(["-metadata", &format!("title={}", clean_title)]);
    }
    if !artist.is_empty() {
        ffmpeg_cmd.args(["-metadata", &format!("artist={}", artist)]);
    }
    if !playlist_title.is_empty() {
        ffmpeg_cmd.args(["-metadata", &format!("album={}", playlist_title)]);
    }

    ffmpeg_cmd.arg(&tagged_path);

    let output = ffmpeg_cmd.output().map_err(|e| format!("ffmpeg failed: {}", e))?;
    if output.status.success() && tagged_path.exists() {
        let _ = fs::remove_file(&video_path);
        fs::rename(&tagged_path, &video_path).map_err(|e| format!("rename failed: {}", e))?;
        let _ = fs::remove_file(&thumb_path);
        Ok(())
    } else {
        let _ = fs::remove_file(&tagged_path);
        Err(format!("ffmpeg error: {}", String::from_utf8_lossy(&output.stderr).chars().take(200).collect::<String>()))
    }
}

// ── Comment Export ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct CommentExport {
    video: String,
    video_id: String,
    comments: Vec<CommentEntry>,
}

#[derive(Serialize)]
struct CommentEntry {
    author: String,
    text: String,
    date: String,
    likes: u64,
    is_creator: bool,
    parent: String,
}

fn export_comments_to_file(base_dir: &Path, format: &str) -> Result<String, String> {
    let mut all_comments: Vec<CommentExport> = Vec::new();

    let entries = fs::read_dir(base_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let video_dir = entry.path();
        let comments = load_comments_for_video(&video_dir);
        if comments.is_empty() {
            continue;
        }

        // Try to get video info from .info.json
        let mut video_title = String::from("Unknown");
        let mut video_id = String::new();
        if let Ok(dir_entries) = fs::read_dir(&video_dir) {
            for de in dir_entries.flatten() {
                let name = de.file_name().to_string_lossy().to_string();
                if name.ends_with(".info.json") {
                    if let Ok(data) = fs::read_to_string(de.path()) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
                            video_title = json["title"].as_str().unwrap_or("Unknown").to_string();
                            video_id = json["id"].as_str().unwrap_or("").to_string();
                        }
                    }
                    break;
                }
            }
        }

        let entries: Vec<CommentEntry> = comments.iter().map(|c| {
            let ts = c["timestamp"].as_u64().unwrap_or(0);
            let date = chrono::DateTime::from_timestamp(ts as i64, 0)
                .map(|dt| dt.format("%d/%m/%Y %H:%M").to_string())
                .unwrap_or_default();
            CommentEntry {
                author: c["author"].as_str().unwrap_or("Anon").to_string(),
                text: c["text"].as_str().unwrap_or("").to_string(),
                date,
                likes: c["like_count"].as_u64().unwrap_or(0),
                is_creator: c["author_is_uploader"].as_bool().unwrap_or(false),
                parent: c["parent"].as_str().unwrap_or("root").to_string(),
            }
        }).collect();

        all_comments.push(CommentExport {
            video: video_title,
            video_id,
            comments: entries,
        });
    }

    if all_comments.is_empty() {
        return Err("No comments found to export".into());
    }

    let total_comments: usize = all_comments.iter().map(|c| c.comments.len()).sum();

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&all_comments)
                .map_err(|e| format!("JSON error: {}", e))?;
            let path = base_dir.join("comments.json");
            fs::write(&path, json).map_err(|e| e.to_string())?;
            Ok(format!("comments.json ({} comments)", total_comments))
        }
        "csv" => {
            let path = base_dir.join("comments.csv");
            let mut csv = String::from("Video,Video ID,Author,Comment,Date,Likes,Is Creator,Parent\n");
            for entry in &all_comments {
                for c in &entry.comments {
                    csv.push_str(&format!(
                        "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                        entry.video.replace('"', "\"\""),
                        entry.video_id,
                        c.author.replace('"', "\"\""),
                        c.text.replace('"', "\"\""),
                        c.date,
                        c.likes,
                        c.is_creator,
                        c.parent,
                    ));
                }
            }
            fs::write(&path, csv).map_err(|e| e.to_string())?;
            Ok(format!("comments.csv ({} comments)", total_comments))
        }
        _ => Err(format!("Unknown format: {}", format)),
    }
}

// ── Commands ───────────────────────────────────────────────────────────

#[tauri::command]
async fn check_ytdlp(path_state: State<'_, YtDlpPath>) -> Result<String, String> {
    let path = path_state.0.lock().await;
    let mut cmd = new_cmd(&*path);
    let output = cmd.arg("--version")
        .output()
        .await
        .map_err(|e| format!("yt-dlp not found: {}", e))?;
    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(version)
    } else {
        Err("yt-dlp check failed".into())
    }
}

#[tauri::command]
async fn install_ytdlp(path_state: State<'_, YtDlpPath>) -> Result<String, String> {
    if cfg!(target_os = "windows") {
        // Windows: download yt-dlp.exe directly from GitHub releases
        let app_data = std::env::var("APPDATA").unwrap_or_else(|_| {
            format!("C:\\Users\\{}\\AppData\\Roaming",
                std::env::var("USERNAME").unwrap_or_default())
        });
        let dir = PathBuf::from(&app_data).join("yt-playlist-downloader");
        fs::create_dir_all(&dir).map_err(|e| format!("Cannot create dir: {}", e))?;
        let exe_path = dir.join("yt-dlp.exe");
        let url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";

        // Try curl first, then PowerShell as fallback
        let curl_ok = new_cmd("curl")
            .args(["-sL", url, "-o"])
            .arg(&exe_path)
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !curl_ok || !exe_path.exists() {
            let ps_ok = new_cmd("powershell")
                .args(["-NoProfile", "-Command",
                    &format!("Invoke-WebRequest -Uri '{}' -OutFile '{}'", url, exe_path.to_string_lossy())])
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !ps_ok || !exe_path.exists() {
                return Err("Failed to download yt-dlp.exe. Please install manually from https://github.com/yt-dlp/yt-dlp".into());
            }
        }

        // Update path to point to downloaded exe
        let new_path = exe_path.to_string_lossy().to_string();
        *path_state.0.lock().await = new_path.clone();

        // Verify
        let version_output = new_cmd(&new_path)
            .arg("--version")
            .output()
            .await
            .map_err(|e| format!("yt-dlp version check failed: {}", e))?;
        Ok(String::from_utf8_lossy(&version_output.stdout).trim().to_string())
    } else {
        // macOS / Linux: use pip
        let python = "python3";
        let output = new_cmd(python)
            .args(["-m", "pip", "install", "--upgrade", "yt-dlp"])
            .output()
            .await
            .map_err(|e| format!("pip failed: {}", e))?;
        if output.status.success() {
            let path = path_state.0.lock().await;
            let version_output = new_cmd(&*path)
                .arg("--version")
                .output()
                .await
                .map_err(|e| format!("yt-dlp version check failed: {}", e))?;
            Ok(String::from_utf8_lossy(&version_output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }
}

#[tauri::command]
async fn fetch_playlist(
    url: String,
    cookie_file: String,
    proxy: Option<String>,
    path_state: State<'_, YtDlpPath>,
) -> Result<PlaylistResult, String> {
    let playlist_url = match extract_playlist_id(&url) {
        Some(id) => format!("https://www.youtube.com/playlist?list={}", id),
        None => url.clone(),
    };

    let yt_path = path_state.0.lock().await;
    let mut cmd = new_cmd(&*yt_path);
    cmd.args(yt_dlp_extra());
    if !cookie_file.is_empty() {
        cmd.args(["--cookies", &cookie_file]);
    }
    cmd.args(["--dump-json", "--flat-playlist"])
        .arg(&playlist_url);

    if let Some(ref p) = proxy {
        cmd.args(["--proxy", p]);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut videos = Vec::new();
    let mut playlist_title = String::from("YouTube Playlist");

    for (i, line) in stdout.lines().enumerate() {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(line) {
            if i == 0 {
                playlist_title = data["playlist_title"]
                    .as_str()
                    .or(data["playlist"].as_str())
                    .unwrap_or("YouTube Playlist")
                    .to_string();
            }
            // Filter out unavailable/private/upcoming videos
            let availability = data["availability"].as_str().unwrap_or("");
            let live_status = data["live_status"].as_str().unwrap_or("");
            if availability == "private"
                || availability == "premium_only"
                || live_status == "is_upcoming"
                || live_status == "post_live"
            {
                continue;
            }
            // Skip videos with no title or "Deleted" / "Private" placeholders
            let title = data["title"].as_str().unwrap_or("");
            if title.is_empty()
                || title == "[Private video]"
                || title == "[Deleted video]"
            {
                continue;
            }
            videos.push(VideoInfo {
                id: data["id"].as_str().unwrap_or("").to_string(),
                title: title.to_string(),
                channel: data["channel"].as_str().unwrap_or("").to_string(),
                duration: data["duration"].as_u64(),
                thumbnail: data["thumbnail"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            });
        }
    }

    Ok(PlaylistResult {
        title: playlist_title,
        videos,
    })
}

async fn fetch_single_video(
    url: String,
    cookie_file: String,
    proxy: Option<String>,
    path_state: State<'_, YtDlpPath>,
) -> Result<VideoInfo, String> {
    let yt_path = path_state.0.lock().await;
    let mut cmd = new_cmd(&*yt_path);
    cmd.args(yt_dlp_extra());
    if !cookie_file.is_empty() {
        cmd.args(["--cookies", &cookie_file]);
    }
    cmd.args(["--dump-single-json", "--no-playlist"]).arg(&url);
    if let Some(ref p) = proxy {
        cmd.args(["--proxy", p]);
    }

    let output = cmd.output().await.map_err(|e| format!("Failed to run yt-dlp: {}", e))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let data: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| format!("Parse error: {}", e))?;

    Ok(VideoInfo {
        id: data["id"].as_str().unwrap_or("").to_string(),
        title: data["title"].as_str().unwrap_or("Unknown").to_string(),
        channel: data["channel"].as_str().unwrap_or("").to_string(),
        duration: data["duration"].as_u64(),
        thumbnail: data["thumbnail"].as_str().unwrap_or("").to_string(),
    })
}

#[tauri::command]
async fn start_download(
    settings: DownloadSettings,
    cancel: State<'_, CancelState>,
    path_state: State<'_, YtDlpPath>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    cancel.0.store(false, Ordering::SeqCst);

    let yt_path = path_state.0.lock().await.clone();
    let fmt = quality_format(&settings.quality, &settings.format);
    let base_dir = PathBuf::from(&settings.output_dir);
    fs::create_dir_all(&base_dir).map_err(|e| e.to_string())?;

    let mut video_ok = 0u32;
    let mut comment_ok = 0u32;
    let mut total_comments = 0u32;

    let playlist_title: String;
    let videos: Vec<VideoInfo>;
    let selected: Vec<usize>;

    if settings.single_video {
        // Single video mode - fetch just one video
        let video = fetch_single_video(
            settings.playlist_url.clone(),
            settings.cookie_file.clone(),
            settings.proxy.clone(),
            path_state.clone(),
        )
        .await?;
        playlist_title = video.title.clone();
        videos = vec![video];
        selected = vec![0];
    } else {
        // Playlist mode
        let playlist_url = match extract_playlist_id(&settings.playlist_url) {
            Some(id) => format!("https://www.youtube.com/playlist?list={}", id),
            None => settings.playlist_url.clone(),
        };

        let result = fetch_playlist(
            playlist_url,
            settings.cookie_file.clone(),
            settings.proxy.clone(),
            path_state.clone(),
        )
        .await?;
        playlist_title = result.title.clone();
        videos = result.videos;
        selected = if settings.selected_indices.is_empty() {
            (0..videos.len()).collect()
        } else {
            settings.selected_indices.clone()
        };
    }

    let total = videos.len();

    // Wrap in playlist-named subfolder for playlist mode
    let base_dir = if settings.single_video {
        base_dir
    } else {
        let playlist_folder = base_dir.join(sanitize_folder_name(&playlist_title));
        fs::create_dir_all(&playlist_folder).map_err(|e| e.to_string())?;
        playlist_folder
    };

    app.emit(
        "download-log",
        if settings.single_video {
            format!("Video: {}", playlist_title)
        } else {
            format!("Playlist: {} ({} videos)", playlist_title, total)
        },
    )
    .ok();

    let selected_count = selected.len();
    let mut completed: usize = 0;

    for (i, video) in videos.iter().enumerate() {
        if !selected.contains(&i) {
            app.emit("download-status", (i + 1, "Skipped".to_string())).ok();
            continue;
        }
        if cancel.0.load(Ordering::SeqCst) {
            app.emit("download-log", "Cancelled.".to_string()).ok();
            break;
        }

        // Update mode: skip already downloaded videos
        if settings.update_mode {
            let folder_name = sanitize_folder_name(&video.title);
            let video_dir_check = base_dir.join(&folder_name);
            let exists = ["mp4","mp3","webm","mkv","avi","flac","wav","ogg","m4a"]
                .iter().any(|ext| video_dir_check.join(format!("video.{}", ext)).exists());
            if exists {
                app.emit("download-status", (i + 1, "Exists".to_string())).ok();
                app.emit("download-log", format!("[{}] {} - already exists, skipping", i + 1, video.title)).ok();
                video_ok += 1;
                continue;
            }
        }

        if cancel.0.load(Ordering::SeqCst) {
            app.emit("download-log", "Cancelled.".to_string()).ok();
            break;
        }

        let idx = i + 1;
        completed += 1;
        app.emit("download-progress", (completed, selected_count)).ok();
        app.emit(
            "download-status",
            (idx, "downloading".to_string()),
        )
        .ok();
        app.emit(
            "download-log",
            format!("[{}/{}] {}", idx, total, video.title),
        )
        .ok();

        let folder_name = sanitize_folder_name(&video.title);
        let video_dir = base_dir.join(&folder_name);
        fs::create_dir_all(&video_dir).ok();

        let video_url = format!("https://www.youtube.com/watch?v={}", video.id);

        // Download comments (if enabled)
        if settings.include_comments {
        let mut comment_cmd = new_cmd(&yt_path);
        comment_cmd
            .args(yt_dlp_extra());
        if !settings.cookie_file.is_empty() {
            comment_cmd.args(["--cookies", &settings.cookie_file]);
        }
        if let Some(ref p) = settings.proxy {
            comment_cmd.args(["--proxy", p]);
        }
        comment_cmd
            .args(["--write-comments", "--skip-download", "--no-warnings", "--force-ipv4"])
            .arg("-o")
            .arg(video_dir.join("video.%(ext)s").to_string_lossy().as_ref())
            .arg(&video_url);

        if let Ok(output) = comment_cmd.output().await {
            if output.status.success() {
                comment_ok += 1;
                // Generate comments.html
                let n = generate_video_comments_html(&video_dir, &video.title, &video.id, &video.channel);
                total_comments += n as u32;
                app.emit("download-log", format!("  -> {} comments", n)).ok();
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let msg: String = stderr.chars().take(200).collect();
                app.emit("download-log", format!("  -> Comment error: {}", msg)).ok();
                if msg.contains("Sign in") || msg.contains("bot") || msg.contains("cookie") || msg.contains("HTTP Error 403") {
                    app.emit("download-log", "  Hint: Your cookies may have expired. Please re-export from browser and paste again.".to_string()).ok();
                }
            }
        }
        }

        if cancel.0.load(Ordering::SeqCst) {
            break;
        }

        // Download video
        {
            let mut video_cmd = new_cmd(&yt_path);
            video_cmd
                .args(yt_dlp_extra());
            if !settings.cookie_file.is_empty() {
                video_cmd.args(["--cookies", &settings.cookie_file]);
            }
            video_cmd
                .args(["-f", &fmt])
                .arg("-o")
                .arg(video_dir.join("video.%(ext)s").to_string_lossy().as_ref())
                .args(["--no-overwrites", "--continue", "--no-warnings", "--force-ipv4"])
                .args(["--retries", "10", "--fragment-retries", "10"])
                .args(["--socket-timeout", "30", "--throttled-rate", "100K"])
                .args(["--file-access-retries", "5", "--retry-sleep", "3"])
                .args(["--buffer-size", "16K"])
                .args(["--extractor-retries", "3"])
                .args(["--concurrent-fragments", "4", "--progress"]);

            // Format-specific args
            let is_audio = matches!(settings.format.as_str(), "mp3" | "flac" | "wav" | "ogg" | "m4a");
            if is_audio {
                video_cmd.args(["--extract-audio", "--audio-format", &settings.format]);
            } else if settings.format != "mp4" {
                video_cmd.args(["--merge-output-format", &settings.format]);
            } else {
                video_cmd.args(["--merge-output-format", "mp4"]);
            }

            // Auto-tagging
            if settings.auto_tag && is_audio {
                let (artist, title, _genre) = parse_title_metadata(&video.title);
                if !artist.is_empty() {
                    video_cmd.args([
                        "--parse-metadata", &format!("title:{}", title),
                        "--parse-metadata", &format!("artist:{}", artist),
                    ]);
                }
            }

            // Subtitles
            if settings.download_subs && !is_audio {
                video_cmd.args(["--write-subs", "--convert-subs", "srt"]);
                if settings.auto_subs {
                    video_cmd.arg("--write-auto-subs");
                }
                let langs = settings.sub_langs.as_deref().unwrap_or("en");
                video_cmd.args(["--sub-langs", langs]);
                // Embed subs for mp4/webm/mkv
                if matches!(settings.format.as_str(), "mp4" | "webm" | "mkv") {
                    video_cmd.arg("--embed-subs");
                }
            }

            if let Some(ref p) = settings.proxy {
                video_cmd.args(["--proxy", p]);
            }

            video_cmd.arg(&video_url);

            // Spawn with piped stdout for real-time progress parsing
            video_cmd.stdout(std::process::Stdio::piped());
            video_cmd.stderr(std::process::Stdio::piped());

            if let Ok(mut child) = video_cmd.spawn() {
                let stdout = child.stdout.take();
                let stderr_handle = child.stderr.take();
                let progress_re = &*RE_PROGRESS;

                // Parse progress from stdout with 10-minute timeout
                let download_result = tokio::time::timeout(
                    std::time::Duration::from_secs(600),
                    async {
                        if let Some(stdout) = stdout {
                            let reader = BufReader::new(stdout);
                            let mut lines = reader.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                if let Some(caps) = progress_re.captures(&line) {
                                    let pct = &caps[1];
                                    app.emit("download-status", (idx, format!("downloading {}%", pct))).ok();
                                }
                            }
                        }
                        child.wait().await
                    }
                ).await;

                let exit_status = match download_result {
                    Ok(status) => status,
                    Err(_) => {
                        // Timeout — kill the process
                        app.emit("download-log", format!("  -> Timeout (10 min), killing process")).ok();
                        let _ = child.kill().await;
                        app.emit("download-status", (idx, "Failed".to_string())).ok();
                        continue;
                    }
                };

                // Read stderr for error messages
                let mut stderr_buf = String::new();
                if let Some(stderr) = stderr_handle {
                    let mut reader = BufReader::new(stderr);
                    let _ = reader.read_to_string(&mut stderr_buf).await;
                }

                let ext = &settings.format;
                let video_path = video_dir.join(format!("video.{}", ext));

                match exit_status {
                    Ok(s) if s.success() && video_path.exists() => {
                        let size_mb = video_path.metadata().map(|m| m.len()).unwrap_or(0) as f64
                            / (1024.0 * 1024.0);
                        video_ok += 1;
                        app.emit(
                            "download-log",
                            format!("  -> Video OK ({:.1} MB)", size_mb),
                        )
                        .ok();
                        app.emit("download-status", (idx, "done".to_string())).ok();

                        // Metadata injection
                        if settings.inject_metadata {
                            if let Err(e) = inject_metadata(&video_dir, &video.thumbnail, &video.title, &playlist_title, &settings.format) {
                                app.emit("download-log", format!("  -> Metadata error: {}", e)).ok();
                            }
                        }
                    }
                    _ => {
                        let err = if stderr_buf.contains("members-only")
                            || stderr_buf.contains("Join this channel")
                        {
                            "Members only"
                        } else if stderr_buf.contains("Video unavailable") || stderr_buf.contains("Private") {
                            "Unavailable"
                        } else if stderr_buf.contains("Sign in") || stderr_buf.contains("bot") || stderr_buf.contains("cookie") || stderr_buf.contains("HTTP Error 403") {
                            "Cookie expired"
                        } else {
                            "Failed"
                        };
                        for line in stderr_buf.lines().take(40) {
                            app.emit("download-log", format!("  | {}", line)).ok();
                        }
                        if err == "Cookie expired" || err == "Members only" {
                            app.emit("download-log", "  Hint: Your cookies may have expired. Please re-export from browser and paste again.".to_string()).ok();
                        }
                        app.emit("download-status", (idx, err.to_string())).ok();
                    }
                }
            }
        }
    }

    app.emit(
        "download-log",
        format!(
            "\nDone! Videos: {}/{} | Comments: {} ({} total)",
            video_ok, total, comment_ok, total_comments
        ),
    )
    .ok();

    // Generate index.html report
    generate_index_html(&playlist_title, &videos, &base_dir, video_ok, total_comments);
    app.emit("download-log", "Report saved: index.html".to_string()).ok();

    // Export comments if enabled
    if let Some(ref export_fmt) = settings.export_comments {
        match export_comments_to_file(&base_dir, export_fmt) {
            Ok(msg) => { app.emit("download-log", format!("Exported: {}", msg)).ok(); }
            Err(e) => { app.emit("download-log", format!("Export error: {}", e)).ok(); }
        }
    }

    app.emit("download-done", (video_ok, total, base_dir.to_string_lossy().to_string())).ok();

    Ok(())
}

#[tauri::command]
fn cancel_download(cancel: State<'_, CancelState>) {
    cancel.0.store(true, Ordering::SeqCst);
}

#[tauri::command]
fn check_existing_videos(
    output_dir: String,
    playlist_title: String,
    videos: Vec<VideoInfo>,
) -> Vec<bool> {
    let base_dir = PathBuf::from(&output_dir).join(sanitize_folder_name(&playlist_title));
    if !base_dir.exists() {
        return vec![false; videos.len()];
    }
    videos.iter().map(|v| {
        let folder = sanitize_folder_name(&v.title);
        let video_dir = base_dir.join(&folder);
        ["mp4","mp3","webm","mkv","avi","flac","wav","ogg","m4a"]
            .iter().any(|ext| video_dir.join(format!("video.{}", ext)).exists())
    }).collect()
}

#[tauri::command]
fn save_cookie_text(text: String) -> Result<String, String> {
    let temp_dir = std::env::temp_dir().join("yt-downloader-cookies");
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    let filename = format!("cookies_{}.txt", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis());
    let path = temp_dir.join(&filename);
    fs::write(&path, &text).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Entry ──────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let yt_path = if cfg!(target_os = "windows") {
        "yt-dlp.exe".into()
    } else {
        "yt-dlp".into()
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(CancelState(AtomicBool::new(false)))
        .manage(YtDlpPath(Mutex::new(yt_path)))
        .invoke_handler(tauri::generate_handler![
            check_ytdlp,
            install_ytdlp,
            fetch_playlist,
            start_download,
            cancel_download,
            check_existing_videos,
            save_cookie_text,
            open_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
