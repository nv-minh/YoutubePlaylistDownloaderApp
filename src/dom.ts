import type { VideoInfo } from "./types";

// ── DOM Elements ───────────────────────────────────────────────────────

export const $url = document.getElementById("url") as HTMLInputElement;
export const $urlVideos = document.getElementById("url-videos") as HTMLTextAreaElement;
export const $urlCount = document.getElementById("url-count") as HTMLElement;
export const $urlTiktok = document.getElementById("url-tiktok") as HTMLTextAreaElement;
export const $urlCountTiktok = document.getElementById("url-count-tiktok") as HTMLElement;
export const $cookieText = document.getElementById("cookie-text") as HTMLTextAreaElement;
export const $output = document.getElementById("output") as HTMLInputElement;
export const $quality = document.getElementById("quality") as HTMLSelectElement;
export const $format = document.getElementById("format") as HTMLSelectElement;
export const $proxy = document.getElementById("proxy") as HTMLInputElement;
export const $autoTag = document.getElementById("auto-tag") as HTMLInputElement;
export const $commentsOnly = document.getElementById("comments-only") as HTMLInputElement;
export const $injectMetadata = document.getElementById("inject-metadata") as HTMLInputElement;
export const $updateMode = document.getElementById("update-mode") as HTMLInputElement;
export const $exportComments = document.getElementById("export-comments") as HTMLSelectElement;
export const $downloadSubs = document.getElementById("download-subs") as HTMLInputElement;
export const $writeInfoJson = document.getElementById("write-info-json") as HTMLInputElement;
export const $flatOutput = document.getElementById("flat-output") as HTMLInputElement;
export const $subLangSelect = document.getElementById("sub-lang-select") as HTMLSelectElement;
export const $subLangs = document.getElementById("sub-langs") as HTMLInputElement;
export const $subCustom = document.getElementById("sub-custom") as HTMLElement;
export const $subOptions = document.getElementById("sub-options") as HTMLElement;
export const $start = document.getElementById("btn-start") as HTMLButtonElement;
export const $stop = document.getElementById("btn-stop") as HTMLButtonElement;
export const $queue = document.getElementById("queue") as HTMLElement;
export const $log = document.getElementById("log") as HTMLElement;
export const $progressFill = document.getElementById("progress-fill") as HTMLElement;
export const $stats = document.getElementById("stats") as HTMLElement;
export const $folder = document.getElementById("btn-folder") as HTMLButtonElement;
export const $ytdlpStatus = document.getElementById("ytdlp-status") as HTMLElement;
export const $lang = document.getElementById("lang") as HTMLSelectElement;
export const $noWatermark = document.getElementById("no-watermark") as HTMLInputElement;
export const $maxConcurrent = document.getElementById("max-concurrent") as HTMLInputElement;
export const $maxConcurrentVal = document.getElementById("max-concurrent-val") as HTMLElement;
export const $themeToggle = document.getElementById("theme-toggle") as HTMLButtonElement;

// ── App State ──────────────────────────────────────────────────────────

export let isDownloading = false;
export let outputDir = "";
export let actualDir = "";
export let accessType = "public";
export let downloadMode: "playlist" | "videos" | "tiktok" = "playlist";
export let playlistVideos: (VideoInfo | null)[] = [];
export let failedIndices: number[] = [];
export let cookieErrorIndices: number[] = [];
export let impSupported = false;

export function setIsDownloading(v: boolean): void { isDownloading = v; }
export function setOutputDir(v: string): void { outputDir = v; }
export function setActualDir(v: string): void { actualDir = v; }
export function setAccessType(v: "public" | "private"): void { accessType = v; }
export function setDownloadMode(v: "playlist" | "videos" | "tiktok"): void { downloadMode = v; }

export function parseVideoUrls(text: string): string[] {
  return text
    .split(/[\n,]+/)
    .map(s => s.trim())
    .filter(s => s.length > 0 && /^https?:\/\//.test(s));
}
export function setPlaylistVideos(v: (VideoInfo | null)[]): void { playlistVideos = v; }
export function setFailedIndices(v: number[]): void { failedIndices = v; }
export function setCookieErrorIndices(v: number[]): void { cookieErrorIndices = v; }
export function setImpSupported(v: boolean): void { impSupported = v; }
