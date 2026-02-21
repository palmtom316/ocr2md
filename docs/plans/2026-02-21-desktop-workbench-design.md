# Ocr2Md Desktop Workbench Design

**Date:** 2026-02-21  
**Status:** Approved  
**Target Platforms:** macOS, Windows

## 1. Goal

Build a colorblind-friendly, modern, high-tech desktop workbench for Ocr2Md that:

- supports PDF / DOC / DOCX (including dual-layer PDF),
- uses GLM-OCR (and GLM file parsing for Word) for extraction,
- routes structuring to OpenAI / Claude / Gemini / OpenAI-compatible relays (including cc-switch),
- provides batch operations, history, configuration management, and robust error handling.

## 2. User-Confirmed Product Decisions

- Application type: **Desktop app**
- Scope: **Full workbench** (batch queue + API config management + task history)
- Secret storage: **Local encrypted config file** (not keychain)
- Primary interaction: **Dual-pane workbench**

## 3. Approach Options and Trade-offs

### Option A (Recommended): Tauri v2 + Rust Core + Web UI

- Pros:
  - Native desktop packaging for macOS/Windows.
  - Reuses current Rust pipeline logic with minimal rewrite.
  - High UI flexibility for modern/tech visual language.
- Cons:
  - Dual-stack development (Rust + frontend).

### Option B: All-Rust Native UI (egui)

- Pros:
  - Single language stack.
  - Fast backend/UI integration.
- Cons:
  - Harder to achieve polished cross-platform visual parity with macOS/Windows conventions.

### Option C: Flutter + Rust FFI

- Pros:
  - Rapid UI composition and consistent rendering.
- Cons:
  - Highest complexity and integration overhead.

**Decision:** Option A.

## 4. Architecture

### 4.1 Layered Design

1. **UI Layer (Tauri frontend)**  
   Dual-pane workbench UI, task controls, previews, settings, accessibility controls.

2. **Orchestrator Layer (Rust service in Tauri backend)**  
   Task lifecycle/state machine:
   - `Queued`
   - `Running(OCR|LLM|Export)`
   - `Retrying`
   - `Success`
   - `Failed`

3. **Core Pipeline Layer (existing Rust logic, refactored into reusable modules/crate)**  
   File kind detection, GLM extraction, provider-specific LLM structuring, markdown output.

4. **Secure Config Layer**  
   Encrypted local config for provider profiles and secrets.

### 4.2 Runtime Model

- Single desktop app process.
- Long-running jobs run in async worker(s) in Rust backend.
- Progress/logs emitted to UI through Tauri events.

### 4.3 Cross-Platform UX Alignment

- macOS conventions: command-key shortcuts, native file dialogs, mac-style window semantics.
- Windows conventions: control-key shortcuts, familiar titlebar/menu behavior.
- Same interaction model; platform-specific shortcut mapping.

## 5. Information Architecture and Layout

### 5.1 Global Layout

- **Top toolbar:** new task, import batch, start/pause, settings, search.
- **Left pane (~34%):** task queue and file operations.
- **Right pane (~66%):** markdown workspace and diagnostics.
- **Bottom status bar:** provider/model/concurrency/failures/remaining.

### 5.2 Left Pane Modules

- Task filters: all / running / failed / completed.
- Queue list items: filename, file type, state, progress, duration.
- Batch actions: retry failed, export all, clear completed.
- Task details drawer: model/provider/retries/trace id/error summary.

### 5.3 Right Pane Tabs

- Markdown (editor + preview)
- OCR source text
- Diff (OCR vs structured output)
- Logs (phase-filtered)
- Metadata (hash, pages, tokens, timings, endpoint)

### 5.4 Settings Center

- API profiles: GLM, OpenAI, Claude, Gemini, OpenAI-compatible/cc-switch.
- Security: encryption password, auto-lock timeout, unlock session.
- Task policy: concurrency, timeout, retries, output naming.
- Accessibility: colorblind mode, contrast boost, type scale, motion reduction.

## 6. Visual Language

### 6.1 Colorblind-Friendly Theme Rules

- Avoid red/green-only semantics.
- Status must use **color + icon + text**.
- Base palette:
  - primary: cyan/blue family,
  - warning: amber,
  - failure: magenta/deep orange.
- Maintain readable contrast (AA-level targets).

### 6.2 Style Direction

- Modern minimal with subtle technical atmosphere:
  - neutral background,
  - low-noise texture/grid hints,
  - soft elevation and border system.
- Default light theme, optional dark theme.

### 6.3 Typography and Components

- Headings: geometric sans.
- Body: highly legible sans.
- Logs/code: monospace.
- Strong focus ring and keyboard navigation states.
- Controlled motion (120–180ms), with reduced-motion option.

## 7. Data Flow and Reliability

1. User imports files -> queue creation.
2. Pre-check and classification.
3. OCR/parse stage (GLM).
4. Structuring stage (selected LLM provider).
5. Markdown preview/edit/export.
6. Persist task history and metadata.

Reliability controls:

- Request timeout.
- Retry with exponential backoff for `429/5xx/network`.
- Retriable task execution at queue level.
- Trace ID through pipeline and logs.

## 8. Error Handling

- **User errors:** unsupported file type, missing keys, invalid paths.
- **Network errors:** timeout/connectivity/rate limit.
- **Service errors:** invalid response shape, unavailable model, quota limits.

Presentation strategy:

- concise toast summary,
- full context in task details,
- copyable diagnostic report with trace ID.

## 9. Security Model (Local Encrypted Config File)

- Encrypted storage for:
  - API keys,
  - base URLs,
  - model defaults,
  - provider profiles.
- Master password-derived key (KDF + AEAD).
- Session unlock with auto-lock timeout.
- Encrypted backup export/import path for migration.

## 10. Testing and Acceptance

### 10.1 Tests

- Rust unit tests:
  - file type detection,
  - provider routing/parsers,
  - retry/backoff behavior,
  - state transitions.
- Integration tests:
  - queue lifecycle,
  - mocked provider failures and retry recovery.
- UI tests:
  - key flows (import, run, retry, export),
  - accessibility interaction checks.

### 10.2 Acceptance Criteria

- End-to-end conversion works on macOS and Windows.
- Batch operations and history available.
- Colorblind mode and keyboard-first navigation available.
- Secrets never logged in plaintext.
- Error traces are diagnosable via UI and logs.

## 11. Out of Scope (Phase 1)

- Cloud sync account system.
- Team collaboration/multi-user permissions.
- Local Office-to-PDF conversion pipeline for forcing Word through OCR path.

