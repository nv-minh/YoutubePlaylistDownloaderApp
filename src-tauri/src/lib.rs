mod commands;
mod htmlgen;
mod types;
mod utils;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;
use types::{CancelState, YtDlpPath};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Fix libEGL warning on Linux — use software rendering for WebKitGTK
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    let yt_path = if cfg!(target_os = "windows") {
        "yt-dlp.exe".into()
    } else {
        "yt-dlp".into()
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(CancelState(Arc::new(AtomicBool::new(false))))
        .manage(YtDlpPath(Mutex::new(yt_path)))
        .invoke_handler(tauri::generate_handler![
            commands::check_ytdlp,
            commands::install_ytdlp,
            commands::fetch_playlist,
            commands::start_download,
            commands::cancel_download,
            commands::check_existing_videos,
            commands::save_cookie_text,
            commands::open_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
