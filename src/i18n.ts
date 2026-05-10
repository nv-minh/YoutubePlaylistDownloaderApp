import type { Lang, TranslationKey } from "./types";

const translations: Record<Lang, Record<TranslationKey, string>> = {
  vi: {
    appTitle: "OmniGrab",
    playlistUrl: "Link Playlist", urlPlaceholder: "Dán link YouTube playlist, channel, hoặc video...",
    accessType: "Loại truy cập", tabPublic: "Công khai", tabPrivate: "Riêng tư / Member",
    publicHint: "Playlist công khai không cần cookie.",
    cookieStep1: 'Cài extension <a href="https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbecc" target="_blank" class="link">Get cookies.txt LOCALLY</a> cho Chrome',
    cookieStep2: 'Mở <a href="https://www.youtube.com" target="_blank" class="link">youtube.com</a> và đăng nhập',
    cookieStep3: "Bấm icon extension, rồi bấm <strong>Export</strong>",
    cookieStep4: "Copy toàn bộ nội dung và dán vào bên dưới",
    cookiePlaceholder: "Dán nội dung cookies.txt vào đây...",
    saveTo: "Lưu tại", outputPlaceholder: "Chọn thư mục lưu...", browse: "Chọn",
    videoQuality: "Chất lượng video", outputFormat: "Định dạng đầu ra",
    proxy: "Proxy (tùy chọn)",
    autoTag: "Tự động gắn tag (artist, title, genre)",
    commentsOnly: "Tải bình luận (comments)",
    injectMetadata: "Gắn metadata (ảnh bìa, nghệ sĩ, album)",
    updateMode: "Chế độ cập nhật (bỏ qua đã tải)",
    exportComments: "Xuất bình luận", exportNone: "Không",
    statusExists: "Đã có",
    downloadSubs: "Tải phụ đề (subtitles)",
    writeInfoJson: "Lưu thông tin video (.info.json)",
    flatOutput: "Flat output (không tạo folder riêng cho mỗi video)",
    subLangs: "Ngôn ngữ phụ đề",
    subLangCustom: "Tùy chỉnh...",
    deleteVideo: "Xóa video này",
    startDownload: "Bắt đầu tải", stop: "Dừng",
    openFolder: "Mở thư mục", clear: "Xóa",
    queueEmpty: "Dán link playlist/video và bấm Bắt đầu",
    selectAll: "Chọn tất cả", selected: "đã chọn",
    noCookieAlert: "Vui lòng dán cookie cho nội dung riêng tư.",
    fetching: "Đang lấy thông tin...",
    redownload: "Tải lại lỗi",
    tabPlaylist: "Playlist", tabVideo: "1 Video",
    videoUrl: "Link Video", videoUrlPlaceholder: "Dán link YouTube video...",
    tabTiktok: "TikTok", tiktokUrl: "Link TikTok",
    tiktokUrlPlaceholder: "Dán link video hoặc @username...",
    noWatermark: "Tải không watermark",
    parallelDownloads: "Tải song song",
    fetchInfo: "Lấy thông tin",
  },
  en: {
    appTitle: "OmniGrab",
    playlistUrl: "Playlist URL", urlPlaceholder: "Paste YouTube playlist, channel, or video URL...",
    accessType: "Access Type", tabPublic: "Public", tabPrivate: "Private / Members-only",
    publicHint: "Public playlists don't require cookies.",
    cookieStep1: 'Install <a href="https://chromewebstore.google.com/detail/get-cookiestxt-locally/cclelndahbckbenkjhflpdbgdldlbecc" target="_blank" class="link">Get cookies.txt LOCALLY</a> extension for Chrome',
    cookieStep2: 'Open <a href="https://www.youtube.com" target="_blank" class="link">youtube.com</a> and log in',
    cookieStep3: "Click the extension icon, then click <strong>Export</strong>",
    cookieStep4: "Copy all content and paste below",
    cookiePlaceholder: "Paste cookies.txt content here...",
    saveTo: "Save To", outputPlaceholder: "Select output folder...", browse: "Browse",
    videoQuality: "Video Quality", outputFormat: "Output Format",
    proxy: "Proxy (optional)",
    autoTag: "Auto-tag (artist, title, genre)",
    commentsOnly: "Include comments",
    injectMetadata: "Inject metadata (thumbnail, artist, album)",
    updateMode: "Update mode (skip already downloaded)",
    exportComments: "Export comments", exportNone: "None",
    statusExists: "Exists",
    downloadSubs: "Download subtitles",
    writeInfoJson: "Save video info (.info.json)",
    flatOutput: "Flat output (no subfolder per video)",
    subLangs: "Subtitle language",
    subLangCustom: "Custom...",
    deleteVideo: "Remove this video",
    startDownload: "Start Download", stop: "Stop",
    openFolder: "Open Folder", clear: "Clear",
    queueEmpty: "Paste a playlist/video URL and click Start",
    selectAll: "Select all", selected: "selected",
    noCookieAlert: "Please paste cookies for private content.",
    fetching: "Fetching info...",
    redownload: "Redownload failed",
    tabPlaylist: "Playlist", tabVideo: "1 Video",
    videoUrl: "Video URL", videoUrlPlaceholder: "Paste YouTube video URL...",
    tabTiktok: "TikTok", tiktokUrl: "TikTok URL",
    tiktokUrlPlaceholder: "Paste video link or @username...",
    noWatermark: "Download without watermark",
    parallelDownloads: "Parallel downloads",
    fetchInfo: "Fetch Info",
  },
};

export function t(key: TranslationKey): string {
  const lang = (document.getElementById("lang") as HTMLSelectElement).value as Lang;
  return translations[lang]?.[key] ?? translations.en[key] ?? key;
}

export function applyTranslations(): void {
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.getAttribute("data-i18n") as TranslationKey;
    if (key) el.innerHTML = t(key);
  });
  document.querySelectorAll("[data-i18n-placeholder]").forEach((el) => {
    const key = el.getAttribute("data-i18n-placeholder") as TranslationKey;
    if (key && el instanceof HTMLInputElement) el.placeholder = t(key);
  });
}
