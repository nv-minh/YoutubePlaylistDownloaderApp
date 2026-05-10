export interface VideoInfo {
  id: string;
  title: string;
  channel: string;
  duration?: number;
  thumbnail: string;
}

export interface PlaylistResult {
  title: string;
  videos: VideoInfo[];
}

export interface DownloadSettings {
  playlist_url: string;
  cookie_file: string;
  output_dir: string;
  quality: string;
  format: string;
  proxy: string | null;
  include_comments: boolean;
  auto_tag: boolean;
  selected_indices: number[];
  single_video: boolean;
  inject_metadata: boolean;
  update_mode: boolean;
  export_comments: string | null;
  download_subs: boolean;
  sub_langs: string | null;
  auto_subs: boolean;
  write_info_json: boolean;
  flat_output: boolean;
  no_watermark: boolean;
  max_concurrent: number;
  is_tiktok: boolean;
}

export type Lang = "vi" | "en";

export type TranslationKey =
  | "appTitle" | "playlistUrl" | "urlPlaceholder" | "accessType"
  | "tabPublic" | "tabPrivate" | "publicHint" | "cookieStep1"
  | "cookieStep2" | "cookieStep3" | "cookieStep4" | "cookiePlaceholder"
  | "saveTo" | "outputPlaceholder" | "browse" | "videoQuality"
  | "outputFormat" | "proxy" | "autoTag" | "commentsOnly"
  | "injectMetadata" | "updateMode" | "exportComments" | "exportNone"
  | "statusExists" | "downloadSubs" | "writeInfoJson" | "flatOutput"
  | "subLangs" | "subLangCustom" | "deleteVideo" | "startDownload"
  | "stop" | "openFolder" | "clear" | "queueEmpty" | "selectAll"
  | "selected" | "noCookieAlert" | "fetching" | "redownload"
  | "tabPlaylist" | "tabVideo" | "videoUrl" | "videoUrlPlaceholder"
  | "tabTiktok" | "tiktokUrl" | "tiktokUrlPlaceholder" | "noWatermark"
  | "parallelDownloads" | "fetchInfo";
