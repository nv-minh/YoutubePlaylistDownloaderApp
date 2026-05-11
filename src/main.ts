import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { applyTranslations } from "./i18n";
import {
  $url, $urlVideos, $urlCount, $urlTiktok, $urlCountTiktok, $output, $downloadSubs, $subLangSelect,
  $subCustom, $subOptions, $start, $stop, $folder, $log,
  $progressFill, $stats, $queue, $lang,
  $noWatermark, $maxConcurrent, $maxConcurrentVal, $themeToggle,
  outputDir, actualDir, playlistVideos, failedIndices,
  setOutputDir, setActualDir, setPlaylistVideos, setFailedIndices, setAccessType, setDownloadMode,
  parseVideoUrls,
} from "./dom";
import { checkYtdlp, startDownload, redownloadFailed, setupEventListeners } from "./download";
import { t } from "./i18n";
import { appendLog } from "./ui";

// ── Language switch ────────────────────────────────────────────────────
$lang.addEventListener("change", () => applyTranslations());

// ── Theme toggle ──────────────────────────────────────────────────────
const savedTheme = localStorage.getItem("theme") || "dark";
document.documentElement.setAttribute("data-theme", savedTheme);
$themeToggle.addEventListener("click", () => {
  const current = document.documentElement.getAttribute("data-theme") || "dark";
  const next = current === "dark" ? "light" : "dark";
  document.documentElement.setAttribute("data-theme", next);
  localStorage.setItem("theme", next);
});

// ── Parallel downloads slider ─────────────────────────────────────────
$maxConcurrent.addEventListener("input", () => {
  $maxConcurrentVal.textContent = $maxConcurrent.value;
});

// ── Subtitle toggle ────────────────────────────────────────────────────
$downloadSubs.addEventListener("change", () => {
  $subOptions.style.display = $downloadSubs.checked ? "block" : "none";
});

$subLangSelect.addEventListener("change", () => {
  $subCustom.style.display = $subLangSelect.value === "custom" ? "block" : "none";
});

// ── Multi-URL counter ─────────────────────────────────────────────────
function updateUrlCount(textarea: HTMLTextAreaElement, counter: HTMLElement): void {
  const urls = parseVideoUrls(textarea.value);
  if (urls.length > 0) {
    counter.textContent = `${urls.length} URL${urls.length > 1 ? "s" : ""}`;
    counter.classList.add("has-urls");
  } else {
    counter.textContent = "";
    counter.classList.remove("has-urls");
  }
}

$urlVideos.addEventListener("input", () => updateUrlCount($urlVideos, $urlCount));
$urlTiktok.addEventListener("input", () => updateUrlCount($urlTiktok, $urlCountTiktok));

// Dynamic button label: "Download Now" for single video URLs, "Fetch Info" otherwise
function updateStartButtonLabel(): void {
  if (downloadMode === "videos") {
    $start.textContent = t("startDownload");
    return;
  }
  if (downloadMode === "tiktok") {
    const urls = parseVideoUrls($urlTiktok.value);
    if (urls.length === 1 && !/^https?:\/\/(www\.)?tiktok\.com\/@\w[\w.-]*\/?$/.test(urls[0].trim())) {
      $start.textContent = t("startDownload");
    } else {
      $start.textContent = t("fetchInfo");
    }
    return;
  }
  // Playlist mode
  const val = $url.value.trim();
  const isSingleVideo = /^https?:\/\/(www\.)?(youtube\.com\/watch\?|youtu\.be\/|youtube\.com\/shorts\/)/.test(val)
    && !/[?&]list=/.test(val);
  $start.textContent = isSingleVideo ? t("startDownload") : t("fetchInfo");
}
$url.addEventListener("input", updateStartButtonLabel);
$urlTiktok.addEventListener("input", updateStartButtonLabel);

// ── Access tab switching ───────────────────────────────────────────────
document.querySelectorAll<HTMLElement>(".tab").forEach((tab) => {
  tab.addEventListener("click", () => {
    if (tab.dataset.mode) {
      // Mode tabs: deactivate ALL mode tabs across both groups, then activate clicked
      document.querySelectorAll<HTMLElement>(".tab-group .tab").forEach((t) => t.classList.remove("active"));
    } else {
      // Other tabs (access type): only deactivate within same group
      const group = tab.closest<HTMLElement>(".tabs")!;
      group.querySelectorAll<HTMLElement>(".tab").forEach((t) => t.classList.remove("active"));
    }
    tab.classList.add("active");
    if (tab.dataset.tab) {
      setAccessType(tab.dataset.tab as "public" | "private");
      document.querySelectorAll<HTMLElement>(".tab-content").forEach((c) => {
        if (c.id.startsWith("content-")) c.classList.remove("active");
      });
      document.getElementById(`content-${tab.dataset.tab}`)!.classList.add("active");
    }
    if (tab.dataset.mode) {
      setDownloadMode(tab.dataset.mode as "playlist" | "videos" | "tiktok");
      document.querySelectorAll<HTMLElement>(".mode-content").forEach((c) => c.classList.remove("active"));
      document.getElementById(`mode-${tab.dataset.mode}`)!.classList.add("active");
      const isTiktok = tab.dataset.mode === "tiktok";
      document.querySelectorAll<HTMLElement>(".yt-only").forEach((el) => {
        el.style.display = isTiktok ? "none" : "";
      });
      // Reset button label and queue on mode switch
      updateStartButtonLabel();
      $start.disabled = false;
      setPlaylistVideos([]);
      setFailedIndices([]);
      $queue.innerHTML = `<div class="queue-empty" data-i18n="queueEmpty">${t("queueEmpty")}</div>`;
    }
  });
});

// ── Folder picker ──────────────────────────────────────────────────────
document.getElementById("btn-output")!.addEventListener("click", async () => {
  const selected = await open({ directory: true });
  if (selected) { $output.value = selected; setOutputDir(selected); }
});

// ── Clear ──────────────────────────────────────────────────────────────
document.getElementById("btn-clear")!.addEventListener("click", () => {
  setPlaylistVideos([]);
  setFailedIndices([]);
  setActualDir("");
  $url.value = "";
  $urlVideos.value = "";
  $urlCount.textContent = "";
  $urlTiktok.value = "";
  $urlCountTiktok.textContent = "";
  $log.innerHTML = "";
  $progressFill.style.width = "0%";
  $stats.textContent = "";
  $start.disabled = false;
  $start.textContent = t("fetchInfo");
  $stop.disabled = true;
  $folder.disabled = true;
  const $redownload = document.getElementById("btn-redownload");
  if ($redownload) $redownload.style.display = "none";
  const $cookieHint = document.getElementById("cookie-hint");
  if ($cookieHint) $cookieHint.style.display = "none";
  $queue.innerHTML = `<div class="queue-empty" data-i18n="queueEmpty">${t("queueEmpty")}</div>`;
});

// ── Start / Stop / Folder / Redownload ─────────────────────────────────
$start.addEventListener("click", startDownload);

$stop.addEventListener("click", () => {
  invoke("cancel_download");
  $stop.disabled = true;
  appendLog("Cancelling...");
});

$folder.addEventListener("click", () => {
  const dir = actualDir || outputDir;
  if (dir) invoke("open_folder", { path: dir });
});

document.getElementById("btn-redownload")!.addEventListener("click", redownloadFailed);

// ── Init ───────────────────────────────────────────────────────────────
window.addEventListener("DOMContentLoaded", () => {
  applyTranslations();
  checkYtdlp();
  setupEventListeners();
  $start.textContent = t("fetchInfo");
});
