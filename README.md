<div align="center">

# YouTube Playlist Downloader

### Tải xuống toàn bộ playlist YouTube chỉ với 1 cú click

<img src="screenshot.png" width="720" alt="Screenshot" />

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Windows-lightgrey.svg)]()
[![Size](https://img.shields.io/badge/Size-%E2%89%884MB-brightgreen.svg)]()
[![Tauri](https://img.shields.io/badge/Tauri-2.0-orange.svg)]()
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-dea584.svg)]()

**[Tiếng Việt](#-tính-năng) &nbsp;·&nbsp; [English](#-features)**

[⬇️ Tải xuống](#-cài-đặt) &nbsp;·&nbsp; [Cách sử dụng](#-cách-sử-dụng) &nbsp;·&nbsp; [Build từ source](#-build-từ-source)

</div>

---

## 🇻🇳 Tiếng Việt

### ✨ Tính năng

<table>
<tr><td>

#### 🎬 Tải xuống video
- **Playlist & Video đơn** — Tải cả playlist hoặc chỉ 1 video
- **Giao diện thẻ (Card UI)** — Xem trước thumbnail, chọn/bỏ chọn từng video
- **Chọn tất cả / Bỏ chọn tất cả** — Chọn nhanh video cần tải
- **Xóa video khỏi hàng đợi** — Loại bỏ video không muốn tải
- **9 định dạng** — MP4, MP3, WebM, MKV, AVI, FLAC, WAV, OGG, M4A
- **4 mức chất lượng** — Best, 1080p, 720p, 480p
- **Chế độ cập nhật** — Tự động bỏ qua video đã tải, chỉ tải video mới
- **Theo dõi tiến trình** — Thanh tiến trình + phần trăm theo thời gian thực

</td></tr>
<tr><td>

#### 🔐 Hỗ trợ nội dung riêng tư
- **Playlist công khai** — Không cần đăng nhập
- **Playlist riêng tư / Members-only** — Hỗ trợ nhập cookie từ trình duyệt
- **Hướng dẫn 4 bước** — Export cookie trực tiếp trong app

</td></tr>
<tr><td>

#### 💬 Bình luận (Comments)
- **Tải bình luận** — Lấy tất cả bình luận kèm phản hồi lồng nhau
- **Báo cáo HTML** — Tự động tạo trang HTML đẹp để xem bình luận
- **Xuất JSON / CSV** — Xuất dữ liệu bình luận để phân tích

</td></tr>
<tr><td>

#### 📝 Phụ đề (Subtitles)
- **Tải phụ đề** — Tự động tải phụ đề kèm video
- **Chọn ngôn ngữ** — 15+ ngôn ngữ phổ biến + tùy chỉnh
- **Bao gồm phụ đề tự động** — Tự động tạo phụ đề nếu không có bản thủ công
- **Nhúng vào video** — Phụ đề được nhúng trực tiếp vào MP4/WebM/MKV

</td></tr>
<tr><td>

#### 🎵 Metadata & Tagging
- **Tự động gắn tag** — Trích xuất artist, title, genre từ tên video
- **Gắn metadata** — Nhúng ảnh bìa, nghệ sĩ, album vào file (qua ffmpeg)
- **Hỗ trợ proxy** — SOCKS5 / HTTP proxy để vượt giới hạn địa lý

</td></tr>
<tr><td>

#### 🌍 Đa ngôn ngữ
- **15 ngôn ngữ UI** — Tiếng Việt, English, العربية, 中文, Nederlands, Français, Deutsch, עברית, Italiano, Polski, Português (BR), Română, Русский, Español, Türkçe
- Chuyển ngôn ngữ tức thì

</td></tr>
<tr><td>

#### ⚡ Hiệu năng
- **Nhẹ (~4MB)** — So với ~150MB của ứng dụng Electron
- **Tauri 2.0 + Rust** — Backend nhanh, an toàn, tiết kiệm RAM
- **Tải xuống song song** — 4 fragment đồng thời, 20 lần thử lại
- **Cross-platform** — macOS (Apple Silicon + Intel) & Windows

</td></tr>
</table>

---

### 📥 Cài đặt

Vào [Releases](https://github.com/nv-minh/YoutubePlaylistDownloaderApp/releases) và tải:

| Nền tảng | File |
|----------|------|
| **macOS Apple Silicon** (M1/M2/M3/M4) | `YouTube Playlist Downloader_*_aarch64.dmg` |
| **macOS Intel** | `YouTube Playlist Downloader_*_x86_64.dmg` |
| **Windows** | `YouTube Playlist Downloader_*_x64-setup.exe` |

> **Lưu ý macOS**: Nếu gặp lỗi "Cannot be opened because it is from an unidentified developer", chuột phải → Open → Open.

---

### 🎯 Cách sử dụng

#### Playlist công khai
1. Dán link playlist YouTube
2. Chọn video, chất lượng, định dạng
3. Bấm **Bắt đầu tải**

#### Playlist riêng tư / Members-only
1. Chuyển sang tab **Riêng tư / Member**
2. Làm theo 4 bước trên màn hình để export cookie
3. Dán nội dung cookie vào ô文本
4. Bấm **Bắt đầu tải**

#### Xuất cookie từ trình duyệt
1. Cài extension [Get cookies.txt LOCALLY](https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbecc)
2. Mở [youtube.com](https://www.youtube.com) và đăng nhập
3. Bấm icon extension → **Export**
4. Copy toàn bộ nội dung và dán vào app

---

### 🔧 Xử lý lỗi

| Vấn đề | Giải pháp |
|--------|-----------|
| **"Cookie expired"** | Cookie đã hết hạn. Export lại từ trình duyệt |
| **"Members only"** | Cần đăng ký thành viên kênh + dùng cookie mới |
| **Folder trống** | Thường do cookie hết hạn — export lại |
| **yt-dlp không tìm thấy** | App tự cài yt-dlp khi mở lần đầu |

---

### 🏗️ Build từ source

**Yêu cầu:**
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install) 1.80+
- yt-dlp (tự cài khi chạy lần đầu)

```bash
git clone https://github.com/nv-minh/YoutubePlaylistDownloaderApp.git
cd YoutubePlaylistDownloaderApp
npm install
npm run tauri dev
```

**Build release:**
```bash
npm run tauri build
```

---

### 🛠️ Tech Stack

| Thành phần | Công nghệ |
|-----------|-----------|
| Frontend | HTML / CSS / JS — Giao diện tối phong cách Apple |
| Backend | Rust (Tauri 2.0) |
| Downloader | yt-dlp |
| Kích thước | ~4MB |

---
---

## 🇬🇧 English

### ✨ Features

<table>
<tr><td>

#### 🎬 Video Download
- **Playlist & Single Video** — Download full playlists or individual videos
- **Card-based UI** — Browse thumbnails, select/deselect videos individually
- **Select All / Deselect All** — Quick toggle for video selection
- **Delete from queue** — Remove unwanted videos before downloading
- **9 Output Formats** — MP4, MP3, WebM, MKV, AVI, FLAC, WAV, OGG, M4A
- **4 Quality Levels** — Best, 1080p, 720p, 480p
- **Update Mode** — Skip already downloaded videos, only grab new ones
- **Real-time Progress** — Progress bar + live percentage tracking

</td></tr>
<tr><td>

#### 🔐 Private Content Support
- **Public playlists** — No login required
- **Private / Members-only** — Import cookies from your browser
- **4-step guide** — Export cookies directly within the app

</td></tr>
<tr><td>

#### 💬 Comments
- **Download comments** — Fetch all comments with nested replies
- **HTML reports** — Auto-generate beautiful HTML pages for viewing
- **Export JSON / CSV** — Export comment data for analysis

</td></tr>
<tr><td>

#### 📝 Subtitles
- **Download subtitles** — Automatically fetch subtitles with videos
- **Language selection** — 15+ popular languages + custom option
- **Auto-generated subs** — Include auto-generated subtitles when manual subs unavailable
- **Embed into video** — Subtitles embedded directly into MP4/WebM/MKV

</td></tr>
<tr><td>

#### 🎵 Metadata & Tagging
- **Auto-tagging** — Extract artist, title, genre from video title
- **Metadata injection** — Embed thumbnail, artist, album into files (via ffmpeg)
- **Proxy support** — SOCKS5 / HTTP proxy to bypass geo-restrictions

</td></tr>
<tr><td>

#### 🌍 Multilingual
- **15 UI languages** — Vietnamese, English, العربية, 中文, Nederlands, Français, Deutsch, עברית, Italiano, Polski, Português (BR), Română, Русский, Español, Türkçe
- Instant language switching

</td></tr>
<tr><td>

#### ⚡ Performance
- **Lightweight (~4MB)** — Compared to ~150MB for Electron apps
- **Tauri 2.0 + Rust** — Fast, safe, low memory backend
- **Concurrent downloads** — 4 simultaneous fragments, 20 retries
- **Cross-platform** — macOS (Apple Silicon + Intel) & Windows

</td></tr>
</table>

---

### 📥 Installation

Go to [Releases](https://github.com/nv-minh/YoutubePlaylistDownloaderApp/releases) and download:

| Platform | File |
|----------|------|
| **macOS Apple Silicon** (M1/M2/M3/M4) | `YouTube Playlist Downloader_*_aarch64.dmg` |
| **macOS Intel** | `YouTube Playlist Downloader_*_x86_64.dmg` |
| **Windows** | `YouTube Playlist Downloader_*_x64-setup.exe` |

> **macOS note**: If you see "Cannot be opened because it is from an unidentified developer", right-click → Open → Open.

---

### 🎯 How to Use

#### Public Playlist
1. Paste the YouTube playlist URL
2. Select videos, quality, and format
3. Click **Start Download**

#### Private / Members-only Playlist
1. Switch to **Private / Members-only** tab
2. Follow the 4-step on-screen guide to export cookies
3. Paste the cookie content into the text area
4. Click **Start Download**

#### Export Cookies from Browser
1. Install [Get cookies.txt LOCALLY](https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbecc) extension
2. Open [youtube.com](https://www.youtube.com) and log in
3. Click the extension icon → **Export**
4. Copy all content and paste into the app

---

### 🔧 Troubleshooting

| Issue | Solution |
|-------|----------|
| **"Cookie expired"** | Cookies expired. Re-export from browser |
| **"Members only"** | Must be a channel member + use fresh cookies |
| **Empty folders** | Usually expired cookies — re-export and try again |
| **yt-dlp not found** | App auto-installs yt-dlp on first launch |

---

### 🏗️ Build from Source

**Prerequisites:**
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install) 1.80+
- yt-dlp (auto-installed on first run)

```bash
git clone https://github.com/nv-minh/YoutubePlaylistDownloaderApp.git
cd YoutubePlaylistDownloaderApp
npm install
npm run tauri dev
```

**Build release:**
```bash
npm run tauri build
```

---

### 🛠️ Tech Stack

| Component | Technology |
|-----------|-----------|
| Frontend | HTML / CSS / JS — Apple-inspired dark theme |
| Backend | Rust (Tauri 2.0) |
| Downloader | yt-dlp |
| Size | ~4MB |

---

<div align="center">

## 📄 License

This project is licensed under the [MIT License](LICENSE).

Nếu app này hữu ích, hãy cho repo một ⭐ nhé!

If you find this app useful, please give the repo a ⭐!

</div>
