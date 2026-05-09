const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;
const { listen } = window.__TAURI__.event;

// ── i18n ──────────────────────────────────────────────────────────────
const translations = {
  vi: {
    appTitle: "YouTube Playlist Downloader",
    playlistUrl: "Link Playlist", urlPlaceholder: "Dán link YouTube playlist hoặc video...",
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
    commentsOnly: "Chỉ tải bình luận (bỏ qua video)",
    startDownload: "Bắt đầu tải", stop: "Dừng",
    openFolder: "Mở thư mục",
    queueEmpty: "Dán link playlist và bấm Bắt đầu",
    selectAll: "Chọn tất cả", selected: "đã chọn",
    noCookieAlert: "Vui lòng dán cookie cho nội dung riêng tư.",
    fetching: "Đang lấy thông tin playlist...",
  },
  en: {
    appTitle: "YouTube Playlist Downloader",
    playlistUrl: "Playlist URL", urlPlaceholder: "Paste YouTube playlist or video URL...",
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
    commentsOnly: "Comments only (skip video)",
    startDownload: "Start Download", stop: "Stop",
    openFolder: "Open Folder",
    queueEmpty: "Paste a playlist URL and click Start",
    selectAll: "Select all", selected: "selected",
    noCookieAlert: "Please paste cookies for private content.",
    fetching: "Fetching playlist...",
  },
};

// Fallback: any key not found falls back to English, then to the key itself
function t(key) {
  const lang = document.getElementById('lang').value;
  return (translations[lang] && translations[lang][key])
    || translations.en[key]
    || key;
}

function applyTranslations() {
  document.querySelectorAll('[data-i18n]').forEach(el => {
    const key = el.getAttribute('data-i18n');
    el.innerHTML = t(key);
  });
  document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
    el.placeholder = t(el.getAttribute('data-i18n-placeholder'));
  });
}

// ── Elements ──────────────────────────────────────────────────────────
const $url = document.getElementById('url');
const $cookieText = document.getElementById('cookie-text');
const $output = document.getElementById('output');
const $quality = document.getElementById('quality');
const $format = document.getElementById('format');
const $proxy = document.getElementById('proxy');
const $autoTag = document.getElementById('auto-tag');
const $commentsOnly = document.getElementById('comments-only');
const $start = document.getElementById('btn-start');
const $stop = document.getElementById('btn-stop');
const $queue = document.getElementById('queue');
const $log = document.getElementById('log');
const $progressFill = document.getElementById('progress-fill');
const $stats = document.getElementById('stats');
const $folder = document.getElementById('btn-folder');
const $ytdlpStatus = document.getElementById('ytdlp-status');
const $lang = document.getElementById('lang');

let outputDir = '';
let accessType = 'public';
let playlistVideos = []; // store for checkbox tracking

// ── Language switch ───────────────────────────────────────────────────
$lang.addEventListener('change', () => applyTranslations());

// ── Tab switching ─────────────────────────────────────────────────────
document.querySelectorAll('.tab').forEach(tab => {
  tab.addEventListener('click', () => {
    document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
    tab.classList.add('active');
    accessType = tab.dataset.tab;
    document.getElementById(`content-${accessType}`).classList.add('active');
  });
});

// ── yt-dlp check ──────────────────────────────────────────────────────
async function checkYtdlp() {
  try {
    const version = await invoke('check_ytdlp');
    $ytdlpStatus.textContent = `yt-dlp v${version}`;
    $ytdlpStatus.className = 'ytdlp-status ok';
  } catch {
    $ytdlpStatus.textContent = 'Installing yt-dlp...';
    $ytdlpStatus.className = 'ytdlp-status error';
    try {
      const version = await invoke('install_ytdlp');
      $ytdlpStatus.textContent = `yt-dlp v${version}`;
      $ytdlpStatus.className = 'ytdlp-status ok';
    } catch {
      $ytdlpStatus.textContent = 'yt-dlp install failed';
    }
  }
}

// ── Cookie handling ───────────────────────────────────────────────────
async function getCookieFile() {
  if (accessType === 'public') return '';
  const text = $cookieText.value.trim();
  if (!text) { alert(t('noCookieAlert')); return null; }
  try {
    return await invoke('save_cookie_text', { text });
  } catch (e) {
    alert('Cookie error: ' + e);
    return null;
  }
}

// ── Folder picker ─────────────────────────────────────────────────────
document.getElementById('btn-output').addEventListener('click', async () => {
  const selected = await open({ directory: true });
  if (selected) { $output.value = selected; outputDir = selected; }
});

// ── Checkbox handling ─────────────────────────────────────────────────
function toggleAllVideos(checked) {
  document.querySelectorAll('.video-check').forEach(cb => { cb.checked = checked; });
  updateCardStyles();
}

function updateCardStyles() {
  document.querySelectorAll('.video-card').forEach(card => {
    const cb = card.querySelector('.video-check');
    card.classList.toggle('unchecked', !cb.checked);
  });
  const total = document.querySelectorAll('.video-check').length;
  const checked = document.querySelectorAll('.video-check:checked').length;
  const countEl = document.querySelector('.queue-header .count');
  if (countEl) countEl.textContent = `${checked}/${total} ${t('selected')}`;
}

// ── Start download ────────────────────────────────────────────────────
$start.addEventListener('click', async () => {
  const url = $url.value.trim();
  if (!url) { $url.focus(); return; }

  const cookieFile = await getCookieFile();
  if (cookieFile === null) return;

  if (!outputDir) {
    const selected = await open({ directory: true });
    if (!selected) return;
    outputDir = selected;
    $output.value = selected;
  }

  // Get selected video indices
  const selectedIndices = [];
  document.querySelectorAll('.video-check:checked').forEach(cb => {
    selectedIndices.push(parseInt(cb.dataset.index));
  });

  if (playlistVideos.length > 0 && selectedIndices.length === 0) {
    alert('Please select at least one video.');
    return;
  }

  $start.disabled = true;
  $stop.disabled = false;
  $folder.disabled = true;
  $log.innerHTML = '';
  $progressFill.style.width = '0%';
  $stats.textContent = '';

  if (playlistVideos.length === 0) {
    $queue.innerHTML = `<div class="queue-empty">${t('fetching')}</div>`;
  }

  try {
    if (playlistVideos.length === 0) {
      const result = await invoke('fetch_playlist', {
        url, cookieFile: cookieFile || '', proxy: $proxy.value || null,
      });
      playlistVideos = result.videos;
      renderQueue(result.videos);
      $log.innerHTML = '';
      appendLog(`${result.title} (${result.videos.length} videos)`);
      $start.disabled = false;
      return; // Let user select videos, then click Start again
    }

    // Download selected videos
    await invoke('start_download', {
      settings: {
        playlistUrl: url,
        cookieFile: cookieFile || '',
        outputDir,
        quality: $quality.value,
        format: $format.value,
        proxy: $proxy.value || null,
        commentsOnly: $commentsOnly.checked,
        autoTag: $autoTag.checked,
        selectedIndices,
      },
    });
  } catch (e) {
    appendLog(`Error: ${e}`);
    $start.disabled = false;
    $stop.disabled = true;
  }
});

// ── Stop ──────────────────────────────────────────────────────────────
$stop.addEventListener('click', () => {
  invoke('cancel_download');
  $stop.disabled = true;
  appendLog('Cancelling...');
});

// ── Open folder ───────────────────────────────────────────────────────
$folder.addEventListener('click', () => {
  if (outputDir) invoke('open_folder', { path: outputDir });
});

// ── Render queue as cards with checkboxes ─────────────────────────────
function renderQueue(videos) {
  playlistVideos = videos;
  const header = `
    <div class="queue-header">
      <label class="select-all">
        <input type="checkbox" id="select-all" checked />
        <span>${t('selectAll')}</span>
      </label>
      <span class="count">${videos.length}/${videos.length} ${t('selected')}</span>
    </div>`;

  const cards = videos.map((v, i) => {
    const dur = v.duration ? formatDuration(v.duration) : '';
    return `
    <div class="video-card" id="row-${i + 1}">
      <div class="check-col">
        <input type="checkbox" class="video-check" data-index="${i}" checked />
      </div>
      <img class="thumb" src="${v.thumbnail}" alt="" onerror="this.style.display='none'" />
      <div class="info">
        <div class="title">${escapeHtml(v.title)}</div>
        <div class="channel">${escapeHtml(v.channel)}</div>
        <div class="meta">${dur ? `<span class="dur">${dur}</span>` : ''}</div>
      </div>
      <span class="status pending" id="status-${i + 1}">Pending</span>
    </div>`;
  }).join('');

  $queue.innerHTML = header + cards;

  // Bind events
  document.getElementById('select-all').addEventListener('change', (e) => {
    toggleAllVideos(e.target.checked);
  });
  document.querySelectorAll('.video-check').forEach(cb => {
    cb.addEventListener('change', updateCardStyles);
  });
}

function formatDuration(sec) {
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  const h = Math.floor(m / 60);
  const rm = m % 60;
  return h > 0 ? `${h}:${String(rm).padStart(2,'0')}:${String(s).padStart(2,'0')}` : `${rm}:${String(s).padStart(2,'0')}`;
}

// ── Events from Rust ──────────────────────────────────────────────────
listen('download-log', (event) => appendLog(event.payload));

listen('download-status', (event) => {
  const [idx, status] = event.payload;
  const el = document.getElementById(`status-${idx}`);
  if (el) { el.textContent = status; el.className = `status ${status}`; }
});

listen('download-progress', (event) => {
  const [current, total] = event.payload;
  $progressFill.style.width = Math.round((current / total) * 100) + '%';
  $stats.textContent = `${current}/${total}`;
});

listen('download-done', (event) => {
  const [ok, total] = event.payload;
  $start.disabled = false;
  $stop.disabled = true;
  $folder.disabled = false;
  $stats.textContent = `Done: ${ok}/${total}`;
  $progressFill.style.width = '100%';
  playlistVideos = [];
});

function appendLog(text) {
  const line = document.createElement('div');
  line.textContent = text;
  $log.appendChild(line);
  $log.scrollTop = $log.scrollHeight;
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// ── Init ──────────────────────────────────────────────────────────────
window.addEventListener('DOMContentLoaded', () => {
  applyTranslations();
  checkYtdlp();
});
