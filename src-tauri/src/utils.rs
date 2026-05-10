use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tokio::process::Command;

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
pub static RE_PROGRESS: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\[download\]\s+([\d.]+)%").unwrap()
});

// ── Environment Setup ──────────────────────────────────────────────────

pub fn setup_env(cmd: &mut Command) {
    let sep = if cfg!(target_os = "windows") { ";" } else { ":" };
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();

    let mut paths: Vec<String> = vec![];

    if cfg!(target_os = "windows") {
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
        let app_data = std::env::var("APPDATA").unwrap_or_default();
        if !app_data.is_empty() {
            paths.push(format!("{}\\yt-playlist-downloader", app_data));
        }
    } else {
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

pub fn new_cmd(program: &str) -> Command {
    let mut cmd = Command::new(program);
    setup_env(&mut cmd);
    cmd
}

pub fn yt_dlp_extra() -> Vec<String> {
    vec![
        "--js-runtimes".into(),
        "node".into(),
        "--remote-components".into(),
        "ejs:github".into(),
    ]
}

// ── String Helpers ─────────────────────────────────────────────────────

pub fn quality_format(quality: &str, format: &str) -> String {
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

pub fn parse_title_metadata(title: &str) -> (String, String, String) {
    if let Some(pos) = title.find(" - ") {
        let artist = title[..pos].trim().to_string();
        let rest = title[pos + 3..].trim();
        let clean_title = RE_TITLE_METADATA.replace_all(rest, "").trim().to_string();
        return (artist, clean_title, String::new());
    }
    (String::new(), title.to_string(), String::new())
}

pub fn sanitize_folder_name(name: &str) -> String {
    let cleaned = RE_SANITIZE_FOLDER.replace_all(name, "").to_string();
    let trimmed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.len() > 200 { trimmed[..200].to_string() } else { trimmed }
}

pub fn slugify(name: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let slug: String = name.nfd()
        .filter(|c| c.is_ascii() && (c.is_alphanumeric() || *c == ' '))
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");
    if slug.len() > 150 { slug[..150].trim_end_matches('-').to_string() } else { slug }
}

pub fn extract_playlist_id(url: &str) -> Option<String> {
    if let Some(caps) = RE_PLAYLIST_ID.captures(url) {
        return Some(caps[1].to_string());
    }
    if RE_PLAIN_ID.is_match(url.trim()) {
        return Some(url.trim().to_string());
    }
    None
}

// ── Metadata Injection ─────────────────────────────────────────────────

pub fn inject_metadata(
    video_dir: &Path,
    thumbnail_url: &str,
    video_title: &str,
    playlist_title: &str,
    format: &str,
) -> Result<(), String> {
    let thumb_path = video_dir.join("thumb.jpg");
    let ext = format;

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
