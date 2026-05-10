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
  },
};

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
const $urlVideo = document.getElementById('url-video');
const $cookieText = document.getElementById('cookie-text');
const $output = document.getElementById('output');
const $quality = document.getElementById('quality');
const $format = document.getElementById('format');
const $proxy = document.getElementById('proxy');
const $autoTag = document.getElementById('auto-tag');
const $commentsOnly = document.getElementById('comments-only');
const $injectMetadata = document.getElementById('inject-metadata');
const $updateMode = document.getElementById('update-mode');
const $exportComments = document.getElementById('export-comments');
const $downloadSubs = document.getElementById('download-subs');
const $writeInfoJson = document.getElementById('write-info-json');
const $flatOutput = document.getElementById('flat-output');
const $subLangSelect = document.getElementById('sub-lang-select');
const $subLangs = document.getElementById('sub-langs');
const $subCustom = document.getElementById('sub-custom');
const $subOptions = document.getElementById('sub-options');
const $start = document.getElementById('btn-start');
const $stop = document.getElementById('btn-stop');
const $queue = document.getElementById('queue');
const $log = document.getElementById('log');
const $progressFill = document.getElementById('progress-fill');
const $stats = document.getElementById('stats');
const $folder = document.getElementById('btn-folder');
const $ytdlpStatus = document.getElementById('ytdlp-status');
const $lang = document.getElementById('lang');
let isDownloading = false;

let outputDir = '';
let actualDir = ''; // tracks the playlist subfolder
let accessType = 'public';
let downloadMode = 'playlist'; // 'playlist' or 'video'
let playlistVideos = [];
let failedIndices = [];

const deleteIcon = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>`;

// ── Language switch ───────────────────────────────────────────────────
$lang.addEventListener('change', () => applyTranslations());

// ── Subtitle toggle ───────────────────────────────────────────────────
$downloadSubs.addEventListener('change', () => {
  $subOptions.style.display = $downloadSubs.checked ? 'block' : 'none';
});

$subLangSelect.addEventListener('change', () => {
  $subCustom.style.display = $subLangSelect.value === 'custom' ? 'block' : 'none';
});

function getSubLangs() {
  if (!$downloadSubs.checked) return null;
  if ($subLangSelect.value === 'custom') {
    return $subLangs.value.trim() || null;
  }
  return $subLangSelect.value;
}

// ── Access tab switching ──────────────────────────────────────────────
document.querySelectorAll('.tab').forEach(tab => {
  tab.addEventListener('click', () => {
    const group = tab.closest('.tabs');
    group.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
    tab.classList.add('active');
    // Access type tabs
    if (tab.dataset.tab) {
      accessType = tab.dataset.tab;
      document.querySelectorAll('.tab-content').forEach(c => {
        if (c.id.startsWith('content-')) c.classList.remove('active');
      });
      document.getElementById(`content-${accessType}`).classList.add('active');
    }
    // Download mode tabs
    if (tab.dataset.mode) {
      downloadMode = tab.dataset.mode;
      document.querySelectorAll('.mode-content').forEach(c => c.classList.remove('active'));
      document.getElementById(`mode-${downloadMode}`).classList.add('active');
    }
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
  if (isDownloading) return;
  document.querySelectorAll('.video-check').forEach(cb => { cb.checked = checked; });
  updateCardStyles();
}

function setCheckboxState(disabled) {
  document.querySelectorAll('.video-check').forEach(cb => { cb.disabled = disabled; });
  const selectAll = document.getElementById('select-all');
  if (selectAll) selectAll.disabled = disabled;
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

// ── Clear ─────────────────────────────────────────────────────────────
document.getElementById('btn-clear').addEventListener('click', () => {
  playlistVideos = [];
  failedIndices = [];
  actualDir = '';
  $url.value = '';
  $urlVideo.value = '';
  $log.innerHTML = '';
  $progressFill.style.width = '0%';
  $stats.textContent = '';
  $start.disabled = false;
  $stop.disabled = true;
  $folder.disabled = true;
  const $redownload = document.getElementById('btn-redownload');
  if ($redownload) $redownload.style.display = 'none';
  $queue.innerHTML = `<div class="queue-empty" data-i18n="queueEmpty">${t('queueEmpty')}</div>`;
});

// ── Start download ────────────────────────────────────────────────────
$start.addEventListener('click', async () => {
  if ($start.disabled) return;
  $start.disabled = true;
  const url = (downloadMode === 'video' ? $urlVideo.value.trim() : $url.value.trim());
  if (!url) { $start.disabled = false; (downloadMode === 'video' ? $urlVideo : $url).focus(); return; }

  const cookieFile = await getCookieFile();
  if (cookieFile === null) { $start.disabled = false; return; }

  if (!outputDir) {
    const selected = await open({ directory: true });
    if (!selected) { $start.disabled = false; return; }
    outputDir = selected;
    $output.value = selected;
  }

  // Single video mode - download directly
  if (downloadMode === 'video') {
    isDownloading = true;
    setCheckboxState(true);
    $start.disabled = true;
    $stop.disabled = false;
    $folder.disabled = true;
    $log.innerHTML = '';
    $progressFill.style.width = '0%';
    $stats.textContent = '';
    $queue.innerHTML = `<div class="queue-empty">${t('fetching')}</div>`;

    try {
      await invoke('start_download', {
        settings: {
          playlist_url: url,
          cookie_file: cookieFile || '',
          output_dir: outputDir,
          quality: $quality.value,
          format: $format.value,
          proxy: $proxy.value || null,
          include_comments: $commentsOnly.checked,
          auto_tag: $autoTag.checked,
          selected_indices: [],
          single_video: true,
          inject_metadata: $injectMetadata.checked,
          update_mode: $updateMode.checked,
          export_comments: $exportComments.value || null,
          download_subs: $downloadSubs.checked,
          sub_langs: getSubLangs(),
          auto_subs: true,
          write_info_json: $writeInfoJson.checked,
          flat_output: $flatOutput.checked,
        },
      });
    } catch (e) {
      appendLog(`Error: ${e}`);
      isDownloading = false;
      setCheckboxState(false);
      $start.disabled = false;
      $stop.disabled = true;
    }
    return;
  }

  // Playlist mode
  const selectedIndices = [];
  document.querySelectorAll('.video-check:checked').forEach(cb => {
    const idx = parseInt(cb.dataset.index);
    if (playlistVideos[idx] !== null) selectedIndices.push(idx);
  });

  if (playlistVideos.length > 0 && selectedIndices.length === 0) {
    alert('Please select at least one video.');
    $start.disabled = false;
    return;
  }

  isDownloading = true;
  setCheckboxState(true);
  $stop.disabled = false;
  $folder.disabled = true;
  $log.innerHTML = '';
  $progressFill.style.width = '0%';
  $stats.textContent = '';
  failedIndices = [];

  const $redownload = document.getElementById('btn-redownload');
  if ($redownload) $redownload.style.display = 'none';

  if (playlistVideos.length === 0) {
    $queue.innerHTML = `<div class="queue-empty">${t('fetching')}</div>`;
  }

  try {
    if (playlistVideos.length === 0) {
      const result = await invoke('fetch_playlist', {
        url, cookieFile: cookieFile || '', proxy: $proxy.value || null,
      });
      playlistVideos = result.videos;

      // Check existing videos if update mode is on
      let existing = null;
      if ($updateMode.checked && outputDir) {
        try {
          existing = await invoke('check_existing_videos', {
            outputDir, playlistTitle: result.title, videos: result.videos, flatOutput: $flatOutput.checked,
          });
        } catch {}
      }

      renderQueue(result.videos, existing);
      $log.innerHTML = '';
      const newCount = existing ? existing.filter(e => !e).length : result.videos.length;
      appendLog(`${result.title} (${result.videos.length} videos${existing ? `, ${newCount} new` : ''})`);
      $start.disabled = false;
      return;
    }

    await invoke('start_download', {
      settings: {
        playlist_url: url,
        cookie_file: cookieFile || '',
        output_dir: outputDir,
        quality: $quality.value,
        format: $format.value,
        proxy: $proxy.value || null,
        include_comments: $commentsOnly.checked,
        auto_tag: $autoTag.checked,
        selected_indices: selectedIndices,
        single_video: false,
        inject_metadata: $injectMetadata.checked,
        update_mode: $updateMode.checked,
        export_comments: $exportComments.value || null,
        download_subs: $downloadSubs.checked,
        sub_langs: getSubLangs(),
        auto_subs: true,
        write_info_json: $writeInfoJson.checked,
        flat_output: $flatOutput.checked,
      },
    });
  } catch (e) {
    appendLog(`Error: ${e}`);
    isDownloading = false;
    setCheckboxState(false);
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
  const dir = actualDir || outputDir;
  if (dir) invoke('open_folder', { path: dir });
});

// ── Redownload failed ─────────────────────────────────────────────────
document.getElementById('btn-redownload').addEventListener('click', async () => {
  if (failedIndices.length === 0) return;
  const cookieFile = await getCookieFile();
  if (cookieFile === null) return;

  const $redownload = document.getElementById('btn-redownload');
  $redownload.style.display = 'none';

  isDownloading = true;
  setCheckboxState(true);
  $start.disabled = true;
  $stop.disabled = false;
  $folder.disabled = true;
  $log.innerHTML = '';
  $progressFill.style.width = '0%';

  failedIndices.forEach(i => {
    const el = document.getElementById(`status-${i + 1}`);
    if (el) { el.textContent = 'Pending'; el.className = 'status pending'; }
  });

  const indices = [...failedIndices];
  failedIndices = [];

  try {
    await invoke('start_download', {
      settings: {
        playlist_url: $url.value.trim(),
        cookie_file: cookieFile || '',
        output_dir: outputDir,
        quality: $quality.value,
        format: $format.value,
        proxy: $proxy.value || null,
        include_comments: $commentsOnly.checked,
        auto_tag: $autoTag.checked,
        selected_indices: indices,
        single_video: false,
        inject_metadata: $injectMetadata.checked,
        update_mode: $updateMode.checked,
        export_comments: $exportComments.value || null,
        download_subs: $downloadSubs.checked,
        sub_langs: getSubLangs(),
        auto_subs: true,
        write_info_json: $writeInfoJson.checked,
        flat_output: $flatOutput.checked,
      },
    });
  } catch (e) {
    appendLog(`Error: ${e}`);
    isDownloading = false;
    setCheckboxState(false);
    $start.disabled = false;
    $stop.disabled = true;
  }
});

// ── Render queue as cards with checkboxes ─────────────────────────────
function renderQueue(videos, existing = null) {
  playlistVideos = videos;
  const checkedCount = existing ? existing.filter(e => !e).length : videos.length;
  const allChecked = !existing || checkedCount === videos.length;
  const header = `
    <div class="queue-header">
      <label class="select-all">
        <input type="checkbox" id="select-all" ${allChecked ? 'checked' : ''} />
        <span>${t('selectAll')}</span>
      </label>
      <span class="count">${checkedCount}/${videos.length} ${t('selected')}</span>
    </div>`;

  const cards = videos.map((v, i) => {
    const dur = v.duration ? formatDuration(v.duration) : '';
    const isExisting = existing && existing[i];
    return `
    <div class="video-card${isExisting ? ' unchecked' : ''}" id="row-${i + 1}">
      <div class="check-col">
        <input type="checkbox" class="video-check" data-index="${i}" ${isExisting ? '' : 'checked'} />
      </div>
      <img class="thumb" src="${v.thumbnail}" alt="" onerror="this.style.display='none'" />
      <div class="info">
        <div class="title">${escapeHtml(v.title)}</div>
        <div class="channel">${escapeHtml(v.channel)}</div>
        <div class="meta">${dur ? `<span class="dur">${dur}</span>` : ''}</div>
      </div>
      <span class="status ${isExisting ? 'done' : 'pending'}" id="status-${i + 1}">${isExisting ? (t('statusExists') || 'Exists') : 'Pending'}</span>
      <button class="btn-delete-card" data-index="${i}" title="${t('deleteVideo')}">${deleteIcon}</button>
    </div>`;
  }).join('');

  $queue.innerHTML = header + cards;

  document.getElementById('select-all').addEventListener('change', (e) => {
    toggleAllVideos(e.target.checked);
  });
  document.querySelectorAll('.video-check').forEach(cb => {
    cb.addEventListener('change', updateCardStyles);
  });
  // Click on card row to toggle checkbox
  document.querySelectorAll('.video-card').forEach(card => {
    card.addEventListener('click', (e) => {
      if (isDownloading) return;
      if (e.target.tagName === 'INPUT' || e.target.tagName === 'A' || e.target.tagName === 'BUTTON' || e.target.closest('.btn-delete-card')) return;
      const cb = card.querySelector('.video-check');
      cb.checked = !cb.checked;
      updateCardStyles();
    });
  });
  // Delete button handler
  document.querySelectorAll('.btn-delete-card').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const idx = parseInt(btn.dataset.index);
      const card = document.getElementById(`row-${idx + 1}`);
      if (card) {
        card.style.transition = 'opacity 0.2s, transform 0.2s';
        card.style.opacity = '0';
        card.style.transform = 'translateX(20px)';
        setTimeout(() => {
          card.remove();
          playlistVideos[idx] = null;
          failedIndices = failedIndices.filter(fi => fi !== idx);
          updateCardStyles();
        }, 200);
      }
    });
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
  if (!el) return;
  if (status.startsWith('downloading')) {
    el.textContent = status;
    el.className = 'status downloading';
  } else if (status === 'Exists') {
    el.textContent = t('statusExists') || 'Exists';
    el.className = 'status done';
  } else {
    el.textContent = status;
    el.className = `status ${status}`;
  }
  const videoIdx = idx - 1;
  if (['Failed', 'Members only', 'Unavailable', 'Cookie expired'].includes(status)) {
    if (!failedIndices.includes(videoIdx)) failedIndices.push(videoIdx);
  } else if (status === 'done' || status === 'Exists') {
    failedIndices = failedIndices.filter(i => i !== videoIdx);
  }
});

listen('download-progress', (event) => {
  const [current, total] = event.payload;
  $progressFill.style.width = Math.round((current / total) * 100) + '%';
  $stats.textContent = `${current}/${total}`;
});

listen('download-done', (event) => {
  const [ok, total, folderPath] = event.payload;
  isDownloading = false;
  setCheckboxState(false);
  $start.disabled = false;
  $stop.disabled = true;
  $folder.disabled = false;
  $stats.textContent = `Done: ${ok}/${total}`;
  $progressFill.style.width = '100%';
  actualDir = folderPath || outputDir;

  const $redownload = document.getElementById('btn-redownload');
  if ($redownload && failedIndices.length > 0) {
    $redownload.style.display = 'inline-block';
    $redownload.textContent = `${t('redownload')} (${failedIndices.length})`;
  }
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
