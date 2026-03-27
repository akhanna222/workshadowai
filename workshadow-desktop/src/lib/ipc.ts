import { invoke } from "@tauri-apps/api/core";

// ── Types ──

export interface SearchFilters {
  date_from?: number;
  date_to?: number;
  app_ids?: string[];
  url_domains?: string[];
}

export interface SearchResult {
  frame_id: number;
  timestamp_ms: number;
  matched_text: string;
  window_title: string;
  app_id: string;
  relevance_score: number;
}

export interface FrameSummary {
  frame_id: number;
  timestamp_ms: number;
  window_title: string;
  app_id: string;
  thumbnail_path: string | null;
}

export interface FrameDetail {
  frame_id: number;
  timestamp_ms: number;
  window_title: string;
  app_id: string;
  browser_url: string | null;
  ocr_text: string | null;
  pii_flags: string[] | null;
  segment_file: string;
  segment_offset: number;
}

export type CaptureState = "Idle" | "Recording" | "Paused";

export interface CaptureStatus {
  state: CaptureState;
  session_id: number | null;
  frames_captured: number;
  current_fps: number;
  recording_since_ms: number | null;
}

export interface StorageUsage {
  total_frames: number;
  total_sessions: number;
  disk_usage_bytes: number;
  oldest_frame_ms: number | null;
  newest_frame_ms: number | null;
}

export interface CaptureConfig {
  fps: number;
  idle_fps: number;
  idle_threshold_secs: number;
  max_resolution: [number, number];
  segment_duration_secs: number;
  multi_monitor: boolean;
}

export interface StorageConfig {
  data_dir: string;
  max_retention_days: number;
  max_storage_gb: number;
  cleanup_interval_hours: number;
}

export interface OcrConfig {
  enabled: boolean;
  language: string;
  dedup_threshold: number;
  pii_detection: boolean;
  quality_threshold: number;
  deepseek_model_path: string | null;
  quality_reanalysis: boolean;
}

export interface OcrStatus {
  fast_backend: string;
  quality_available: boolean;
  quality_model: string;
}

export interface PrivacyConfig {
  excluded_apps: string[];
  excluded_url_patterns: string[];
  recording_indicator: boolean;
  global_hotkey_pause: string;
}

export interface SearchConfig {
  index_dir: string;
  max_results: number;
}

export interface AppConfig {
  capture: CaptureConfig;
  storage: StorageConfig;
  ocr: OcrConfig;
  privacy: PrivacyConfig;
  search: SearchConfig;
}

export interface DailySummary {
  date: string;
  total_frames: number;
  hours_by_app: [string, number][];
  top_urls: [string, number][];
  top_windows: [string, number][];
}

// ── IPC Commands ──

export const ipc = {
  search: (query: string, filters?: SearchFilters) =>
    invoke<SearchResult[]>("search", { query, filters }),

  getTimelineRange: (startMs: number, endMs: number) =>
    invoke<FrameSummary[]>("get_timeline_range", { startMs, endMs }),

  getFrame: (frameId: number) =>
    invoke<FrameDetail | null>("get_frame", { frameId }),

  startCapture: () => invoke<void>("start_capture"),

  pauseCapture: () => invoke<void>("pause_capture"),

  resumeCapture: () => invoke<void>("resume_capture"),

  stopCapture: () => invoke<void>("stop_capture"),

  getCaptureStatus: () => invoke<CaptureStatus>("get_capture_status"),

  getSettings: () => invoke<AppConfig>("get_settings"),

  updateSettings: (newConfig: AppConfig) =>
    invoke<void>("update_settings", { newConfig }),

  getStorageUsage: () => invoke<StorageUsage>("get_storage_usage"),

  getDailySummary: (date: string) =>
    invoke<DailySummary>("get_daily_summary", { date }),

  getOcrStatus: () => invoke<OcrStatus>("get_ocr_status"),

  reanalyzeFrame: (frameId: number) =>
    invoke<string>("reanalyze_frame", { frameId }),

  downloadQualityModel: () => invoke<string>("download_quality_model"),

  deleteTimeRange: (startMs: number, endMs: number) =>
    invoke<number>("delete_time_range", { startMs, endMs }),

  getAuditLog: () =>
    invoke<{ timestamp: string; event: string }[]>("get_audit_log"),

  getPrivacyStatus: () =>
    invoke<{
      encryption_active: boolean;
      excluded_apps_count: number;
      excluded_url_patterns_count: number;
      audit_log_entries: number;
    }>("get_privacy_status"),
};
