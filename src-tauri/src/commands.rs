use crate::htmlgen::{
    export_comments_to_file, generate_index_html, generate_video_comments_html,
};
use crate::types::*;
use crate::utils::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::{Emitter, State};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

// ── yt-dlp Management ──────────────────────────────────────────────────

#[tauri::command]
pub async fn check_ytdlp(path_state: State<'_, YtDlpPath>) -> Result<String, String> {
    let path = path_state.0.lock().await;
    let mut cmd = new_cmd(&*path);
    let output = cmd.arg("--version")
        .output()
        .await
        .map_err(|e| format!("yt-dlp not found: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err("yt-dlp check failed".into())
    }
}

#[tauri::command]
pub async fn install_ytdlp(path_state: State<'_, YtDlpPath>) -> Result<String, String> {
    if cfg!(target_os = "windows") {
        let app_data = std::env::var("APPDATA").unwrap_or_else(|_| {
            format!("C:\\Users\\{}\\AppData\\Roaming",
                std::env::var("USERNAME").unwrap_or_default())
        });
        let dir = PathBuf::from(&app_data).join("yt-playlist-downloader");
        fs::create_dir_all(&dir).map_err(|e| format!("Cannot create dir: {}", e))?;
        let exe_path = dir.join("yt-dlp.exe");
        let url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";

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

        let new_path = exe_path.to_string_lossy().to_string();
        *path_state.0.lock().await = new_path.clone();

        let version_output = new_cmd(&new_path)
            .arg("--version")
            .output()
            .await
            .map_err(|e| format!("yt-dlp version check failed: {}", e))?;
        Ok(String::from_utf8_lossy(&version_output.stdout).trim().to_string())
    } else {
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

// ── Playlist / Video Fetch ─────────────────────────────────────────────

#[tauri::command]
pub async fn fetch_playlist(
    url: String,
    cookie_file: String,
    proxy: Option<String>,
    path_state: State<'_, YtDlpPath>,
) -> Result<PlaylistResult, String> {
    let playlist_url = if url.contains("tiktok.com") {
        url.clone()
    } else if is_youtube_channel_url(&url) {
        url.clone()
    } else {
        match extract_playlist_id(&url) {
            Some(id) => format!("https://www.youtube.com/playlist?list={}", id),
            None => url.clone(),
        }
    };

    let yt_path = path_state.0.lock().await;
    let is_tiktok = url.contains("tiktok.com");
    let mut cmd = new_cmd(&*yt_path);
    cmd.args(yt_dlp_extra());
    if !cookie_file.is_empty() {
        cmd.args(["--cookies", &cookie_file]);
    }
    if is_tiktok {
        // --flat-playlist doesn't return thumbnails for TikTok, use --dump-json instead
        cmd.args(["--dump-json"]);
    } else {
        cmd.args(["--dump-json", "--flat-playlist"]);
    }
    cmd.arg(&playlist_url);

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
            let availability = data["availability"].as_str().unwrap_or("");
            let live_status = data["live_status"].as_str().unwrap_or("");
            if availability == "private"
                || availability == "premium_only"
                || live_status == "is_upcoming"
                || live_status == "post_live"
            {
                continue;
            }
            let title = data["title"].as_str().unwrap_or("");
            if title.is_empty()
                || title == "[Private video]"
                || title == "[Deleted video]"
            {
                continue;
            }
            let video_id = data["id"].as_str().unwrap_or("").to_string();
            let thumbnail = data["thumbnail"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    if is_tiktok {
                        String::new()
                    } else {
                        format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", video_id)
                    }
                });
            videos.push(VideoInfo {
                id: video_id,
                title: title.to_string(),
                channel: data["channel"].as_str().unwrap_or("").to_string(),
                duration: data["duration"].as_u64(),
                thumbnail,
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

    let video_id = data["id"].as_str().unwrap_or("").to_string();
    let thumbnail = data["thumbnail"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", video_id));
    Ok(VideoInfo {
        id: video_id,
        title: data["title"].as_str().unwrap_or("Unknown").to_string(),
        channel: data["channel"].as_str().unwrap_or("").to_string(),
        duration: data["duration"].as_u64(),
        thumbnail,
    })
}

// ── Main Download Command ──────────────────────────────────────────────

#[tauri::command]
pub async fn start_download(
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

    if settings.is_tiktok {
        let url = settings.playlist_url.clone();
        if let Some(caps) = RE_TIKTOK_USER.captures(&url) {
            let username = &caps[1];
            let tiktok_url = if url.contains("/video/") || url.contains("/@") && !url.trim_end_matches('/').ends_with(username) {
                url.clone()
            } else {
                format!("https://www.tiktok.com/@{}", username)
            };
            let result = fetch_playlist(
                tiktok_url,
                settings.cookie_file.clone(),
                settings.proxy.clone(),
                path_state.clone(),
            )
            .await?;
            playlist_title = format!("@{}", username);
            videos = result.videos;
        } else {
            let urls: Vec<&str> = url.split(|c: char| c.is_whitespace() || c == ',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            let mut all_videos = Vec::new();
            for tiktok_url in &urls {
                let mut cmd = new_cmd(&yt_path);
                cmd.args(yt_dlp_extra());
                cmd.args(["--dump-single-json", "--no-playlist"]);
                if settings.no_watermark {
                    cmd.args(["--extractor-args", "tiktok:video_codec=h264"]);
                }
                cmd.arg(tiktok_url);
                if let Some(ref p) = settings.proxy {
                    cmd.args(["--proxy", p]);
                }
                if let Ok(output) = cmd.output().await {
                    if output.status.success() {
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&output.stdout)) {
                            let video_id = data["id"].as_str().unwrap_or("").to_string();
                            all_videos.push(VideoInfo {
                                id: video_id,
                                title: data["title"].as_str().unwrap_or("TikTok").to_string(),
                                channel: data["channel"].as_str().unwrap_or("").to_string(),
                                duration: data["duration"].as_u64(),
                                thumbnail: data["thumbnail"].as_str().unwrap_or("").to_string(),
                            });
                        }
                    }
                }
            }
            playlist_title = "TikTok".to_string();
            videos = all_videos;
        }
        selected = if settings.selected_indices.is_empty() {
            (0..videos.len()).collect()
        } else {
            settings.selected_indices.clone()
        };
    } else if settings.single_video {
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
        let playlist_url = if is_youtube_channel_url(&settings.playlist_url) {
            settings.playlist_url.clone()
        } else {
            match extract_playlist_id(&settings.playlist_url) {
                Some(id) => format!("https://www.youtube.com/playlist?list={}", id),
                None => settings.playlist_url.clone(),
            }
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
            let check_slug = slugify(&video.title);
            let video_dir_check = if settings.flat_output {
                base_dir.clone()
            } else {
                base_dir.join(sanitize_folder_name(&video.title))
            };
            let video_exts = ["mp4","mp3","webm","mkv","avi","flac","wav","ogg","m4a"];
            let exists = video_exts.iter().any(|ext| {
                video_dir_check.join(format!("{}.{}", check_slug, ext)).exists()
                    || video_dir_check.join(format!("video.{}", ext)).exists()
            });
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
        app.emit("download-status", (idx, "downloading".to_string())).ok();
        app.emit("download-log", format!("[{}/{}] {}", idx, total, video.title)).ok();

        let file_slug = slugify(&video.title);
        let video_dir = if settings.flat_output {
            base_dir.clone()
        } else {
            let folder_name = sanitize_folder_name(&video.title);
            let dir = base_dir.join(&folder_name);
            fs::create_dir_all(&dir).ok();
            dir
        };

        // Clean up partial files from previous failed downloads
        if video_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&video_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let lower = name.to_lowercase();
                    if name.starts_with(&format!("{}.", file_slug)) {
                        let is_final = lower.ends_with(&format!(".{}", settings.format));
                        let is_media = ["mp4","mp3","webm","mkv","avi","flac","wav","ogg","m4a"].iter()
                            .any(|ext| lower.ends_with(&format!(".{}", ext)));
                        if is_media && !is_final {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }

        let video_url = if settings.is_tiktok && !video.id.contains("http") {
            format!("https://www.tiktok.com/@_/video/{}", video.id)
        } else if video.id.contains("http") {
            video.id.clone()
        } else {
            format!("https://www.youtube.com/watch?v={}", video.id)
        };

        // Download comments (if enabled, YouTube only)
        if settings.include_comments {
            let mut comment_cmd = new_cmd(&yt_path);
            comment_cmd.args(yt_dlp_extra());
            if !settings.cookie_file.is_empty() {
                comment_cmd.args(["--cookies", &settings.cookie_file]);
            }
            if let Some(ref p) = settings.proxy {
                comment_cmd.args(["--proxy", p]);
            }
            comment_cmd
                .args(["--write-comments", "--skip-download", "--no-warnings", "--force-ipv4"])
                .arg("-o")
                .arg(video_dir.join(format!("{}.%(ext)s", file_slug)).to_string_lossy().as_ref())
                .arg(&video_url);

            if let Ok(output) = comment_cmd.output().await {
                if output.status.success() {
                    comment_ok += 1;
                    let n = generate_video_comments_html(&video_dir, &video.title, &video.id, &video.channel, &file_slug, settings.flat_output);
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
            video_cmd.args(yt_dlp_extra());
            if !settings.cookie_file.is_empty() {
                video_cmd.args(["--cookies", &settings.cookie_file]);
            }
            if settings.is_tiktok && settings.no_watermark {
                video_cmd.args(["--extractor-args", "tiktok:video_codec=h264"]);
            }
            video_cmd
                .args(["-f", &fmt])
                .arg("-o")
                .arg(video_dir.join(format!("{}.%(ext)s", file_slug)).to_string_lossy().as_ref())
                .args(["--no-overwrites", "--continue", "--no-warnings", "--force-ipv4"])
                .args(["--retries", "10", "--fragment-retries", "10"])
                .args(["--socket-timeout", "30", "--throttled-rate", "100K"])
                .args(["--file-access-retries", "5", "--retry-sleep", "3"])
                .args(["--buffer-size", "16K"])
                .args(["--extractor-retries", "3"])
                .args(["--concurrent-fragments", &settings.max_concurrent.to_string(), "--progress", "--newline"]);
            if settings.write_info_json {
                video_cmd.arg("--write-info-json");
            }

            let is_audio = matches!(settings.format.as_str(), "mp3" | "flac" | "wav" | "ogg" | "m4a");
            if is_audio {
                video_cmd.args(["--extract-audio", "--audio-format", &settings.format]);
            } else if settings.format != "mp4" {
                video_cmd.args(["--merge-output-format", &settings.format]);
            } else {
                video_cmd.args(["--merge-output-format", "mp4"]);
            }

            if settings.auto_tag && is_audio {
                let (artist, title, _) = parse_title_metadata(&video.title);
                if !artist.is_empty() {
                    video_cmd.args([
                        "--parse-metadata", &format!("title:{}", title),
                        "--parse-metadata", &format!("artist:{}", artist),
                    ]);
                }
            }

            if settings.download_subs && !is_audio {
                video_cmd.args(["--write-subs", "--convert-subs", "srt"]);
                if settings.auto_subs {
                    video_cmd.arg("--write-auto-subs");
                }
                let langs = settings.sub_langs.as_deref().unwrap_or("en");
                video_cmd.args(["--sub-langs", langs]);
                if matches!(settings.format.as_str(), "mp4" | "webm" | "mkv") {
                    video_cmd.arg("--embed-subs");
                }
            }

            if let Some(ref p) = settings.proxy {
                video_cmd.args(["--proxy", p]);
            }

            video_cmd.arg(&video_url);
            video_cmd.stdout(std::process::Stdio::piped());
            video_cmd.stderr(std::process::Stdio::piped());

            if let Ok(mut child) = video_cmd.spawn() {
                let stdout = child.stdout.take();
                let stderr_handle = child.stderr.take();
                let progress_re = &*RE_PROGRESS;
                let app_clone = app.clone();

                let stderr_task = tokio::spawn(async move {
                    if let Some(stderr) = stderr_handle {
                        let mut reader = BufReader::new(stderr);
                        let mut buf = String::new();
                        let _ = reader.read_to_string(&mut buf).await;
                        buf
                    } else {
                        String::new()
                    }
                });

                let stdout_task = tokio::spawn({
                    let app_clone = app_clone.clone();
                    async move {
                        if let Some(stdout) = stdout {
                            let reader = BufReader::new(stdout);
                            let mut lines = reader.lines();
                            while let Ok(Some(line)) = lines.next_line().await {
                                for segment in line.split('\r') {
                                    if let Some(caps) = RE_PROGRESS_DETAIL.captures(segment) {
                                        let pct = caps[1].to_string();
                                        let size = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                                        let speed = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                                        let eta = caps.get(4).map(|m| m.as_str()).unwrap_or("");
                                        app_clone.emit("download-status", (idx, format!("downloading {}%", &pct))).ok();
                                        app_clone.emit("video-progress", (idx, format!("{}%", &pct), speed.to_string(), eta.to_string(), size.to_string())).ok();
                                    } else if let Some(caps) = progress_re.captures(segment) {
                                        let pct = &caps[1];
                                        app_clone.emit("download-status", (idx, format!("downloading {}%", pct))).ok();
                                    }
                                }
                            }
                        }
                    }
                });

                let exit_result = tokio::time::timeout(
                    std::time::Duration::from_secs(600),
                    child.wait()
                ).await;

                stdout_task.abort();

                let exit_status = match exit_result {
                    Ok(status) => status,
                    Err(_) => {
                        app.emit("download-log", "  -> Timeout (10 min), killing process".to_string()).ok();
                        let _ = child.kill().await;
                        let _ = stderr_task.abort();
                        app.emit("download-status", (idx, "Failed".to_string())).ok();
                        continue;
                    }
                };

                let stderr_buf = stderr_task.await.unwrap_or_default();

                let ext = &settings.format;
                let video_path = {
                    let ideal = video_dir.join(format!("{}.{}", file_slug, ext));
                    if ideal.exists() {
                        ideal
                    } else if let Ok(entries) = fs::read_dir(&video_dir) {
                        let mut found = None;
                        for entry in entries.flatten() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if name.starts_with(&format!("{}.", file_slug)) || name.starts_with("video.") {
                                let lower = name.to_lowercase();
                                if lower.ends_with(&format!(".{}", ext)) {
                                    found = Some(entry.path());
                                    break;
                                }
                            }
                        }
                        found.unwrap_or(ideal)
                    } else {
                        ideal
                    }
                };

                match exit_status {
                    Ok(s) if s.success() && video_path.exists() => {
                        let clean_path = video_dir.join(format!("{}.{}", file_slug, ext));
                        if video_path != clean_path {
                            let _ = fs::rename(&video_path, &clean_path);
                        }
                        let final_path = if clean_path.exists() { &clean_path } else { &video_path };
                        let size_mb = final_path.metadata().map(|m| m.len()).unwrap_or(0) as f64
                            / (1024.0 * 1024.0);
                        video_ok += 1;
                        app.emit("download-log", format!("  -> Video OK ({:.1} MB)", size_mb)).ok();
                        app.emit("download-status", (idx, "done".to_string())).ok();

                        if settings.inject_metadata {
                            if let Err(e) = inject_metadata(&video_dir, &video.thumbnail, &video.title, &playlist_title, &settings.format) {
                                app.emit("download-log", format!("  -> Metadata error: {}", e)).ok();
                            }
                        }
                    }
                    _ => {
                        // Retry on 503 / network error
                        let is_503 = stderr_buf.contains("HTTP Error 503");
                        let is_network_error = stderr_buf.contains("Giving up after") || stderr_buf.contains("bytes read");
                        let should_retry = (is_503 || is_network_error) && !is_audio;
                        if should_retry {
                            let reason = if is_503 { "Server busy (503)" } else { "Network error" };
                            app.emit("download-log", format!("  -> {}, retrying with single-stream format...", reason)).ok();
                            if !settings.flat_output {
                                let _ = fs::remove_dir_all(&video_dir);
                                let _ = fs::create_dir_all(&video_dir);
                            } else {
                                if let Ok(entries) = fs::read_dir(&video_dir) {
                                    for entry in entries.flatten() {
                                        let name = entry.file_name().to_string_lossy().to_string();
                                        if name.starts_with(&format!("{}.", file_slug)) {
                                            let _ = fs::remove_file(entry.path());
                                        }
                                    }
                                }
                            }

                            let mut retry_cmd = new_cmd(&yt_path);
                            retry_cmd.args(yt_dlp_extra());
                            if !settings.cookie_file.is_empty() {
                                retry_cmd.args(["--cookies", &settings.cookie_file]);
                            }
                            retry_cmd
                                .args(["-f", "best"])
                                .arg("-o")
                                .arg(video_dir.join(format!("{}.%(ext)s", file_slug)).to_string_lossy().as_ref())
                                .args(["--no-overwrites", "--no-warnings", "--force-ipv4"])
                                .args(["--retries", "3", "--fragment-retries", "3"])
                                .args(["--socket-timeout", "30", "--throttled-rate", "100K"])
                                .args(["--extractor-retries", "2"])
                                .args(["--progress", "--newline"])
                                .args(["--merge-output-format", &settings.format]);
                            if let Some(ref p) = settings.proxy {
                                retry_cmd.args(["--proxy", p]);
                            }
                            retry_cmd.arg(&video_url);
                            retry_cmd.stdout(std::process::Stdio::piped());
                            retry_cmd.stderr(std::process::Stdio::piped());

                            if let Ok(mut retry_child) = retry_cmd.spawn() {
                                let retry_stdout = retry_child.stdout.take();
                                let retry_stderr = retry_child.stderr.take();
                                let retry_re = &*RE_PROGRESS;
                                let retry_app = app.clone();

                                let retry_stderr_task = tokio::spawn(async move {
                                    if let Some(stderr) = retry_stderr {
                                        let mut reader = BufReader::new(stderr);
                                        let mut buf = String::new();
                                        let _ = reader.read_to_string(&mut buf).await;
                                        buf
                                    } else {
                                        String::new()
                                    }
                                });

                                let retry_stdout_task = tokio::spawn({
                                    let retry_app = retry_app.clone();
                                    async move {
                                        if let Some(stdout) = retry_stdout {
                                            let reader = BufReader::new(stdout);
                                            let mut lines = reader.lines();
                                            while let Ok(Some(line)) = lines.next_line().await {
                                                for seg in line.split('\r') {
                                                    if let Some(caps) = retry_re.captures(seg) {
                                                        retry_app.emit("download-status", (idx, format!("downloading {}%", &caps[1]))).ok();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });

                                let retry_exit = tokio::time::timeout(
                                    std::time::Duration::from_secs(300),
                                    retry_child.wait()
                                ).await;
                                retry_stdout_task.abort();

                                let retry_stderr_buf = retry_stderr_task.await.unwrap_or_default();

                                match retry_exit {
                                    Ok(Ok(s)) if s.success() => {
                                        let retry_path = {
                                            let ideal = video_dir.join(format!("{}.{}", file_slug, ext));
                                            if ideal.exists() {
                                                ideal
                                            } else if let Ok(entries) = fs::read_dir(&video_dir) {
                                                entries.flatten().find_map(|entry| {
                                                    let name = entry.file_name().to_string_lossy().to_string();
                                                    if (name.starts_with(&format!("{}.", file_slug)) || name.starts_with("video."))
                                                        && name.to_lowercase().ends_with(&format!(".{}", ext))
                                                    { Some(entry.path()) } else { None }
                                                }).unwrap_or(ideal)
                                            } else { video_dir.join(format!("{}.{}", file_slug, ext)) }
                                        };
                                        if retry_path.exists() {
                                            let clean = video_dir.join(format!("{}.{}", file_slug, ext));
                                            if retry_path != clean { let _ = fs::rename(&retry_path, &clean); }
                                            let fp = if clean.exists() { &clean } else { &retry_path };
                                            let sz = fp.metadata().map(|m| m.len()).unwrap_or(0) as f64 / (1024.0 * 1024.0);
                                            video_ok += 1;
                                            app.emit("download-log", format!("  -> Video OK via fallback ({:.1} MB)", sz)).ok();
                                            app.emit("download-status", (idx, "done".to_string())).ok();
                                            if settings.include_comments {
                                                let mut comment_cmd = new_cmd(&yt_path);
                                                comment_cmd.args(yt_dlp_extra());
                                                if !settings.cookie_file.is_empty() {
                                                    comment_cmd.args(["--cookies", &settings.cookie_file]);
                                                }
                                                comment_cmd
                                                    .args(["--no-warnings", "--force-ipv4", "--skip-download"])
                                                    .args(["--write-comments", "--print", "none"])
                                                    .arg(&video_url);
                                                if let Some(ref p) = settings.proxy {
                                                    comment_cmd.args(["--proxy", p]);
                                                }
                                                if let Ok(output) = comment_cmd.output().await {
                                                    if output.status.success() {
                                                        let n = generate_video_comments_html(&video_dir, &video.title, &video.id, &video.channel, &file_slug, settings.flat_output);
                                                        total_comments += n as u32;
                                                        app.emit("download-log", format!("  -> {} comments", n)).ok();
                                                    }
                                                }
                                            }
                                            if settings.inject_metadata {
                                                if let Err(e) = inject_metadata(&video_dir, &video.thumbnail, &video.title, &playlist_title, &settings.format) {
                                                    app.emit("download-log", format!("  -> Metadata error: {}", e)).ok();
                                                }
                                            }
                                            continue;
                                        }
                                    }
                                    _ => {
                                        for line in retry_stderr_buf.lines().take(10) {
                                            app.emit("download-log", format!("  | {}", line)).ok();
                                        }
                                    }
                                }
                            }
                        }

                        let err = if stderr_buf.contains("members-only")
                            || stderr_buf.contains("Join this channel")
                        {
                            "Members only"
                        } else if stderr_buf.contains("Video unavailable") || stderr_buf.contains("Private") {
                            "Unavailable"
                        } else if stderr_buf.contains("Sign in") || stderr_buf.contains("bot") || stderr_buf.contains("cookie") || stderr_buf.contains("HTTP Error 403") {
                            "Cookie expired"
                        } else if is_503 {
                            "Server busy"
                        } else if is_network_error {
                            "Network error"
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

    generate_index_html(&playlist_title, &videos, &base_dir, video_ok, total_comments, settings.flat_output);
    app.emit("download-log", "Report saved: index.html".to_string()).ok();

    if let Some(ref export_fmt) = settings.export_comments {
        match export_comments_to_file(&base_dir, export_fmt) {
            Ok(msg) => { app.emit("download-log", format!("Exported: {}", msg)).ok(); }
            Err(e) => { app.emit("download-log", format!("Export error: {}", e)).ok(); }
        }
    }

    app.emit("download-done", (video_ok, total, base_dir.to_string_lossy().to_string())).ok();

    Ok(())
}

// ── Cancel ─────────────────────────────────────────────────────────────

#[tauri::command]
pub fn cancel_download(cancel: State<'_, CancelState>) {
    cancel.0.store(true, Ordering::SeqCst);
}

// ── Check Existing ─────────────────────────────────────────────────────

#[tauri::command]
pub fn check_existing_videos(
    output_dir: String,
    playlist_title: String,
    videos: Vec<VideoInfo>,
    flat_output: bool,
) -> Vec<bool> {
    let base_dir = if flat_output {
        PathBuf::from(&output_dir)
    } else {
        PathBuf::from(&output_dir).join(sanitize_folder_name(&playlist_title))
    };
    if !base_dir.exists() {
        return vec![false; videos.len()];
    }
    let video_exts = ["mp4","mp3","webm","mkv","avi","flac","wav","ogg","m4a"];
    videos.iter().map(|v| {
        let file_prefix = slugify(&v.title);
        let video_dir = if flat_output { base_dir.clone() } else { base_dir.join(sanitize_folder_name(&v.title)) };
        if !video_dir.is_dir() { return false; }
        if let Ok(entries) = fs::read_dir(&video_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&format!("{}.", file_prefix)) || name.starts_with("video.") {
                    let lower = name.to_lowercase();
                    if video_exts.iter().any(|ext| lower.ends_with(&format!(".{}", ext))) {
                        return true;
                    }
                }
            }
        }
        false
    }).collect()
}

// ── Cookie & Folder ────────────────────────────────────────────────────

#[tauri::command]
pub fn save_cookie_text(text: String) -> Result<String, String> {
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
pub fn open_folder(path: String) -> Result<(), String> {
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
