const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;
const { listen } = window.__TAURI__.event;

// ── Elements ──────────────────────────────────────────────────────────
const $url = document.getElementById('url');
const $cookie = document.getElementById('cookie');
const $output = document.getElementById('output');
const $quality = document.getElementById('quality');
const $proxy = document.getElementById('proxy');
const $commentsOnly = document.getElementById('comments-only');
const $start = document.getElementById('btn-start');
const $stop = document.getElementById('btn-stop');
const $queue = document.getElementById('queue');
const $log = document.getElementById('log');
const $progressFill = document.getElementById('progress-fill');
const $stats = document.getElementById('stats');
const $folder = document.getElementById('btn-folder');
const $ytdlpStatus = document.getElementById('ytdlp-status');

let outputDir = '';

// ── yt-dlp check ──────────────────────────────────────────────────────
async function checkYtdlp() {
  try {
    const version = await invoke('check_ytdlp');
    $ytdlpStatus.textContent = `yt-dlp v${version}`;
    $ytdlpStatus.className = 'ytdlp-status ok';
  } catch (e) {
    $ytdlpStatus.textContent = 'yt-dlp not found. Installing...';
    $ytdlpStatus.className = 'ytdlp-status error';
    try {
      const version = await invoke('install_ytdlp');
      $ytdlpStatus.textContent = `yt-dlp v${version} installed`;
      $ytdlpStatus.className = 'ytdlp-status ok';
    } catch (e2) {
      $ytdlpStatus.textContent = 'Failed to install yt-dlp';
    }
  }
}

// ── File/Folder pickers ───────────────────────────────────────────────
document.getElementById('btn-cookie').addEventListener('click', async () => {
  const selected = await open({ multiple: false, filters: [{ name: 'Text', extensions: ['txt'] }] });
  if (selected) $cookie.value = selected;
});

document.getElementById('btn-output').addEventListener('click', async () => {
  const selected = await open({ directory: true });
  if (selected) {
    $output.value = selected;
    outputDir = selected;
  }
});

// ── Start download ────────────────────────────────────────────────────
$start.addEventListener('click', async () => {
  const url = $url.value.trim();
  if (!url) { $url.focus(); return; }
  if (!$cookie.value) { alert('Please select a cookie file'); return; }

  if (!outputDir) {
    const selected = await open({ directory: true });
    if (!selected) return;
    outputDir = selected;
    $output.value = selected;
  }

  $start.disabled = true;
  $stop.disabled = false;
  $folder.disabled = true;
  $log.innerHTML = '';
  $progressFill.style.width = '0%';
  $stats.textContent = '';
  $queue.innerHTML = '<div class="queue-empty">Fetching playlist...</div>';

  try {
    const result = await invoke('fetch_playlist', {
      url,
      cookieFile: $cookie.value,
      proxy: $proxy.value || null,
    });

    renderQueue(result.videos);
    $log.innerHTML = '';
    appendLog(`Playlist: ${result.title} (${result.videos.length} videos)`);

    await invoke('start_download', {
      settings: {
        playlistUrl: url,
        cookieFile: $cookie.value,
        outputDir: outputDir,
        quality: $quality.value,
        proxy: $proxy.value || null,
        commentsOnly: $commentsOnly.checked,
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

// ── Events from Rust ──────────────────────────────────────────────────
function renderQueue(videos) {
  $queue.innerHTML = videos.map((v, i) => `
    <div class="video-row" id="row-${i + 1}">
      <span class="idx">${i + 1}</span>
      <img class="thumb" src="${v.thumbnail}" alt="" onerror="this.style.display='none'" />
      <div class="info">
        <div class="title">${escapeHtml(v.title)}</div>
        <div class="channel">${escapeHtml(v.channel)}</div>
      </div>
      <span class="status pending" id="status-${i + 1}">Pending</span>
    </div>
  `).join('');
}

listen('download-log', (event) => {
  appendLog(event.payload);
});

listen('download-status', (event) => {
  const [idx, status] = event.payload;
  const el = document.getElementById(`status-${idx}`);
  if (el) {
    el.textContent = status;
    el.className = `status ${status}`;
  }
});

listen('download-progress', (event) => {
  const [current, total] = event.payload;
  const pct = Math.round((current / total) * 100);
  $progressFill.style.width = pct + '%';
  $stats.textContent = `${current}/${total}`;
});

listen('download-done', (event) => {
  const [ok, total] = event.payload;
  $start.disabled = false;
  $stop.disabled = true;
  $folder.disabled = false;
  $stats.textContent = `Done: ${ok}/${total}`;
  $progressFill.style.width = '100%';
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
  checkYtdlp();
});
