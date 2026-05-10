import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import type { PlaylistResult, DownloadSettings } from "./types";
import {
  $url, $urlVideo, $urlTiktok, $output, $quality, $format, $proxy,
  $autoTag, $commentsOnly, $injectMetadata, $updateMode,
  $exportComments, $downloadSubs, $writeInfoJson, $flatOutput,
  $subLangSelect, $subLangs, $start, $stop, $folder,
  $log, $progressFill, $stats, $queue,
  $noWatermark, $maxConcurrent,
  isDownloading, outputDir, actualDir, accessType, downloadMode,
  playlistVideos, failedIndices,
  setIsDownloading, setOutputDir, setActualDir, setFailedIndices,
} from "./dom";
import { t } from "./i18n";
import { setCheckboxState, appendLog, renderQueue } from "./ui";

// ── yt-dlp check ───────────────────────────────────────────────────────

export async function checkYtdlp(): Promise<void> {
  const $ytdlpStatus = document.getElementById("ytdlp-status")!;
  try {
    const version = await invoke<string>("check_ytdlp");
    $ytdlpStatus.textContent = `yt-dlp v${version}`;
    $ytdlpStatus.className = "ytdlp-status ok";
  } catch {
    $ytdlpStatus.textContent = "Installing yt-dlp...";
    $ytdlpStatus.className = "ytdlp-status error";
    try {
      const version = await invoke<string>("install_ytdlp");
      $ytdlpStatus.textContent = `yt-dlp v${version}`;
      $ytdlpStatus.className = "ytdlp-status ok";
    } catch {
      $ytdlpStatus.textContent = "yt-dlp install failed";
    }
  }
}

// ── Cookie handling ────────────────────────────────────────────────────

async function getCookieFile(): Promise<string | null> {
  const $cookieText = document.getElementById("cookie-text") as HTMLTextAreaElement;
  if (accessType === "public") return "";
  const text = $cookieText.value.trim();
  if (!text) { alert(t("noCookieAlert")); return null; }
  try {
    return await invoke<string>("save_cookie_text", { text });
  } catch (e) {
    alert("Cookie error: " + e);
    return null;
  }
}

// ── Subtitle helper ────────────────────────────────────────────────────

function getSubLangs(): string | null {
  if (!$downloadSubs.checked) return null;
  if ($subLangSelect.value === "custom") {
    return $subLangs.value.trim() || null;
  }
  return $subLangSelect.value;
}

// ── Build settings object ──────────────────────────────────────────────

function buildSettings(overrides: Partial<DownloadSettings>): DownloadSettings {
  return {
    playlist_url: overrides.playlist_url ?? "",
    cookie_file: overrides.cookie_file ?? "",
    output_dir: overrides.output_dir ?? outputDir,
    quality: $quality.value,
    format: $format.value,
    proxy: $proxy.value || null,
    include_comments: overrides.is_tiktok ? false : $commentsOnly.checked,
    auto_tag: $autoTag.checked,
    selected_indices: overrides.selected_indices ?? [],
    single_video: overrides.single_video ?? false,
    inject_metadata: $injectMetadata.checked,
    update_mode: $updateMode.checked,
    export_comments: $exportComments.value || null,
    download_subs: $downloadSubs.checked,
    sub_langs: getSubLangs(),
    auto_subs: true,
    write_info_json: $writeInfoJson.checked,
    flat_output: $flatOutput.checked,
    no_watermark: $noWatermark.checked,
    max_concurrent: parseInt($maxConcurrent.value) || 3,
    is_tiktok: overrides.is_tiktok ?? false,
  };
}

// ── Start download ─────────────────────────────────────────────────────

export async function startDownload(): Promise<void> {
  if ($start.disabled) return;
  $start.disabled = true;
  const url = (downloadMode === "video" ? $urlVideo.value.trim() : downloadMode === "tiktok" ? $urlTiktok.value.trim() : $url.value.trim());
  if (!url) {
    $start.disabled = false;
    (downloadMode === "video" ? $urlVideo : downloadMode === "tiktok" ? $urlTiktok : $url).focus();
    return;
  }

  const cookieFile = await getCookieFile();
  if (cookieFile === null) { $start.disabled = false; return; }

  if (!outputDir) {
    const selected = await open({ directory: true });
    if (!selected) { $start.disabled = false; return; }
    setOutputDir(selected);
    $output.value = selected;
  }

  // Single video mode
  if (downloadMode === "video") {
    setIsDownloading(true);
    setCheckboxState(true);
    $start.disabled = true;
    $stop.disabled = false;
    $folder.disabled = true;
    $log.innerHTML = "";
    $progressFill.style.width = "0%";
    $stats.textContent = "";
    $queue.innerHTML = `<div class="queue-empty">${t("fetching")}</div>`;

    try {
      await invoke("start_download", {
        settings: buildSettings({
          playlist_url: url,
          cookie_file: cookieFile || "",
          single_video: true,
        }),
      });
    } catch (e) {
      appendLog(`Error: ${e}`);
      setIsDownloading(false);
      setCheckboxState(false);
      $start.disabled = false;
      $stop.disabled = true;
    }
    return;
  }

  // TikTok mode — two-phase: fetch → cards → download
  if (downloadMode === "tiktok") {
    const selectedIndices: number[] = [];
    document.querySelectorAll<HTMLInputElement>(".video-check:checked").forEach((cb) => {
      const idx = parseInt(cb.dataset.index!);
      if (playlistVideos[idx] !== null) selectedIndices.push(idx);
    });

    if (playlistVideos.length > 0 && selectedIndices.length === 0) {
      alert("Please select at least one video.");
      $start.disabled = false;
      return;
    }

    setIsDownloading(true);
    setCheckboxState(true);
    $start.disabled = true;
    $stop.disabled = false;
    $folder.disabled = true;
    $log.innerHTML = "";
    $progressFill.style.width = "0%";
    $stats.textContent = "";
    setFailedIndices([]);

    const $redownload = document.getElementById("btn-redownload") as HTMLElement | null;
    if ($redownload) $redownload.style.display = "none";

    if (playlistVideos.length === 0) {
      $queue.innerHTML = `<div class="queue-empty">${t("fetching")}</div>`;
    }

    try {
      if (playlistVideos.length === 0) {
        const result = await invoke<PlaylistResult>("fetch_playlist", {
          url, cookieFile: cookieFile || "", proxy: $proxy.value || null,
        });
        renderQueue(result.videos, null);
        $log.innerHTML = "";
        appendLog(`${result.title} (${result.videos.length} videos)`);
        setIsDownloading(false);
        setCheckboxState(false);
        $stop.disabled = true;
        $folder.disabled = false;
        $start.disabled = false;
        $start.textContent = t("startDownload");
        return;
      }

      await invoke("start_download", {
        settings: buildSettings({
          playlist_url: url,
          cookie_file: cookieFile || "",
          is_tiktok: true,
          single_video: false,
          selected_indices: selectedIndices,
        }),
      });
    } catch (e) {
      appendLog(`Error: ${e}`);
      setIsDownloading(false);
      setCheckboxState(false);
      $start.disabled = false;
      $stop.disabled = true;
    }
    return;
  }

  // Playlist mode
  const selectedIndices: number[] = [];
  document.querySelectorAll<HTMLInputElement>(".video-check:checked").forEach((cb) => {
    const idx = parseInt(cb.dataset.index!);
    if (playlistVideos[idx] !== null) selectedIndices.push(idx);
  });

  if (playlistVideos.length > 0 && selectedIndices.length === 0) {
    alert("Please select at least one video.");
    $start.disabled = false;
    return;
  }

  setIsDownloading(true);
  setCheckboxState(true);
  $stop.disabled = false;
  $folder.disabled = true;
  $log.innerHTML = "";
  $progressFill.style.width = "0%";
  $stats.textContent = "";
  setFailedIndices([]);

  const $redownload = document.getElementById("btn-redownload") as HTMLElement | null;
  if ($redownload) $redownload.style.display = "none";

  if (playlistVideos.length === 0) {
    $queue.innerHTML = `<div class="queue-empty">${t("fetching")}</div>`;
  }

  try {
    if (playlistVideos.length === 0) {
      const result = await invoke<PlaylistResult>("fetch_playlist", {
        url, cookieFile: cookieFile || "", proxy: $proxy.value || null,
      });

      let existing: boolean[] | null = null;
      if ($updateMode.checked && outputDir) {
        try {
          existing = await invoke<boolean[]>("check_existing_videos", {
            outputDir, playlistTitle: result.title, videos: result.videos, flatOutput: $flatOutput.checked,
          });
        } catch { /* ignore */ }
      }

      renderQueue(result.videos, existing);
      $log.innerHTML = "";
      const newCount = existing ? existing.filter((e) => !e).length : result.videos.length;
      appendLog(`${result.title} (${result.videos.length} videos${existing ? `, ${newCount} new` : ""})`);
      setIsDownloading(false);
      setCheckboxState(false);
      $stop.disabled = true;
      $folder.disabled = false;
      $start.disabled = false;
      $start.textContent = t("startDownload");
      return;
    }

    await invoke("start_download", {
      settings: buildSettings({
        playlist_url: url,
        cookie_file: cookieFile || "",
        selected_indices: selectedIndices,
      }),
    });
  } catch (e) {
    appendLog(`Error: ${e}`);
    setIsDownloading(false);
    setCheckboxState(false);
    $start.disabled = false;
    $stop.disabled = true;
  }
}

// ── Redownload failed ──────────────────────────────────────────────────

export async function redownloadFailed(): Promise<void> {
  if (failedIndices.length === 0) return;
  const cookieFile = await getCookieFile();
  if (cookieFile === null) return;

  const $redownload = document.getElementById("btn-redownload") as HTMLElement;
  $redownload.style.display = "none";

  setIsDownloading(true);
  setCheckboxState(true);
  $start.disabled = true;
  $stop.disabled = false;
  $folder.disabled = true;
  $log.innerHTML = "";
  $progressFill.style.width = "0%";

  failedIndices.forEach((i) => {
    const el = document.getElementById(`status-${i + 1}`);
    if (el) { el.textContent = "Pending"; el.className = "status pending"; }
  });

  const indices = [...failedIndices];
  setFailedIndices([]);

  try {
    await invoke("start_download", {
      settings: buildSettings({
        playlist_url: $url.value.trim(),
        cookie_file: cookieFile || "",
        selected_indices: indices,
      }),
    });
  } catch (e) {
    appendLog(`Error: ${e}`);
    setIsDownloading(false);
    setCheckboxState(false);
    $start.disabled = false;
    $stop.disabled = true;
  }
}

// ── Event listeners from Rust ──────────────────────────────────────────

export function setupEventListeners(): void {
  listen<string>("download-log", (event) => appendLog(event.payload));

  listen<[number, string]>("download-status", (event) => {
    const [idx, status] = event.payload;
    const el = document.getElementById(`status-${idx}`);
    if (!el) return;
    if (status.startsWith("downloading")) {
      el.textContent = status;
      el.className = "status downloading";
    } else if (status === "Exists") {
      el.textContent = t("statusExists") || "Exists";
      el.className = "status done";
    } else {
      el.textContent = status;
      el.className = `status ${status}`;
    }
    const videoIdx = idx - 1;
    if (["Failed", "Members only", "Unavailable", "Cookie expired"].includes(status)) {
      if (!failedIndices.includes(videoIdx)) {
        setFailedIndices([...failedIndices, videoIdx]);
      }
    } else if (status === "done" || status === "Exists") {
      setFailedIndices(failedIndices.filter((i) => i !== videoIdx));
    }
  });

  listen<[number, string, string, string, string]>("video-progress", (event) => {
    const [idx, percent, speed, eta, size] = event.payload;
    const el = document.getElementById(`status-${idx}`);
    if (el) {
      el.textContent = `${percent}% ${speed}`;
      el.className = "status downloading";
    }
    let progEl = document.getElementById(`progress-${idx}`);
    if (!progEl) {
      const card = document.getElementById(`row-${idx}`);
      if (card) {
        progEl = document.createElement("div");
        progEl.id = `progress-${idx}`;
        progEl.className = "video-progress";
        progEl.innerHTML = `<div class="video-progress-bar"><div class="fill" style="width:0%"></div></div><span class="progress-meta"></span>`;
        card.querySelector(".info")?.appendChild(progEl);
      }
    }
    if (progEl) {
      const fill = progEl.querySelector(".fill") as HTMLElement;
      const meta = progEl.querySelector(".progress-meta") as HTMLElement;
      if (fill) fill.style.width = percent;
      if (meta) meta.textContent = `${speed} · ${eta} · ${size}`;
    }
  });

  listen<[number, number]>("download-progress", (event) => {
    const [current, total] = event.payload;
    $progressFill.style.width = Math.round((current / total) * 100) + "%";
    $stats.textContent = `${current}/${total}`;
  });

  listen<[number, number, string]>("download-done", (event) => {
    const [ok, total, folderPath] = event.payload;
    setIsDownloading(false);
    setCheckboxState(false);
    $start.disabled = false;
    $stop.disabled = true;
    $folder.disabled = false;
    $stats.textContent = `Done: ${ok}/${total}`;
    $progressFill.style.width = "100%";
    setActualDir(folderPath || outputDir);

    const $redownload = document.getElementById("btn-redownload");
    if ($redownload && failedIndices.length > 0) {
      $redownload.style.display = "inline-block";
      $redownload.textContent = `${t("redownload")} (${failedIndices.length})`;
    }
  });
}
