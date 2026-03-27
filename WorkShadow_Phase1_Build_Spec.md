# WorkShadow AI — Phase 1 Build Specification

**Version:** 1.0  
**Date:** March 2026  
**Status:** Engineering Ready  
**Scope:** Capture Agent MVP + Local Search  

---

## 1. Phase 1 Objective

Ship a lightweight desktop agent that continuously captures screen activity, extracts text via OCR, and provides a local searchable timeline of everything the user has seen or done — with zero cloud dependency.

**Success criteria:**
- Installs in < 2 minutes on macOS and Windows
- Runs at < 5% CPU and < 500MB RAM in background
- Captures full workday (8h) in < 3GB storage
- Returns search results in < 2 seconds
- Zero data leaves the device

---

## 2. System Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Tauri Desktop Shell                │
│              (Rust backend + Web frontend)           │
├──────────┬──────────┬───────────┬───────────────────┤
│ Capture  │   OCR    │  Storage  │  Search / Query   │
│ Engine   │ Pipeline │  Engine   │  Interface        │
│ (Rust)   │ (Rust)   │ (SQLite)  │  (React + Rust)   │
└──────────┴──────────┴───────────┴───────────────────┘
     │           │           │              │
     ▼           ▼           ▼              ▼
  Screen      Tesseract   Local DB     Tantivy FTS
  Frames      / PaddleOCR  + Frames    (full-text
  (H.265)                              search index)
```

---

## 3. Component Specifications

### 3.1 Capture Engine

**Responsibility:** Continuous screen frame and active-window metadata capture.

| Parameter | Value |
|---|---|
| Default frame rate | 1 fps (configurable 0.5–5 fps) |
| Encoding | H.265 (HEVC) via FFmpeg with hardware acceleration |
| Resolution | Native display, downscaled to 1920x1080 max |
| Metadata per frame | Timestamp (ms), active window title, app bundle ID, URL (if browser) |
| Encryption | AES-256-GCM at rest, key derived from OS keychain |
| Storage format | Frame chunks in 5-minute segments (.mkv) + metadata sidecar (.jsonl) |

**Platform-specific capture APIs:**
- **macOS:** `CGDisplayStream` (ScreenCaptureKit on macOS 13+)
- **Windows:** DXGI Desktop Duplication API
- **Linux (stretch):** PipeWire screen capture portal

**Implementation notes:**
- Capture runs in a dedicated Rust thread with bounded channel (backpressure if processing lags)
- Hardware encoder detection at startup: VideoToolbox (macOS), NVENC/QSV/AMF (Windows)
- Fallback to libx265 software encoding if no hardware encoder found
- Adaptive frame rate: drop to 0.5 fps when system is idle (no window change for 30s)

```rust
// Core capture loop pseudocode
pub struct CaptureConfig {
    pub fps: f32,              // default 1.0
    pub max_resolution: (u32, u32), // (1920, 1080)
    pub segment_duration_secs: u32, // 300 (5 min)
    pub idle_threshold_secs: u32,   // 30
    pub idle_fps: f32,              // 0.5
}

pub struct FrameMetadata {
    pub timestamp_ms: u64,
    pub active_window_title: String,
    pub app_bundle_id: String,
    pub browser_url: Option<String>,
    pub display_index: u8,
}
```

### 3.2 OCR Pipeline

**Responsibility:** Extract all visible text from each captured frame.

| Parameter | Value |
|---|---|
| Engine | PaddleOCR-Lite (Rust bindings via ONNX Runtime) |
| Trigger | Every captured frame (1 fps = 1 OCR pass/sec) |
| Output | Array of `{text, bounding_box, confidence}` per frame |
| Language | English default; configurable for multi-language |
| Processing | Async pipeline, separate thread pool (2 threads) |
| PII detection | Regex-based pre-filter for emails, phone numbers, SSNs, credit cards — flagged but not deleted |

**Pipeline flow:**
1. Frame received from capture engine via channel
2. Downscale to 1280x720 for OCR (speed optimisation)
3. Run PaddleOCR inference (ONNX Runtime, CPU or CoreML/DirectML)
4. Deduplicate against previous frame's text (Jaccard similarity > 0.9 = skip insert)
5. Write extracted text + bounding boxes to SQLite
6. Index text in Tantivy full-text search

```rust
pub struct OcrResult {
    pub frame_timestamp_ms: u64,
    pub text_blocks: Vec<TextBlock>,
    pub full_text: String,  // concatenated, for search indexing
}

pub struct TextBlock {
    pub text: String,
    pub confidence: f32,
    pub bbox: (f32, f32, f32, f32), // x, y, width, height (normalised 0-1)
}
```

### 3.3 Storage Engine

**Responsibility:** Persist frames, metadata, and OCR text with efficient retrieval and automatic cleanup.

**Database: SQLite (single file, WAL mode)**

```sql
-- Core tables
CREATE TABLE sessions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at  INTEGER NOT NULL, -- unix ms
    ended_at    INTEGER,
    total_frames INTEGER DEFAULT 0
);

CREATE TABLE frames (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      INTEGER NOT NULL REFERENCES sessions(id),
    timestamp_ms    INTEGER NOT NULL,
    segment_file    TEXT NOT NULL,    -- relative path to .mkv chunk
    segment_offset  INTEGER NOT NULL, -- frame index within segment
    window_title    TEXT,
    app_id          TEXT,
    browser_url     TEXT,
    ocr_text        TEXT,            -- full concatenated text
    pii_flags       TEXT,            -- JSON array of detected PII types
    UNIQUE(session_id, timestamp_ms)
);

CREATE INDEX idx_frames_timestamp ON frames(timestamp_ms);
CREATE INDEX idx_frames_app ON frames(app_id);
CREATE INDEX idx_frames_window ON frames(window_title);

-- Retention policy table
CREATE TABLE retention_policy (
    id              INTEGER PRIMARY KEY,
    max_days        INTEGER DEFAULT 90,
    max_gb          REAL DEFAULT 50.0
);
```

**Storage management:**
- Retention policy: configurable max days (default 90) and max disk usage (default 50GB)
- Cleanup job runs hourly: deletes oldest segments when either limit exceeded
- Estimated storage: ~350MB/hour at 1fps with H.265 = ~2.8GB per 8h workday

**Tantivy full-text search index:**
- Indexed fields: `ocr_text`, `window_title`, `app_id`, `browser_url`
- Stored fields: `frame_id`, `timestamp_ms`
- Index rebuilt incrementally as new OCR results arrive
- Supports BM25 ranking and phrase queries

### 3.4 Search and Query Interface

**Responsibility:** Web-based UI served locally by Tauri for timeline browsing and text search.

**Core views:**

#### Timeline View (Default)
- Horizontal scrollable timeline showing thumbnail strips grouped by hour
- Each thumbnail = one frame, colour-coded by application (browser = blue, IDE = green, etc.)
- Click thumbnail → full-size frame with OCR overlay and metadata panel
- Hover → preview tooltip with window title and timestamp

#### Search View
- Full-text search bar with Tantivy backend
- Results rendered as timestamped cards: frame thumbnail + matched text snippet + app icon
- Filters: date range, application, URL domain
- Sort: relevance (BM25) or chronological

#### Activity Summary View (Stretch)
- Daily breakdown: hours per application, top URLs visited, most frequent window titles
- Simple bar charts rendered with Chart.js

**Tech stack for UI:**
- React 18 + TypeScript
- TailwindCSS for styling
- Tauri IPC for Rust ↔ JS communication
- No external network calls — all assets bundled

```typescript
// Core IPC commands exposed to frontend
interface WorkShadowIPC {
  // Search
  search(query: string, filters?: SearchFilters): Promise<SearchResult[]>;

  // Timeline
  getTimelineRange(startMs: number, endMs: number): Promise<FrameSummary[]>;
  getFrame(frameId: number): Promise<FrameDetail>;
  getFrameImage(frameId: number): Promise<Uint8Array>;

  // Control
  startCapture(): Promise<void>;
  pauseCapture(): Promise<void>;
  getCaptureStatus(): Promise<CaptureStatus>;

  // Settings
  getSettings(): Promise<Settings>;
  updateSettings(settings: Partial<Settings>): Promise<void>;

  // Stats
  getDailySummary(date: string): Promise<DailySummary>;
  getStorageUsage(): Promise<StorageUsage>;
}

interface SearchFilters {
  dateFrom?: number;   // unix ms
  dateTo?: number;
  appIds?: string[];
  urlDomains?: string[];
}

interface SearchResult {
  frameId: number;
  timestampMs: number;
  matchedText: string;
  windowTitle: string;
  appId: string;
  relevanceScore: number;
  thumbnailPath: string;
}
```

---

## 4. Privacy and Security Spec

| Requirement | Implementation |
|---|---|
| Data residency | 100% local — no network calls, no telemetry, no cloud sync |
| Encryption at rest | AES-256-GCM; key stored in macOS Keychain / Windows DPAPI |
| Encryption in transit | N/A (no network) — Tauri IPC is in-process |
| PII detection | Regex patterns for email, phone, SSN, credit card at OCR stage |
| App exclusion list | User-configurable list of apps/URLs to never capture (e.g., banking, password managers) |
| Recording indicator | Persistent system tray icon with red dot when actively recording |
| Pause/resume | Global hotkey (Cmd+Shift+P / Ctrl+Shift+P) for instant pause |
| Data deletion | User can delete any time range from UI; segments purged from disk immediately |
| Audit log | Local append-only log of all capture start/stop/delete events |

**Default exclusion list (pre-configured):**
- 1Password, Bitwarden, LastPass, KeePass
- Banking apps (configurable by URL pattern)
- Signal, WhatsApp Desktop (messaging privacy)

---

## 5. Configuration and Settings

```toml
# ~/.workshadow/config.toml

[capture]
fps = 1.0
idle_fps = 0.5
idle_threshold_secs = 30
max_resolution = [1920, 1080]
segment_duration_secs = 300
multi_monitor = true           # capture all displays

[storage]
data_dir = "~/.workshadow/data"
max_retention_days = 90
max_storage_gb = 50.0
cleanup_interval_hours = 1

[ocr]
enabled = true
language = "en"
dedup_threshold = 0.9          # Jaccard similarity to skip duplicate frames
pii_detection = true

[privacy]
excluded_apps = ["1Password", "Bitwarden"]
excluded_url_patterns = ["*bank*", "*paypal*"]
recording_indicator = true
global_hotkey_pause = "CmdOrCtrl+Shift+P"

[search]
index_dir = "~/.workshadow/index"
max_results = 50
```

---

## 6. Build and Distribution

| Aspect | Choice |
|---|---|
| Framework | Tauri 2.x (Rust backend + web frontend) |
| Language (backend) | Rust (2021 edition) |
| Language (frontend) | TypeScript + React 18 |
| Build system | Cargo + Vite |
| Packaging | Tauri bundler → .dmg (macOS), .msi (Windows) |
| Auto-update | Tauri updater plugin (optional, disabled by default for air-gapped installs) |
| CI/CD | GitHub Actions — build matrix for macOS (arm64, x86_64) + Windows (x86_64) |
| Code signing | Apple Developer ID + Windows Authenticode (required for distribution) |
| Min OS | macOS 13 Ventura / Windows 10 21H2 |

**Key Rust crates:**
- `tauri` — app framework
- `ffmpeg-next` — video encoding/decoding
- `ort` (ONNX Runtime) — OCR inference
- `rusqlite` — SQLite with WAL
- `tantivy` — full-text search
- `aes-gcm` — encryption
- `keyring` — OS keychain access
- `notify-rust` — system notifications
- `global-hotkey` — keyboard shortcuts

---

## 7. Dependency Tree

```
workshadow-desktop/
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── main.rs
│   │   ├── capture/
│   │   │   ├── mod.rs
│   │   │   ├── screen.rs       # Platform-specific capture
│   │   │   ├── encoder.rs      # H.265 encoding
│   │   │   └── metadata.rs     # Window title / URL extraction
│   │   ├── ocr/
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs       # PaddleOCR via ONNX
│   │   │   ├── dedup.rs        # Frame text deduplication
│   │   │   └── pii.rs          # PII detection regexes
│   │   ├── storage/
│   │   │   ├── mod.rs
│   │   │   ├── db.rs           # SQLite operations
│   │   │   ├── segments.rs     # Video segment management
│   │   │   └── retention.rs    # Cleanup policies
│   │   ├── search/
│   │   │   ├── mod.rs
│   │   │   └── index.rs        # Tantivy indexing + query
│   │   ├── privacy/
│   │   │   ├── mod.rs
│   │   │   ├── encryption.rs   # AES-256-GCM
│   │   │   ├── exclusions.rs   # App/URL filtering
│   │   │   └── audit.rs        # Event logging
│   │   ├── ipc/
│   │   │   └── commands.rs     # Tauri IPC command handlers
│   │   └── config.rs           # TOML config loader
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                        # React frontend
│   ├── App.tsx
│   ├── views/
│   │   ├── Timeline.tsx
│   │   ├── Search.tsx
│   │   ├── Settings.tsx
│   │   └── Summary.tsx
│   ├── components/
│   │   ├── FrameCard.tsx
│   │   ├── SearchBar.tsx
│   │   ├── TimelineStrip.tsx
│   │   ├── FilterPanel.tsx
│   │   └── TrayMenu.tsx
│   ├── hooks/
│   │   ├── useSearch.ts
│   │   ├── useTimeline.ts
│   │   └── useCaptureStatus.ts
│   └── lib/
│       └── ipc.ts              # Typed Tauri invoke wrappers
├── models/                     # ONNX model files
│   └── paddleocr-lite/
├── package.json
├── vite.config.ts
└── README.md
```

---

## 8. Performance Budgets

| Metric | Target | Measurement |
|---|---|---|
| CPU (idle, recording) | < 3% | Activity Monitor / Task Manager avg over 1h |
| CPU (active OCR) | < 8% peak | Per-frame OCR processing spike |
| RAM (steady state) | < 400MB | RSS after 4h continuous recording |
| Disk I/O | < 5 MB/s sustained | iostat avg during recording |
| Storage per hour | ~350MB | H.265 at 1fps, 1080p downscale |
| Storage per workday (8h) | ~2.8GB | Measured over 5 test days |
| Search latency (p95) | < 500ms | Tantivy query on 90-day index (~50K frames) |
| App startup time | < 3s | Cold start to recording active |
| Installer size | < 150MB | Bundled with ONNX model |

---

## 9. Testing Strategy

**Unit tests (Rust):**
- Capture: mock frame generation, encoding pipeline, segment rotation
- OCR: known-image → expected-text assertion suite (20+ test images)
- Storage: CRUD operations, retention cleanup, concurrent write safety
- Search: indexing, BM25 ranking, phrase match, filter combinations
- Privacy: PII regex coverage, exclusion list matching, encryption round-trip

**Integration tests:**
- Full pipeline: capture → OCR → store → search (headless, synthetic screen)
- Multi-monitor capture on macOS + Windows
- Idle detection and adaptive frame rate switching
- Retention policy enforcement under disk pressure

**Manual QA checklist:**
- 8-hour soak test on macOS and Windows (CPU, RAM, disk growth)
- Search accuracy across 1000+ frames with varied content
- App exclusion verification (banking, password managers)
- Pause/resume via hotkey mid-workflow
- Data deletion and segment purge verification
- Fresh install → first search in < 5 minutes

---

## 10. Sprint Plan (12 Weeks)

| Sprint | Weeks | Deliverable |
|---|---|---|
| S1 | 1–2 | Tauri scaffold + screen capture on macOS (CGDisplayStream) + raw frame output |
| S2 | 3–4 | H.265 encoding pipeline + segment rotation + metadata extraction (window title, app ID) |
| S3 | 5–6 | OCR pipeline (PaddleOCR via ONNX) + SQLite storage + frame deduplication |
| S4 | 7–8 | Tantivy search index + React search UI + timeline view |
| S5 | 9–10 | Privacy layer (encryption, exclusions, PII detection, audit log) + settings UI |
| S6 | 11–12 | Windows port (DXGI) + performance tuning + soak testing + installer packaging |

**Post-Phase 1 (immediate follow-on):**
- Audio capture + Whisper transcription (Phase 1.5)
- Browser URL extraction via accessibility APIs
- Activity summary dashboard with daily/weekly charts
- Export: selected time range → shareable video clip with OCR overlay

---

## 11. Open Technical Decisions

| Decision | Options | Recommendation | Rationale |
|---|---|---|---|
| OCR engine | Tesseract vs PaddleOCR vs Apple Vision | PaddleOCR (ONNX) | Best accuracy/speed ratio cross-platform; Apple Vision as macOS fast-path |
| Video codec | H.265 vs H.264 vs VP9 | H.265 | 40% smaller than H.264 at same quality; wide hardware encoder support |
| Search engine | Tantivy vs SQLite FTS5 vs MeiliSearch | Tantivy | Best BM25 implementation in Rust; incremental indexing; low memory |
| App framework | Tauri vs Electron vs native | Tauri | 10x smaller bundle than Electron; Rust backend eliminates bridge overhead |
| Keychain | OS-native vs custom | OS-native (Keychain/DPAPI) | Users trust OS-level key storage; no custom crypto key management |
| Fork screenpipe? | Fork vs build from scratch | Build from scratch | Screenpipe is GPL-3.0 (viral licence); our commercial model needs MIT/Apache or proprietary |

---

## 12. Risk Register (Phase 1)

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| macOS screen capture permissions rejected by users | Medium | High | Clear onboarding flow explaining why; progressive permission requests |
| OCR accuracy too low on complex UIs (dark mode, custom fonts) | Medium | Medium | Pre-process with contrast normalisation; fallback to Apple Vision on macOS |
| H.265 hardware encoder unavailable on older machines | Low | Medium | Graceful fallback to H.264 or software encoding with reduced fps |
| Storage growth exceeds user expectations | Medium | Low | Aggressive defaults (50GB cap, 90-day retention) + clear storage dashboard |
| Antivirus false positives on screen capture binary | Medium | High | Code signing + Microsoft SmartScreen submission + user documentation |
| Tauri 2.x breaking changes during development | Low | Medium | Pin Tauri version; evaluate at sprint boundaries only |

---

*End of Phase 1 Build Specification*
