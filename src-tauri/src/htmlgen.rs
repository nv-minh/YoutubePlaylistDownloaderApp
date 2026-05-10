use crate::types::{CommentEntry, CommentExport, VideoInfo};
use crate::utils::{sanitize_folder_name, slugify};
use std::fs;
use std::path::{Path, PathBuf};

// ── HTML Helpers ───────────────────────────────────────────────────────

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// ── Comment Loading ────────────────────────────────────────────────────

pub fn load_comments_for_video(video_dir: &PathBuf) -> Vec<serde_json::Value> {
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

// ── Comment HTML Generation ────────────────────────────────────────────

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

// ── Video Comments Page ────────────────────────────────────────────────

pub fn generate_video_comments_html(video_dir: &PathBuf, video_title: &str, video_id: &str, channel: &str, file_slug: &str, flat_output: bool) -> usize {
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

    let html_filename = if flat_output { format!("{}-comments.html", file_slug) } else { "comments.html".to_string() };
    let html_path = video_dir.join(&html_filename);
    let _ = fs::write(&html_path, page);
    count
}

// ── Playlist Index Page ────────────────────────────────────────────────

pub fn generate_index_html(playlist_title: &str, videos: &[VideoInfo], base_dir: &PathBuf, video_ok: u32, total_comments: u32, flat_output: bool) {
    let mut rows = String::new();
    for video in videos {
        let prefix = slugify(&video.title);
        let folder_name = sanitize_folder_name(&video.title);
        let video_dir = if flat_output { base_dir.clone() } else { base_dir.join(&folder_name) };

        let has_video = {
            let video_exts = ["mp4","mp3","webm","mkv","avi","flac","wav","ogg","m4a"];
            if let Ok(entries) = fs::read_dir(&video_dir) {
                entries.flatten().any(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.starts_with(&format!("{}.", prefix)) || name.starts_with("video.") {
                        let lower = name.to_lowercase();
                        video_exts.iter().any(|ext| lower.ends_with(&format!(".{}", ext)))
                    } else { false }
                })
            } else { false }
        };
        let has_comments = video_dir.join(format!("{}-comments.html", prefix)).exists() || video_dir.join("comments.html").exists();

        let dur = video.duration.map(|s| {
            let h = s / 3600;
            let m = (s % 3600) / 60;
            let sec = s % 60;
            if h > 0 { format!("{}:{:02}:{:02}", h, m, sec) } else { format!("{}:{:02}", m, sec) }
        }).unwrap_or_default();

        let comments_link = if has_comments {
            let href = if flat_output {
                format!("{}-comments.html", prefix)
            } else {
                format!("{}/comments.html", folder_name)
            };
            format!("<a class=\"btn-comments\" href=\"{}\">View comments</a>", href)
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

// ── Comment Export ─────────────────────────────────────────────────────

pub fn export_comments_to_file(base_dir: &Path, format: &str) -> Result<String, String> {
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
