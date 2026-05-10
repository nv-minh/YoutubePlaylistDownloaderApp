import type { VideoInfo } from "./types";
import {
  $queue, $log, isDownloading, playlistVideos, failedIndices,
  setPlaylistVideos, setFailedIndices,
} from "./dom";
import { t } from "./i18n";

export const deleteIcon = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>`;

export function toggleAllVideos(checked: boolean): void {
  if (isDownloading) return;
  document.querySelectorAll<HTMLInputElement>(".video-check").forEach((cb) => { cb.checked = checked; });
  updateCardStyles();
}

export function setCheckboxState(disabled: boolean): void {
  document.querySelectorAll<HTMLInputElement>(".video-check").forEach((cb) => { cb.disabled = disabled; });
  const selectAll = document.getElementById("select-all") as HTMLInputElement | null;
  if (selectAll) selectAll.disabled = disabled;
}

export function updateCardStyles(): void {
  document.querySelectorAll<HTMLElement>(".video-card").forEach((card) => {
    const cb = card.querySelector<HTMLInputElement>(".video-check");
    if (cb) card.classList.toggle("unchecked", !cb.checked);
  });
  const total = document.querySelectorAll<HTMLInputElement>(".video-check").length;
  const checked = document.querySelectorAll<HTMLInputElement>(".video-check:checked").length;
  const countEl = document.querySelector<HTMLElement>(".queue-header .count");
  if (countEl) countEl.textContent = `${checked}/${total} ${t("selected")}`;
}

export function appendLog(text: string): void {
  const line = document.createElement("div");
  line.textContent = text;
  $log.appendChild(line);
  $log.scrollTop = $log.scrollHeight;
}

export function escapeHtml(text: string): string {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

export function formatDuration(sec: number): string {
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  const h = Math.floor(m / 60);
  const rm = m % 60;
  return h > 0
    ? `${h}:${String(rm).padStart(2, "0")}:${String(s).padStart(2, "0")}`
    : `${rm}:${String(s).padStart(2, "0")}`;
}

export function renderQueue(videos: VideoInfo[], existing: boolean[] | null = null): void {
  setPlaylistVideos(videos);
  const checkedCount = existing ? existing.filter((e) => !e).length : videos.length;
  const allChecked = !existing || checkedCount === videos.length;

  const header = `
    <div class="queue-header">
      <label class="select-all">
        <input type="checkbox" id="select-all" ${allChecked ? "checked" : ""} />
        <span>${t("selectAll")}</span>
      </label>
      <span class="count">${checkedCount}/${videos.length} ${t("selected")}</span>
    </div>`;

  const cards = videos.map((v, i) => {
    const dur = v.duration ? formatDuration(v.duration) : "";
    const isExisting = existing?.[i];
    return `
    <div class="video-card${isExisting ? " unchecked" : ""}" id="row-${i + 1}">
      <div class="check-col">
        <input type="checkbox" class="video-check" data-index="${i}" ${isExisting ? "" : "checked"} />
      </div>
      <img class="thumb" src="${v.thumbnail}" alt="" onerror="this.style.display='none'" />
      <div class="info">
        <div class="title">${escapeHtml(v.title)}</div>
        <div class="channel">${escapeHtml(v.channel)}</div>
        <div class="meta">${dur ? `<span class="dur">${dur}</span>` : ""}</div>
      </div>
      <span class="status ${isExisting ? "done" : "pending"}" id="status-${i + 1}">${isExisting ? (t("statusExists") || "Exists") : "Pending"}</span>
      <button class="btn-delete-card" data-index="${i}" title="${t("deleteVideo")}">${deleteIcon}</button>
    </div>`;
  }).join("");

  $queue.innerHTML = header + cards;

  document.getElementById("select-all")!.addEventListener("change", (e) => {
    toggleAllVideos((e.target as HTMLInputElement).checked);
  });
  document.querySelectorAll<HTMLInputElement>(".video-check").forEach((cb) => {
    cb.addEventListener("change", updateCardStyles);
  });
  document.querySelectorAll<HTMLElement>(".video-card").forEach((card) => {
    card.addEventListener("click", (e) => {
      if (isDownloading) return;
      if ((e.target as HTMLElement).tagName === "INPUT" || (e.target as HTMLElement).tagName === "A" || (e.target as HTMLElement).tagName === "BUTTON" || (e.target as HTMLElement).closest(".btn-delete-card")) return;
      const cb = card.querySelector<HTMLInputElement>(".video-check")!;
      cb.checked = !cb.checked;
      updateCardStyles();
    });
  });
  document.querySelectorAll<HTMLElement>(".btn-delete-card").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      const idx = parseInt((btn as HTMLElement).dataset.index!);
      const card = document.getElementById(`row-${idx + 1}`);
      if (card) {
        card.style.transition = "opacity 0.2s, transform 0.2s";
        card.style.opacity = "0";
        card.style.transform = "translateX(20px)";
        setTimeout(() => {
          card.remove();
          playlistVideos[idx] = null;
          setFailedIndices(failedIndices.filter((fi) => fi !== idx));
          updateCardStyles();
        }, 200);
      }
    });
  });
}
