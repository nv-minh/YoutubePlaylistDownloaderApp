# YouTube Playlist Downloader

A native desktop app for downloading YouTube playlist videos and comments. Built with Tauri + Rust.

## Features

- **Playlist & Video Download** - Download all videos from a playlist or select specific ones
- **Card-based UI** - Browse videos as cards with thumbnails, select/deselect individually or "Select All"
- **Public & Private** - Public playlists work without login. Private/members-only playlists support cookie import
- **9 Output Formats** - MP4, MP3, WebM, MKV, AVI, FLAC, WAV, OGG, M4A
- **Auto-Tagging** - Automatically extracts artist, title, genre from video title (for audio formats)
- **Nested Comments** - Downloads all comments with nested replies, generates visual HTML report
- **15 Languages** - Vietnamese, English, Arabic, Chinese, Dutch, French, German, Hebrew, Italian, Polish, Portuguese (BR), Romanian, Russian, Spanish, Turkish
- **Cross-Platform** - Native app for macOS (Apple Silicon + Intel) and Windows

## Download

Go to [Releases](https://github.com/nv-minh/YoutubePlaylistDownloaderApp/releases) and download:

- **macOS**: `YouTube Playlist Downloader_*_aarch64.dmg` (Apple Silicon) or `*_x86_64.dmg` (Intel)
- **Windows**: `YouTube Playlist Downloader_*_x64-setup.exe`

## How to Use

### Public Playlist
1. Paste the playlist URL
2. Select videos, quality, and format
3. Click **Start Download**

### Private / Members-only Playlist
1. Switch to **Private / Members-only** tab
2. Follow the on-screen steps to export cookies from your browser
3. Paste the cookie content into the text area
4. Click **Start Download**

### Cookie Export Steps
1. Install [Get cookies.txt LOCALLY](https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbecc) extension
2. Open [youtube.com](https://www.youtube.com) and log in
3. Click the extension icon -> **Export**
4. Copy the file content and paste it into the app

## Development

### Prerequisites
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install)
- yt-dlp (auto-installed on first run)

### Build from source
```bash
git clone https://github.com/nv-minh/YoutubePlaylistDownloaderApp.git
cd YoutubePlaylistDownloaderApp
npm install
npm run tauri dev
```

### Build release
```bash
npm run tauri build
```

## Tech Stack

- **Frontend**: HTML/CSS/JS with Apple-inspired dark design
- **Backend**: Rust (Tauri 2.0)
- **Downloader**: yt-dlp
- **Size**: ~4MB (vs ~150MB for Electron apps)

## License

MIT
