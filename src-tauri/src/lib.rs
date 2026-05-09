use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, State};
use tokio::process::Command;
use tokio::sync::Mutex;

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
    comments_only: bool,
    auto_tag: bool,
    selected_indices: Vec<usize>,
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
        // Remove common suffixes like (Official Video), [MV], etc.
        let re = regex::Regex::new(r#"(?i)\s*[\(\[{].*?(official|video|mv|music|lyric|audio|4k|hd|1080p).*?[\)\]}]"#).unwrap();
        let clean_title = re.replace_all(rest, "").trim().to_string();
        let genre = String::new();
        return (artist, clean_title, genre);
    }
    (String::new(), title.to_string(), String::new())
}

fn sanitize_folder_name(name: &str) -> String {
    let re = regex::Regex::new(r#"[<>:"/\\|?*]"#).unwrap();
    let cleaned = re.replace_all(name, "").to_string();
    let trimmed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() > 200 {
        trimmed[..200].to_string()
    } else {
        trimmed
    }
}

fn extract_playlist_id(url: &str) -> Option<String> {
    let re = regex::Regex::new(r"[?&]list=([a-zA-Z0-9_-]+)").ok()?;
    if let Some(caps) = re.captures(url) {
        return Some(caps[1].to_string());
    }
    if regex::Regex::new(r"^[a-zA-Z0-9_-]+$")
        .unwrap()
        .is_match(url.trim())
    {
        return Some(url.trim().to_string());
    }
    None
}

// ── Commands ───────────────────────────────────────────────────────────

#[tauri::command]
async fn check_ytdlp(path_state: State<'_, YtDlpPath>) -> Result<String, String> {
    let path = path_state.0.lock().await;
    let output = Command::new(&*path)
        .arg("--version")
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
    let python = if cfg!(target_os = "windows") {
        "python"
    } else {
        "python3"
    };
    let output = Command::new(python)
        .args(["-m", "pip", "install", "--upgrade", "yt-dlp"])
        .output()
        .await
        .map_err(|e| format!("pip failed: {}", e))?;
    if output.status.success() {
        let path = path_state.0.lock().await;
        let version_output = Command::new(&*path)
            .arg("--version")
            .output()
            .await
            .map_err(|e| format!("yt-dlp version check failed: {}", e))?;
        Ok(String::from_utf8_lossy(&version_output.stdout)
            .trim()
            .to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
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
    let mut cmd = Command::new(&*yt_path);
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
            videos.push(VideoInfo {
                id: data["id"].as_str().unwrap_or("").to_string(),
                title: data["title"].as_str().unwrap_or("Unknown").to_string(),
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

    let playlist_url = match extract_playlist_id(&settings.playlist_url) {
        Some(id) => format!("https://www.youtube.com/playlist?list={}", id),
        None => settings.playlist_url.clone(),
    };

    // Fetch playlist
    let result = fetch_playlist(
        playlist_url,
        settings.cookie_file.clone(),
        settings.proxy.clone(),
        path_state.clone(),
    )
    .await?;

    let total = result.videos.len();
    app.emit(
        "download-log",
        format!("Playlist: {} ({} videos)", result.title, total),
    )
    .ok();

    let selected: Vec<usize> = if settings.selected_indices.is_empty() {
        (0..result.videos.len()).collect()
    } else {
        settings.selected_indices.clone()
    };

    for (i, video) in result.videos.iter().enumerate() {
        if !selected.contains(&i) {
            app.emit("download-status", (i + 1, "Skipped".to_string())).ok();
            continue;
        }
        if cancel.0.load(Ordering::SeqCst) {
            app.emit("download-log", "Cancelled.".to_string()).ok();
            break;
        }

        let idx = i + 1;
        app.emit("download-progress", (idx, total)).ok();
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

        // Download comments
        let mut comment_cmd = Command::new(&yt_path);
        comment_cmd
            .args(yt_dlp_extra());
        if !settings.cookie_file.is_empty() {
            comment_cmd.args(["--cookies", &settings.cookie_file]);
        }
        comment_cmd
            .args(["--write-comments", "--skip-download", "--no-warnings"])
            .arg("-o")
            .arg(video_dir.join("video.%(ext)s").to_string_lossy().as_ref())
            .arg(&video_url);

        if let Some(ref p) = settings.proxy {
            comment_cmd.args(["--proxy", p]);
        }

        if let Ok(output) = comment_cmd.output().await {
            if output.status.success() {
                comment_ok += 1;
                let stderr = String::from_utf8_lossy(&output.stderr);
                if let Some(caps) =
                    regex::Regex::new(r"Extracted (\d+) comments").unwrap().captures(&stderr)
                {
                    if let Ok(n) = caps[1].parse::<u32>() {
                        total_comments += n;
                        app.emit(
                            "download-log",
                            format!("  -> {} comments", n),
                        )
                        .ok();
                    }
                }
            }
        }

        if cancel.0.load(Ordering::SeqCst) {
            break;
        }

        // Download video
        if !settings.comments_only {
            let mut video_cmd = Command::new(&yt_path);
            video_cmd
                .args(yt_dlp_extra());
            if !settings.cookie_file.is_empty() {
                video_cmd.args(["--cookies", &settings.cookie_file]);
            }
            video_cmd
                .args(["-f", &fmt])
                .arg("-o")
                .arg(video_dir.join("video.%(ext)s").to_string_lossy().as_ref())
                .args(["--no-overwrites", "--continue", "--no-warnings"])
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

            video_cmd.arg(&video_url);

            if let Some(ref p) = settings.proxy {
                video_cmd.args(["--proxy", p]);
            }

            if let Ok(output) = video_cmd.output().await {
                let ext = &settings.format;
                let video_path = video_dir.join(format!("video.{}", ext));
                if output.status.success() && video_path.exists() {
                    let size_mb = video_path.metadata().map(|m| m.len()).unwrap_or(0) as f64
                        / (1024.0 * 1024.0);
                    video_ok += 1;
                    app.emit(
                        "download-log",
                        format!("  -> Video OK ({:.1} MB)", size_mb),
                    )
                    .ok();
                    app.emit("download-status", (idx, "done".to_string())).ok();
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let err = if stderr.contains("members-only")
                        || stderr.contains("Join this channel")
                    {
                        "Members only"
                    } else if stderr.contains("Video unavailable") || stderr.contains("Private") {
                        "Unavailable"
                    } else {
                        "Failed"
                    };
                    app.emit(
                        "download-log",
                        format!("  -> {}", err),
                    )
                    .ok();
                    app.emit("download-status", (idx, err.to_string())).ok();
                }
            }
        } else {
            app.emit("download-status", (idx, "done".to_string())).ok();
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
    app.emit("download-done", (video_ok, total)).ok();

    Ok(())
}

#[tauri::command]
fn cancel_download(cancel: State<'_, CancelState>) {
    cancel.0.store(true, Ordering::SeqCst);
}

#[tauri::command]
fn save_cookie_text(text: String) -> Result<String, String> {
    let temp_dir = std::env::temp_dir().join("yt-downloader-cookies");
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    let path = temp_dir.join("cookies.txt");
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
            save_cookie_text,
            open_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
