# WorkShadow AI — Master Task Plan

**Version:** 1.0
**Date:** March 2026
**Purpose:** Complete, actionable task breakdown to build WorkShadow AI from zero to product-market fit.

---

## Overview

WorkShadow AI is an observe-learn-execute productivity platform. It installs as a lightweight desktop agent, continuously captures screen activity, extracts structured data via OCR, and progressively evolves from a searchable memory tool into an autonomous workflow execution engine.

**Product Tiers:**
| Tier | Capability | Phase |
|------|-----------|-------|
| Observer | Record + OCR + search + knowledge base | Phase 1 |
| Co-Pilot | Workflow extraction + proactive suggestions | Phase 2 |
| Autopilot | Autonomous task execution + analytics + compliance | Phase 3 |

---

## Phase 1 — Observer MVP (Weeks 1–12)

**Goal:** Ship a desktop agent that captures screen activity, extracts text via OCR, and provides local searchable timeline. Zero cloud dependency.

**Success Criteria:**
- Installs in < 2 minutes on macOS and Windows
- < 5% CPU, < 500MB RAM in background
- 8h workday captured in < 3GB storage
- Search results in < 2 seconds
- Zero data leaves the device

---

### Sprint 1 (Weeks 1–2): Project Scaffold + Screen Capture

- [ ] **T1.1** Initialize Tauri 2.x project with Rust backend + React/TypeScript frontend
- [ ] **T1.2** Set up Cargo workspace structure (`capture/`, `ocr/`, `storage/`, `search/`, `privacy/`, `ipc/`)
- [ ] **T1.3** Set up Vite + React 18 + TypeScript + TailwindCSS frontend scaffold
- [ ] **T1.4** Configure GitHub Actions CI/CD — build matrix for macOS (arm64, x86_64) + Windows (x86_64)
- [ ] **T1.5** Implement macOS screen capture via `CGDisplayStream` / ScreenCaptureKit (macOS 13+)
- [ ] **T1.6** Implement `CaptureConfig` struct (fps, max_resolution, segment_duration, idle_threshold)
- [ ] **T1.7** Implement `FrameMetadata` struct (timestamp, window_title, app_bundle_id, browser_url, display_index)
- [ ] **T1.8** Build dedicated capture thread with bounded channel for backpressure
- [ ] **T1.9** Implement active window title + app bundle ID extraction (macOS)
- [ ] **T1.10** Write raw frame output to disk for validation
- [ ] **T1.11** Write unit tests for capture module (mock frame generation)
- [ ] **T1.12** Create system tray icon with recording indicator (red dot when active)

### Sprint 2 (Weeks 3–4): Video Encoding + Metadata Pipeline

- [ ] **T2.1** Integrate `ffmpeg-next` crate for H.265 (HEVC) encoding
- [ ] **T2.2** Implement hardware encoder detection at startup (VideoToolbox on macOS, NVENC/QSV/AMF on Windows)
- [ ] **T2.3** Implement software fallback to libx265 when no hardware encoder found
- [ ] **T2.4** Build segment rotation: 5-minute `.mkv` chunks + `.jsonl` metadata sidecar files
- [ ] **T2.5** Implement resolution downscaling to 1920x1080 max
- [ ] **T2.6** Implement adaptive frame rate: drop to 0.5 fps on idle (no window change for 30s)
- [ ] **T2.7** Implement browser URL extraction (accessibility APIs on macOS)
- [ ] **T2.8** Write integration tests for encoding pipeline + segment rotation
- [ ] **T2.9** Benchmark: verify < 5MB/s sustained disk I/O, ~350MB/hour storage

### Sprint 3 (Weeks 5–6): OCR Pipeline + Storage

- [ ] **T3.1** Integrate PaddleOCR-Lite via ONNX Runtime (`ort` crate) with bundled model files
- [ ] **T3.2** Build async OCR pipeline on separate thread pool (2 threads)
- [ ] **T3.3** Implement frame preprocessing: downscale to 1280x720, contrast normalization for dark mode
- [ ] **T3.4** Implement `OcrResult` and `TextBlock` structs with bounding boxes and confidence scores
- [ ] **T3.5** Implement text deduplication: Jaccard similarity > 0.9 skips insert
- [ ] **T3.6** Set up SQLite database (WAL mode) with `sessions`, `frames`, `retention_policy` tables
- [ ] **T3.7** Create indexes: `idx_frames_timestamp`, `idx_frames_app`, `idx_frames_window`
- [ ] **T3.8** Implement `rusqlite` CRUD operations for frames and sessions
- [ ] **T3.9** Implement PII detection via regex (email, phone, SSN, credit card) — flag but don't delete
- [ ] **T3.10** Wire up pipeline: capture → OCR → SQLite write
- [ ] **T3.11** Write OCR accuracy tests (20+ test images with known expected text)
- [ ] **T3.12** Write storage unit tests (CRUD, concurrent write safety)

### Sprint 4 (Weeks 7–8): Search Engine + UI

- [ ] **T4.1** Integrate Tantivy full-text search engine
- [ ] **T4.2** Configure indexed fields: `ocr_text`, `window_title`, `app_id`, `browser_url`
- [ ] **T4.3** Implement incremental index updates as new OCR results arrive
- [ ] **T4.4** Support BM25 ranking and phrase queries
- [ ] **T4.5** Implement Tauri IPC commands: `search`, `getTimelineRange`, `getFrame`, `getFrameImage`
- [ ] **T4.6** Implement IPC commands: `startCapture`, `pauseCapture`, `getCaptureStatus`
- [ ] **T4.7** Build typed TypeScript IPC wrappers (`lib/ipc.ts`)
- [ ] **T4.8** Build **Timeline View**: horizontal scrollable timeline, thumbnails grouped by hour, color-coded by app
- [ ] **T4.9** Build thumbnail click → full-size frame with OCR overlay + metadata panel
- [ ] **T4.10** Build **Search View**: search bar, timestamped result cards (thumbnail + text snippet + app icon)
- [ ] **T4.11** Build search filters: date range, application, URL domain
- [ ] **T4.12** Build sort toggle: relevance (BM25) vs chronological
- [ ] **T4.13** Write search tests: indexing, ranking, phrase match, filter combinations
- [ ] **T4.14** Benchmark: verify < 500ms p95 search latency on 50K frames

### Sprint 5 (Weeks 9–10): Privacy + Settings

- [ ] **T5.1** Implement AES-256-GCM encryption at rest for stored data
- [ ] **T5.2** Integrate `keyring` crate for OS keychain access (macOS Keychain / Windows DPAPI)
- [ ] **T5.3** Implement app exclusion list (never capture specified apps/URLs)
- [ ] **T5.4** Pre-configure default exclusions: 1Password, Bitwarden, LastPass, KeePass, banking patterns, Signal, WhatsApp
- [ ] **T5.5** Implement global hotkey pause/resume (Cmd+Shift+P / Ctrl+Shift+P) via `global-hotkey` crate
- [ ] **T5.6** Implement local append-only audit log (capture start/stop/delete events)
- [ ] **T5.7** Implement storage retention policy: configurable max days (default 90) + max GB (default 50)
- [ ] **T5.8** Build hourly cleanup job: delete oldest segments when limits exceeded
- [ ] **T5.9** Implement user data deletion: delete any time range from UI, purge segments from disk
- [ ] **T5.10** Build **Settings View**: capture config, storage config, privacy exclusions, retention policy
- [ ] **T5.11** Implement TOML config file loader (`~/.workshadow/config.toml`)
- [ ] **T5.12** Implement IPC commands: `getSettings`, `updateSettings`, `getStorageUsage`
- [ ] **T5.13** Write privacy tests: encryption round-trip, exclusion matching, PII regex coverage

### Sprint 6 (Weeks 11–12): Windows Port + Polish + Ship

- [ ] **T6.1** Implement Windows screen capture via DXGI Desktop Duplication API
- [ ] **T6.2** Implement Windows active window title + app ID extraction
- [ ] **T6.3** Implement Windows hardware encoder detection (NVENC, QSV, AMF)
- [ ] **T6.4** Run full integration test suite on Windows
- [ ] **T6.5** Performance tuning: verify CPU < 3% idle / < 8% peak, RAM < 400MB after 4h
- [ ] **T6.6** Run 8-hour soak test on both macOS and Windows
- [ ] **T6.7** Package macOS installer (.dmg) via Tauri bundler
- [ ] **T6.8** Package Windows installer (.msi) via Tauri bundler
- [ ] **T6.9** Verify installer size < 150MB (with bundled ONNX model)
- [ ] **T6.10** Verify cold start to recording < 3 seconds
- [ ] **T6.11** Set up Apple Developer ID code signing
- [ ] **T6.12** Set up Windows Authenticode code signing + SmartScreen submission
- [ ] **T6.13** Build onboarding flow: permission requests with clear explanations
- [ ] **T6.14** Manual QA: full checklist (search accuracy, exclusions, pause/resume, data deletion, fresh install)
- [ ] **T6.15** Tag v0.1.0 release

---

## Phase 1.5 — Observer Enhancements (Weeks 13–18)

**Goal:** Extend capture to audio, improve browser integration, and add activity analytics.

- [ ] **T7.1** Add audio capture from system audio + microphone
- [ ] **T7.2** Integrate Whisper (ONNX) for local audio transcription
- [ ] **T7.3** Store transcriptions in SQLite, index in Tantivy alongside OCR text
- [ ] **T7.4** Implement browser URL extraction via accessibility APIs (both platforms)
- [ ] **T7.5** Build **Activity Summary View**: daily breakdown (hours per app, top URLs, frequent window titles)
- [ ] **T7.6** Add daily/weekly bar charts using Chart.js
- [ ] **T7.7** Implement IPC command: `getDailySummary`
- [ ] **T7.8** Build export feature: selected time range → shareable video clip with OCR overlay
- [ ] **T7.9** Add Linux support via PipeWire screen capture portal
- [ ] **T7.10** Multi-monitor capture support on all platforms

---

## Phase 2 — Co-Pilot (Weeks 19–36)

**Goal:** Extract structured workflow patterns from recorded sessions and provide proactive suggestions. This is the transition from passive memory to active intelligence.

### 2A: Workflow Extraction Engine

- [ ] **T8.1** Design workflow graph data model (nodes = UI actions, edges = transitions, metadata = app context)
- [ ] **T8.2** Integrate vision transformer (ViT) model for UI element recognition (buttons, fields, menus)
- [ ] **T8.3** Build action classifier: categorize captured frames into action types (click, type, scroll, navigate, copy-paste)
- [ ] **T8.4** Implement sequence detection: identify repetitive action patterns across sessions
- [ ] **T8.5** Build workflow graph builder: convert detected sequences into executable workflow graphs
- [ ] **T8.6** Implement confidence scoring for detected patterns (frequency, consistency, recency)
- [ ] **T8.7** Store workflow graphs in database with versioning
- [ ] **T8.8** Build workflow review UI: visualize detected patterns, let users confirm/reject/edit

### 2B: Proactive Suggestion Engine

- [ ] **T9.1** Implement real-time context matching: compare current activity against known workflow patterns
- [ ] **T9.2** Build suggestion notification system: non-intrusive toast/overlay when a known pattern is detected
- [ ] **T9.3** Implement "next step" prediction: suggest the next action in a recognized workflow
- [ ] **T9.4** Build form auto-fill suggestions: detect form fields and suggest values from historical data
- [ ] **T9.5** Implement copy-paste chain detection and suggestion
- [ ] **T9.6** Add user feedback loop: accept/dismiss suggestions to improve model accuracy
- [ ] **T9.7** Build suggestion analytics: track acceptance rate, time saved per suggestion

### 2C: Knowledge Base + Natural Language Query

- [ ] **T10.1** Integrate vector database (Qdrant, local mode) for semantic search over captured sessions
- [ ] **T10.2** Build embedding pipeline: generate embeddings from OCR text + window context
- [ ] **T10.3** Implement RAG pipeline for natural-language queries ("How did Sarah process a refund?")
- [ ] **T10.4** Build query UI with conversational interface
- [ ] **T10.5** Return timestamped video walkthroughs with annotated steps as query results
- [ ] **T10.6** Implement team knowledge sharing (opt-in, within org boundary)

---

## Phase 3 — Autopilot (Weeks 37–54)

**Goal:** Full autonomous workflow execution with human-in-the-loop controls, enterprise compliance, and analytics.

### 3A: Autonomous Execution Engine

- [ ] **T11.1** Integrate browser automation (Playwright) for web-based workflow replay
- [ ] **T11.2** Integrate native OS automation APIs for desktop app interaction
- [ ] **T11.3** Build execution planner: convert workflow graphs into step-by-step execution plans
- [ ] **T11.4** Implement confidence-gated execution: auto-execute above threshold, prompt user below
- [ ] **T11.5** Build human-in-the-loop approval UI: preview each action, approve/reject/modify before execution
- [ ] **T11.6** Implement execution monitoring: real-time progress tracking, error detection, rollback capability
- [ ] **T11.7** Build exception handling: detect deviations from expected state, pause and alert user
- [ ] **T11.8** Implement execution audit trail: full log of every automated action with before/after screenshots

### 3B: Enterprise Compliance + Deployment

- [ ] **T12.1** Build admin dashboard: manage users, policies, recording scope, data retention
- [ ] **T12.2** Implement role-based access control (RBAC)
- [ ] **T12.3** Build employee consent workflow: opt-in recording with clear disclosure
- [ ] **T12.4** Implement data governance dashboard: transparent view of what's captured, stored, and processed
- [ ] **T12.5** Build PII redaction pipeline (beyond regex): NER-based entity detection for names, addresses, IDs
- [ ] **T12.6** Prepare SOC 2 Type II compliance documentation
- [ ] **T12.7** Implement GDPR compliance features: data export, right to deletion, processing records
- [ ] **T12.8** Implement EU AI Act compliance: transparency, human oversight, risk classification documentation
- [ ] **T12.9** Build private cloud deployment option (Docker/Kubernetes) for enterprise customers
- [ ] **T12.10** Implement centralized configuration management for fleet deployments

### 3C: Analytics + Reporting

- [ ] **T13.1** Build productivity analytics: time per task, automation rate, time saved
- [ ] **T13.2** Build team-level dashboards: aggregate workflow patterns, identify optimization opportunities
- [ ] **T13.3** Implement ROI reporting: hours automated, cost savings, error reduction
- [ ] **T13.4** Build anomaly detection: flag unusual activity patterns for compliance review

---

## Cross-Cutting Tasks (Ongoing)

### Developer Experience
- [ ] **TX.1** Write CONTRIBUTING.md with dev setup instructions
- [ ] **TX.2** Set up pre-commit hooks (formatting, linting, tests)
- [ ] **TX.3** Set up automated release pipeline (tag → build → sign → publish)
- [ ] **TX.4** Create developer documentation for IPC API

### Quality Assurance
- [ ] **TX.5** Set up code coverage tracking (target: > 80% for Rust backend)
- [ ] **TX.6** Create automated regression test suite
- [ ] **TX.7** Set up performance benchmarking CI (catch regressions)
- [ ] **TX.8** Create manual QA test plan template for each sprint

### Security
- [ ] **TX.9** Conduct threat model analysis for screen capture + storage
- [ ] **TX.10** Set up dependency vulnerability scanning (cargo-audit, npm audit)
- [ ] **TX.11** Implement secure update mechanism (Tauri updater, disabled by default)
- [ ] **TX.12** Penetration testing before enterprise launch

### Business / Go-to-Market
- [ ] **TX.13** Recruit 5 design partners for Phase 1 beta (HR ops, customer service, back-office)
- [ ] **TX.14** Build product landing page
- [ ] **TX.15** Implement usage telemetry (opt-in only) for product analytics
- [ ] **TX.16** Set up customer feedback collection pipeline
- [ ] **TX.17** Prepare pre-seed pitch materials with working demo
- [ ] **TX.18** Implement SaaS billing integration for tiered pricing ($29 / $79 / $149 per seat/month)

---

## Technical Decision Log

| Decision | Chosen Option | Rationale |
|----------|--------------|-----------|
| App framework | Tauri 2.x | 10x smaller than Electron; Rust backend eliminates bridge overhead |
| OCR engine | PaddleOCR (ONNX) | Best accuracy/speed cross-platform; Apple Vision as macOS fast-path |
| Video codec | H.265 (HEVC) | 40% smaller than H.264 at same quality; wide HW encoder support |
| Search engine | Tantivy | Best BM25 in Rust; incremental indexing; low memory |
| Keychain | OS-native | Users trust OS-level key storage; no custom crypto key management |
| Codebase | Build from scratch | Screenpipe is GPL-3.0 (viral); commercial model needs MIT/Apache or proprietary |
| Vector DB (Phase 2) | Qdrant (local) | Runs on-device; no cloud dependency; Rust client available |
| Automation (Phase 3) | Playwright + native APIs | Broad browser coverage; native APIs for desktop apps |

---

## Risk Register

| Risk | Phase | Likelihood | Impact | Mitigation |
|------|-------|-----------|--------|------------|
| macOS screen permission rejection | 1 | Medium | High | Clear onboarding flow; progressive permission requests |
| OCR accuracy on dark mode / custom fonts | 1 | Medium | Medium | Contrast normalization; Apple Vision fallback on macOS |
| HW encoder unavailable on older machines | 1 | Low | Medium | Graceful fallback to H.264 or SW encoding at reduced fps |
| Storage growth exceeds expectations | 1 | Medium | Low | Aggressive defaults (50GB/90d); storage dashboard |
| Antivirus false positives | 1 | Medium | High | Code signing + SmartScreen + documentation |
| Tauri 2.x breaking changes | 1 | Low | Medium | Pin version; evaluate at sprint boundaries |
| Workflow extraction accuracy too low | 2 | Medium | High | Start with simple patterns; human-in-the-loop validation |
| Enterprise IT security resistance | 3 | High | High | SOC 2; transparent governance dashboard; opt-in consent |
| Microsoft Recall / UiPath convergence | 2–3 | Medium | Medium | First-mover in observe-to-automate; deep vertical specialization |
| Compute cost for on-device inference | 2–3 | Medium | Medium | HW acceleration; adaptive processing; private cloud offload |

---

*This document is the single source of truth for all WorkShadow AI tasks. Update as decisions are made and milestones are reached.*
