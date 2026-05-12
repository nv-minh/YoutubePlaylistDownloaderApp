use crate::htmlgen::{
    export_comments_to_file, generate_index_html, generate_video_comments_html,
};
use crate::types::*;
use crate::utils::*;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tauri::{Emitter, State};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::sync::Semaphore;

// ── curl_cffi install helper ────────────────────────────────────────────

async fn try_install_curl_cffi() -> bool {
    let candidates: Vec<&str> = if cfg!(target_os = "windows") {
        vec!["python", "python3", "py"]
    } else {
        vec!["python3", "python", "/usr/bin/python3", "/usr/local/bin/python3"]
    };
    for py in &candidates {
        let ok = new_cmd(py)
            .args(["-m", "pip", "install", "--upgrade", "curl_cffi"])
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            return true;
        }
    }
    false
}

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
pub async fn check_impersonate(
    path_state: State<'_, YtDlpPath>,
    imp_state: State<'_, ImpersonateSupport>,
) -> Result<bool, String> {
    let yt_path = path_state.0.lock().await;
    let output = new_cmd(&*yt_path)
        .args(["--list-impersonate-targets"])
        .output()
        .await
        .map_err(|e| format!("check failed: {}", e))?;
    let supported = output.status.success()
        && String::from_utf8_lossy(&output.stdout).contains("chrome");
    imp_state.0.store(supported, Ordering::SeqCst);
    Ok(supported)
}

#[tauri::command]
pub async fn install_curl_cffi(
    path_state: State<'_, YtDlpPath>,
    imp_state: State<'_, ImpersonateSupport>,
) -> Result<bool, String> {
    let ok = try_install_curl_cffi().await;
    if ok {
        let yt_path = path_state.0.lock().await;
        let output = new_cmd(&*yt_path)
            .args(["--list-impersonate-targets"])
            .output()
            .await
            .map_err(|e| format!("check failed: {}", e))?;
        let supported = output.status.success()
            && String::from_utf8_lossy(&output.stdout).contains("chrome");
        imp_state.0.store(supported, Ordering::SeqCst);
        Ok(supported)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn install_ytdlp(
    path_state: State<'_, YtDlpPath>,
    imp_state: State<'_, ImpersonateSupport>,
) -> Result<String, String> {
    // Try pip install first (includes curl_cffi for TikTok impersonation)
    let python = if cfg!(target_os = "windows") { "python" } else { "python3" };
    let pip_ok = new_cmd(python)
        .args(["-m", "pip", "install", "--upgrade", "yt-dlp", "curl_cffi"])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    if pip_ok {
        let _yt_name = if cfg!(target_os = "windows") { "yt-dlp.exe" } else { "yt-dlp" };
        let path = path_state.0.lock().await;
        let version_output = new_cmd(&*path)
            .arg("--version")
            .output()
            .await
            .map_err(|e| format!("yt-dlp version check failed: {}", e))?;
        if version_output.status.success() {
            // Update impersonation support state
            if let Ok(imp_output) = new_cmd(&*path).args(["--list-impersonate-targets"]).output().await {
                let supported = imp_output.status.success()
                    && String::from_utf8_lossy(&imp_output.stdout).contains("chrome");
                imp_state.0.store(supported, Ordering::SeqCst);
            }
            return Ok(String::from_utf8_lossy(&version_output.stdout).trim().to_string());
        }
    }

    // Fallback: download binary directly
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
                return Err("Failed to download yt-dlp. Install manually: pip install yt-dlp curl_cffi".into());
            }
        }

        let new_path = exe_path.to_string_lossy().to_string();
        *path_state.0.lock().await = new_path.clone();
        // Best-effort curl_cffi install for TikTok impersonation
        let _ = try_install_curl_cffi().await;
        let version_output = new_cmd(&new_path)
            .arg("--version")
            .output()
            .await
            .map_err(|e| format!("yt-dlp version check failed: {}", e))?;
        // Update impersonation support state
        if let Ok(imp_output) = new_cmd(&new_path).args(["--list-impersonate-targets"]).output().await {
            let supported = imp_output.status.success()
                && String::from_utf8_lossy(&imp_output.stdout).contains("chrome");
            imp_state.0.store(supported, Ordering::SeqCst);
        }
        Ok(String::from_utf8_lossy(&version_output.stdout).trim().to_string())
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let local_bin = PathBuf::from(&home).join(".local/bin");
        let _ = fs::create_dir_all(&local_bin);
        let bin_path = local_bin.join("yt-dlp");
        let url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp";

            let curl_ok = new_cmd("curl")
                .args(["-sL", url, "-o"])
                .arg(&bin_path)
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);

            if curl_ok && bin_path.exists() {
                let _ = std::process::Command::new("chmod").arg("+x").arg(&bin_path).output();
                let new_path = bin_path.to_string_lossy().to_string();
                *path_state.0.lock().await = new_path.clone();
                // Best-effort curl_cffi install for TikTok impersonation
                let _ = try_install_curl_cffi().await;
                let version_output = new_cmd(&new_path)
                    .arg("--version")
                    .output()
                    .await
                    .map_err(|e| format!("yt-dlp version check failed: {}", e))?;
                // Update impersonation support state
                if let Ok(imp_output) = new_cmd(&new_path).args(["--list-impersonate-targets"]).output().await {
                    let supported = imp_output.status.success()
                        && String::from_utf8_lossy(&imp_output.stdout).contains("chrome");
                    imp_state.0.store(supported, Ordering::SeqCst);
                }
                Ok(String::from_utf8_lossy(&version_output.stdout).trim().to_string())
            } else {
                Err("Failed to install yt-dlp. Install manually: pip install yt-dlp or download from https://github.com/yt-dlp/yt-dlp".into())
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
    imp_state: State<'_, ImpersonateSupport>,
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
        cmd.args(["--dump-json"]);
        if imp_state.0.load(Ordering::SeqCst) {
            cmd.args(["--impersonate", "chrome"]);
        }
        cmd.args(["--extractor-args", "tiktok:api_hostname=api16-normal-c-useast1a.tiktokv.com"]);
    } else {
        cmd.args(["--dump-json", "--flat-playlist"]);
    }
    cmd.arg(&playlist_url);

    if let Some(ref p) = proxy {
        cmd.args(["--proxy", p]);
    }

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        cmd.output()
    ).await
        .map_err(|_| "Timeout: playlist fetch took longer than 5 minutes".to_string())?
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

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        cmd.output()
    ).await
        .map_err(|_| "Timeout: video info fetch took longer than 2 minutes".to_string())?
        .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;
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
    imp_state: State<'_, ImpersonateSupport>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    cancel.0.store(false, Ordering::SeqCst);

    let yt_path = path_state.0.lock().await.clone();
    let fmt = quality_format(&settings.quality, &settings.format);
    let base_dir = PathBuf::from(&settings.output_dir);
    fs::create_dir_all(&base_dir).map_err(|e| e.to_string())?;

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
                imp_state.clone(),
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
                if imp_state.0.load(Ordering::SeqCst) {
                    cmd.args(["--impersonate", "chrome"]);
                }
                cmd.args(["--extractor-args", "tiktok:api_hostname=api16-normal-c-useast1a.tiktokv.com"]);
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
    } else if settings.playlist_url.contains('\n') {
        // Multi-video mode: split by newline, fetch each
        let urls: Vec<&str> = settings.playlist_url
            .split('\n')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let mut all_videos = Vec::new();
        for video_url in &urls {
            match fetch_single_video(
                video_url.to_string(),
                settings.cookie_file.clone(),
                settings.proxy.clone(),
                path_state.clone(),
            ).await {
                Ok(video) => all_videos.push(video),
                Err(e) => {
                    app.emit("download-log", format!("Failed to fetch {}: {}", video_url, e)).ok();
                }
            }
        }
        if all_videos.is_empty() {
            return Err("Could not fetch info for any of the provided URLs".into());
        }
        playlist_title = format!("{} videos", all_videos.len());
        videos = all_videos;
        selected = (0..videos.len()).collect();
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
            imp_state.clone(),
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
        let playlist_folder = base_dir.join(sanitize_path_for_os(&playlist_title));
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
    let completed = Arc::new(AtomicU32::new(0));
    let video_ok = Arc::new(AtomicU32::new(0));
    let comment_ok = Arc::new(AtomicU32::new(0));
    let total_comments = Arc::new(AtomicU32::new(0));
    let max_concurrent = settings.max_concurrent.max(1) as usize;
    let sem = Arc::new(Semaphore::new(max_concurrent));

    let mut tasks = tokio::task::JoinSet::new();

    for (i, video) in videos.iter().enumerate() {
        if !selected.contains(&i) {
            app.emit("download-status", (i + 1, "Skipped".to_string())).ok();
            continue;
        }

        // Update mode: skip already downloaded videos
        if settings.update_mode {
            let check_slug = slugify(&video.title);
            let video_dir_check = if settings.flat_output {
                base_dir.clone()
            } else {
                base_dir.join(sanitize_path_for_os(&video.title))
            };
            let video_exts = ["mp4","mp3","webm","mkv","avi","flac","wav","ogg","m4a"];
            let exists = video_exts.iter().any(|ext| {
                video_dir_check.join(format!("{}.{}", check_slug, ext)).exists()
                    || video_dir_check.join(format!("video.{}", ext)).exists()
            });
            if exists {
                app.emit("download-status", (i + 1, "Exists".to_string())).ok();
                app.emit("download-log", format!("[{}] {} - already exists, skipping", i + 1, video.title)).ok();
                video_ok.fetch_add(1, Ordering::SeqCst);
                continue;
            }
        }

        let idx = i + 1;
        let c = completed.fetch_add(1, Ordering::SeqCst) + 1;
        app.emit("download-progress", (c, selected_count)).ok();
        app.emit("download-status", (idx, "pending".to_string())).ok();
        app.emit("download-log", format!("[{}/{}] {} - queued", idx, total, video.title)).ok();

        // Clone everything needed for the async task
        let video = video.clone();
        let yt_path = yt_path.clone();
        let fmt = fmt.clone();
        let settings = settings.clone();
        let base_dir = base_dir.clone();
        let playlist_title = playlist_title.clone();
        let app = app.clone();
        let cancel = cancel.0.clone();
        let sem = sem.clone();
        let video_ok_c = video_ok.clone();
        let comment_ok_c = comment_ok.clone();
        let total_comments_c = total_comments.clone();
        let imp_supported = imp_state.0.clone();

        tasks.spawn(async move {
            // Acquire semaphore permit — limits concurrent downloads
            let _permit = match sem.acquire().await {
                Ok(p) => p,
                Err(_) => return, // Semaphore closed, bail out
            };

            // Stagger start to avoid triggering bot detection
            let jitter = (idx as u64 % 3) * 2;
            tokio::time::sleep(std::time::Duration::from_secs(jitter)).await;

            if cancel.load(Ordering::SeqCst) {
                return;
            }

            app.emit("download-status", (idx, "downloading".to_string())).ok();
            app.emit("download-log", format!("[{}/{}] {}", idx, total, video.title)).ok();

            let file_slug = slugify(&video.title);
            let video_dir = if settings.flat_output {
                base_dir.clone()
            } else {
                let folder_name = sanitize_path_for_os(&video.title);
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
                        comment_ok_c.fetch_add(1, Ordering::SeqCst);
                        let n = generate_video_comments_html(&video_dir, &video.title, &video.id, &video.channel, &file_slug, settings.flat_output);
                        total_comments_c.fetch_add(n as u32, Ordering::SeqCst);
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

            if cancel.load(Ordering::SeqCst) {
                return;
            }

            // Download video
            let mut video_cmd = new_cmd(&yt_path);
            video_cmd.args(yt_dlp_extra());
            if !settings.cookie_file.is_empty() {
                video_cmd.args(["--cookies", &settings.cookie_file]);
            }
            if settings.is_tiktok {
                let mut tiktok_args = String::from("tiktok:api_hostname=api16-normal-c-useast1a.tiktokv.com");
                if settings.no_watermark {
                    tiktok_args.push_str(";video_codec=h264");
                }
                video_cmd.args(["--extractor-args", &tiktok_args]);
                if imp_supported.load(Ordering::SeqCst) {
                    video_cmd.args(["--impersonate", "chrome"]);
                }
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
                .args(["--sleep-requests", "1"])
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

            // Subtitles are downloaded in a separate step after video succeeds,
            // so subtitle errors (e.g. 429) don't block the video download.

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

                let download_timeout = if video.duration.unwrap_or(0) > 3600 {
                    std::time::Duration::from_secs(1800) // 30 min for long videos (>1h)
                } else {
                    std::time::Duration::from_secs(600) // 10 min for regular videos
                };

                let exit_result = tokio::time::timeout(
                    download_timeout,
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
                        return;
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
                    Ok(_exit_s) if video_path.exists() => {
                        let clean_path = video_dir.join(format!("{}.{}", file_slug, ext));
                        if video_path != clean_path {
                            let _ = fs::rename(&video_path, &clean_path);
                        }
                        let final_path = if clean_path.exists() { &clean_path } else { &video_path };
                        let size_mb = final_path.metadata().map(|m| m.len()).unwrap_or(0) as f64
                            / (1024.0 * 1024.0);
                        video_ok_c.fetch_add(1, Ordering::SeqCst);
                        app.emit("download-log", format!("  -> Video OK ({:.1} MB)", size_mb)).ok();
                        app.emit("download-status", (idx, "done".to_string())).ok();

                        if settings.inject_metadata {
                            if let Err(e) = inject_metadata(&video_dir, &video.thumbnail, &video.title, &playlist_title, &settings.format) {
                                app.emit("download-log", format!("  -> Metadata error: {}", e)).ok();
                            }
                        }

                        // Download subtitles separately so subtitle errors don't block the video
                        if settings.download_subs && !is_audio {
                            let mut sub_cmd = new_cmd(&yt_path);
                            sub_cmd.args(yt_dlp_extra());
                            if !settings.cookie_file.is_empty() {
                                sub_cmd.args(["--cookies", &settings.cookie_file]);
                            }
                            sub_cmd.args(["--write-subs", "--convert-subs", "srt", "--skip-download"]);
                            if settings.auto_subs {
                                sub_cmd.arg("--write-auto-subs");
                            }
                            let langs = settings.sub_langs.as_deref().unwrap_or("en");
                            sub_cmd.args(["--sub-langs", langs]);
                            if matches!(settings.format.as_str(), "mp4" | "webm" | "mkv") {
                                sub_cmd.arg("--embed-subs");
                            }
                            sub_cmd.args(["--no-overwrites", "--no-warnings", "--force-ipv4"]);
                            sub_cmd.args(["--sleep-requests", "1"]);
                            if let Some(ref p) = settings.proxy {
                                sub_cmd.args(["--proxy", p]);
                            }
                            sub_cmd.arg("-o").arg(video_dir.join(format!("{}.%(ext)s", file_slug)).to_string_lossy().as_ref());
                            sub_cmd.arg(&video_url);
                            match tokio::time::timeout(std::time::Duration::from_secs(120), sub_cmd.output()).await {
                                Ok(Ok(out)) if out.status.success() => {
                                    app.emit("download-log", "  -> Subtitles OK".to_string()).ok();
                                }
                                _ => {
                                    app.emit("download-log", "  -> Subtitles skipped (unavailable or rate limited)".to_string()).ok();
                                }
                            }
                        }
                    }
                    _ => {
                        // Retry on 503 / network error / TikTok no formats
                        let is_503 = stderr_buf.contains("HTTP Error 503");
                        let is_network_error = stderr_buf.contains("Giving up after") || stderr_buf.contains("bytes read");
                        let is_no_formats = stderr_buf.contains("No video formats found");
                        let should_retry = (is_503 || is_network_error || (settings.is_tiktok && is_no_formats)) && !is_audio;
                        if should_retry {
                            let reason = if is_503 { "Server busy (503)" }
                                else if settings.is_tiktok && is_no_formats { "No video formats found, retrying without custom API hostname..." }
                                else { "Network error" };
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
                            if settings.is_tiktok {
                                if settings.no_watermark {
                                    retry_cmd.args(["--extractor-args", "tiktok:video_codec=h264"]);
                                }
                                if imp_supported.load(Ordering::SeqCst) {
                                    retry_cmd.args(["--impersonate", "chrome"]);
                                }
                                retry_cmd.args(["--extractor-retries", "5"]);
                            }
                            retry_cmd
                                .args(["-f", "best"])
                                .arg("-o")
                                .arg(video_dir.join(format!("{}.%(ext)s", file_slug)).to_string_lossy().as_ref())
                                .args(["--no-overwrites", "--no-warnings", "--force-ipv4"])
                                .args(["--retries", "3", "--fragment-retries", "3"])
                                .args(["--socket-timeout", "30", "--throttled-rate", "100K"])
                                .args(["--extractor-retries", "2"])
                                .args(["--sleep-requests", "1"])
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
                                    Ok(Ok(_retry_s)) => {
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
                                            video_ok_c.fetch_add(1, Ordering::SeqCst);
                                            app.emit("download-log", format!("  -> Video OK via fallback ({:.1} MB)", sz)).ok();
                                            app.emit("download-status", (idx, "done".to_string())).ok();
                                            if settings.include_comments {
                                                let mut comment_cmd = new_cmd(&yt_path);
                                                comment_cmd.args(yt_dlp_extra());
                                                if !settings.cookie_file.is_empty() {
                                                    comment_cmd.args(["--cookies", &settings.cookie_file]);
                                                }
                                                comment_cmd
                                                    .args(["--write-comments", "--skip-download", "--no-warnings", "--force-ipv4"])
                                                    .arg("-o")
                                                    .arg(video_dir.join(format!("{}.%(ext)s", file_slug)).to_string_lossy().as_ref())
                                                    .arg(&video_url);
                                                if let Ok(output) = comment_cmd.output().await {
                                                    if output.status.success() {
                                                        comment_ok_c.fetch_add(1, Ordering::SeqCst);
                                                        let n = generate_video_comments_html(&video_dir, &video.title, &video.id, &video.channel, &file_slug, settings.flat_output);
                                                        total_comments_c.fetch_add(n as u32, Ordering::SeqCst);
                                                    }
                                                }
                                            }
                                        } else {
                                            app.emit("download-status", (idx, "Failed".to_string())).ok();
                                        }
                                    }
                                    _ => {
                                        let is_members = retry_stderr_buf.contains("Join this channel") || retry_stderr_buf.contains("members-only") || retry_stderr_buf.contains("Sign in to confirm");
                                        let is_unavailable = retry_stderr_buf.contains("Video unavailable") || retry_stderr_buf.contains("Private video");
                                        let is_429 = retry_stderr_buf.contains("HTTP Error 429");
                                        let is_503 = retry_stderr_buf.contains("HTTP Error 503");
                                        let is_network_error = retry_stderr_buf.contains("Giving up after") || retry_stderr_buf.contains("bytes read");
                                        let err = if is_members {
                                            "Members only"
                                        } else if is_unavailable {
                                            "Unavailable"
                                        } else if is_429 {
                                            "Rate limited"
                                        } else if retry_stderr_buf.contains("Sign in") || retry_stderr_buf.contains("bot") || retry_stderr_buf.contains("cookie") || retry_stderr_buf.contains("HTTP Error 403") {
                                            "Cookie expired"
                                        } else if is_503 {
                                            "Server busy"
                                        } else if is_network_error {
                                            "Network error"
                                        } else {
                                            "Failed"
                                        };
                                        for line in retry_stderr_buf.lines().take(40) {
                                            app.emit("download-log", format!("  | {}", line)).ok();
                                        }
                                        if err == "Cookie expired" || err == "Members only" {
                                            app.emit("download-log", "  Hint: Your cookies may have expired. Please re-export from browser and paste again.".to_string()).ok();
                                        } else if err == "Rate limited" {
                                            app.emit("download-log", "  Hint: YouTube is rate-limiting requests. Try reducing parallel downloads or wait a few minutes.".to_string()).ok();
                                        }
                                        app.emit("download-status", (idx, err.to_string())).ok();
                                    }
                                }
                            }
                        } else {
                            let is_members = stderr_buf.contains("Join this channel") || stderr_buf.contains("members-only") || stderr_buf.contains("Sign in to confirm");
                            let is_unavailable = stderr_buf.contains("Video unavailable") || stderr_buf.contains("Private video");
                            let is_429 = stderr_buf.contains("HTTP Error 429");
                            let is_503 = stderr_buf.contains("HTTP Error 503");
                            let is_network_error = stderr_buf.contains("Giving up after") || stderr_buf.contains("bytes read");
                            let err = if is_members {
                                "Members only"
                            } else if is_unavailable {
                                "Unavailable"
                            } else if is_429 {
                                "Rate limited"
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
        });
    }

    // Wait for all download tasks to complete
    while tasks.join_next().await.is_some() {}

    let video_ok = video_ok.load(Ordering::SeqCst);
    let comment_ok = comment_ok.load(Ordering::SeqCst);
    let total_comments = total_comments.load(Ordering::SeqCst);

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
        PathBuf::from(&output_dir).join(sanitize_path_for_os(&playlist_title))
    };
    if !base_dir.exists() {
        return vec![false; videos.len()];
    }
    let video_exts = ["mp4","mp3","webm","mkv","avi","flac","wav","ogg","m4a"];
    videos.iter().map(|v| {
        let file_prefix = slugify(&v.title);
        let video_dir = if flat_output { base_dir.clone() } else { base_dir.join(sanitize_path_for_os(&v.title)) };
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
