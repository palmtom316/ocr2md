# Ocr2Md Desktop Workbench Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Deliver a production-ready desktop workbench (macOS + Windows) with dual-pane UX, batch queue, encrypted local profile storage, and provider routing (OpenAI / Claude / Gemini / OpenAI-compatible including cc-switch).

**Architecture:** Use Tauri v2 (`apps/desktop/src-tauri`) as desktop shell, React + TypeScript (`apps/desktop/ui`) as UI layer, and refactor current Rust CLI logic into reusable core crate (`crates/ocr2md-core`). Desktop backend orchestrates queue/state/events and calls core crate.

**Tech Stack:** Rust (tokio, reqwest, serde), Tauri v2, React + TypeScript + Vite, Vitest + Playwright, cargo tests.

---

### Task 1: Refactor Rust Core Into Reusable Workspace Crate

**Files:**
- Create: `crates/ocr2md-core/Cargo.toml`
- Create: `crates/ocr2md-core/src/lib.rs`
- Create: `crates/ocr2md-core/tests/pipeline_smoke_test.rs`
- Modify: `Cargo.toml`
- Modify: `rust/main.rs`
- Modify: `rust/cli.rs`
- Modify: `rust/config.rs`
- Modify: `rust/error.rs`
- Modify: `rust/file_kind.rs`
- Modify: `rust/http.rs`
- Modify: `rust/llm.rs`
- Modify: `rust/ocr.rs`

**Step 1: Write the failing test**

```rust
// crates/ocr2md-core/tests/pipeline_smoke_test.rs
use std::path::Path;
use ocr2md_core::file_kind::{detect_input_kind, InputKind};

#[test]
fn detects_pdf_kind() {
    let kind = detect_input_kind(Path::new("demo.pdf")).unwrap();
    assert_eq!(kind, InputKind::Pdf);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p ocr2md-core --test pipeline_smoke_test -v`  
Expected: FAIL with crate/module resolution errors (`ocr2md_core` not found).

**Step 3: Write minimal implementation**

- Create workspace root in `Cargo.toml`:

```toml
[workspace]
members = ["crates/ocr2md-core"]
resolver = "2"
```

- Move reusable modules into `crates/ocr2md-core/src/` and export via `lib.rs`:

```rust
pub mod config;
pub mod error;
pub mod file_kind;
pub mod http;
pub mod llm;
pub mod ocr;
```

- Keep `rust/main.rs` as thin CLI wrapper consuming `ocr2md_core::*`.

**Step 4: Run test to verify it passes**

Run: `cargo test -p ocr2md-core --test pipeline_smoke_test -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add Cargo.toml crates/ocr2md-core rust
git commit -m "refactor: extract reusable rust core crate"
```

### Task 2: Add Encrypted Local Config Storage

**Files:**
- Create: `crates/ocr2md-core/src/secure_config.rs`
- Create: `crates/ocr2md-core/tests/secure_config_test.rs`
- Modify: `crates/ocr2md-core/Cargo.toml`
- Modify: `crates/ocr2md-core/src/lib.rs`

**Step 1: Write the failing test**

```rust
// crates/ocr2md-core/tests/secure_config_test.rs
use ocr2md_core::secure_config::{encrypt_blob, decrypt_blob};

#[test]
fn encrypt_decrypt_roundtrip() {
    let plain = br#"{"profiles":[{"name":"openai","api_key":"secret"}]}"#;
    let cipher = encrypt_blob(plain, "passphrase").unwrap();
    let back = decrypt_blob(&cipher, "passphrase").unwrap();
    assert_eq!(back, plain);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p ocr2md-core --test secure_config_test -v`  
Expected: FAIL (missing module/functions).

**Step 3: Write minimal implementation**

- Add dependencies: `argon2`, `chacha20poly1305`, `rand`.
- Implement `encrypt_blob` / `decrypt_blob` with:
  - Argon2id key derivation
  - random salt + nonce
  - AEAD encryption
  - versioned envelope format.

**Step 4: Run test to verify it passes**

Run: `cargo test -p ocr2md-core --test secure_config_test -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/ocr2md-core/src/secure_config.rs crates/ocr2md-core/tests/secure_config_test.rs crates/ocr2md-core/Cargo.toml crates/ocr2md-core/src/lib.rs
git commit -m "feat(core): add encrypted local config storage"
```

### Task 3: Implement Provider Profile Repository

**Files:**
- Create: `crates/ocr2md-core/src/profile_store.rs`
- Create: `crates/ocr2md-core/tests/profile_store_test.rs`
- Modify: `crates/ocr2md-core/src/lib.rs`

**Step 1: Write the failing test**

```rust
use ocr2md_core::profile_store::{ProfileStore, ProviderProfile};

#[test]
fn save_and_load_profiles() {
    let dir = tempfile::tempdir().unwrap();
    let store = ProfileStore::new(dir.path().join("config.enc"));
    let p = ProviderProfile::openai("work", "https://api.openai.com/v1", "k1", "gpt-4o-mini");
    store.save_all("pass", &[p]).unwrap();
    let loaded = store.load_all("pass").unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].name, "work");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p ocr2md-core --test profile_store_test -v`  
Expected: FAIL (missing repository implementation).

**Step 3: Write minimal implementation**

- Define `ProviderProfile` (provider, base_url, api_key, model, enabled).
- Implement `ProfileStore::save_all/load_all` using `secure_config` encryption.
- Ensure no plaintext API key is logged.

**Step 4: Run test to verify it passes**

Run: `cargo test -p ocr2md-core --test profile_store_test -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/ocr2md-core/src/profile_store.rs crates/ocr2md-core/tests/profile_store_test.rs crates/ocr2md-core/src/lib.rs
git commit -m "feat(core): add encrypted provider profile repository"
```

### Task 4: Add Queue State Machine and Orchestrator

**Files:**
- Create: `crates/ocr2md-core/src/queue.rs`
- Create: `crates/ocr2md-core/tests/queue_state_test.rs`
- Modify: `crates/ocr2md-core/src/lib.rs`

**Step 1: Write the failing test**

```rust
use ocr2md_core::queue::{JobState, Queue};

#[test]
fn job_state_transitions_to_success() {
    let mut q = Queue::default();
    let id = q.enqueue("demo.pdf");
    q.mark_running(id, "ocr");
    q.mark_running(id, "llm");
    q.mark_success(id);
    assert_eq!(q.get(id).unwrap().state, JobState::Success);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p ocr2md-core --test queue_state_test -v`  
Expected: FAIL.

**Step 3: Write minimal implementation**

- Define `JobState` and `JobRecord`.
- Implement `Queue` operations:
  - `enqueue`
  - `mark_running`
  - `mark_retrying`
  - `mark_failed`
  - `mark_success`

**Step 4: Run test to verify it passes**

Run: `cargo test -p ocr2md-core --test queue_state_test -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/ocr2md-core/src/queue.rs crates/ocr2md-core/tests/queue_state_test.rs crates/ocr2md-core/src/lib.rs
git commit -m "feat(core): add queue state machine"
```

### Task 5: Scaffold Tauri Desktop Backend

**Files:**
- Create: `apps/desktop/src-tauri/Cargo.toml`
- Create: `apps/desktop/src-tauri/src/main.rs`
- Create: `apps/desktop/src-tauri/src/commands.rs`
- Create: `apps/desktop/src-tauri/src/state.rs`
- Create: `apps/desktop/src-tauri/tauri.conf.json`
- Modify: `Cargo.toml`

**Step 1: Write the failing backend command test**

```rust
#[tokio::test]
async fn enqueue_command_returns_job_id() {
    // invoke command handler directly with test state
    // assert returned id is non-empty
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p ocr2md-desktop -v`  
Expected: FAIL (missing crate/commands).

**Step 3: Write minimal implementation**

- Add Tauri app crate and register commands:
  - `enqueue_files`
  - `start_queue`
  - `retry_job`
  - `load_profiles`
  - `save_profiles`
- Wire shared app state and queue orchestrator.

**Step 4: Run test to verify it passes**

Run: `cargo test -p ocr2md-desktop -v`  
Expected: PASS.

**Step 5: Commit**

```bash
git add apps/desktop/src-tauri Cargo.toml
git commit -m "feat(desktop): scaffold tauri backend and commands"
```

### Task 6: Scaffold Frontend App Shell (Dual-Pane)

**Files:**
- Create: `apps/desktop/ui/package.json`
- Create: `apps/desktop/ui/vite.config.ts`
- Create: `apps/desktop/ui/src/main.tsx`
- Create: `apps/desktop/ui/src/App.tsx`
- Create: `apps/desktop/ui/src/styles/tokens.css`
- Create: `apps/desktop/ui/src/components/LayoutShell.tsx`
- Create: `apps/desktop/ui/src/components/LayoutShell.test.tsx`

**Step 1: Write the failing component test**

```tsx
it('renders dual-pane shell with left queue and right workspace', () => {
  render(<LayoutShell />);
  expect(screen.getByTestId('queue-pane')).toBeInTheDocument();
  expect(screen.getByTestId('workspace-pane')).toBeInTheDocument();
});
```

**Step 2: Run test to verify it fails**

Run: `cd apps/desktop/ui && npm test -- LayoutShell.test.tsx`  
Expected: FAIL.

**Step 3: Write minimal implementation**

- Create shell layout and base design tokens:
  - colorblind-safe semantic colors
  - focus ring tokens
  - spacing/type scale.

**Step 4: Run test to verify it passes**

Run: `cd apps/desktop/ui && npm test -- LayoutShell.test.tsx`  
Expected: PASS.

**Step 5: Commit**

```bash
git add apps/desktop/ui
git commit -m "feat(ui): scaffold dual-pane shell and colorblind-safe tokens"
```

### Task 7: Implement Queue Panel + Batch Controls

**Files:**
- Create: `apps/desktop/ui/src/components/QueuePanel.tsx`
- Create: `apps/desktop/ui/src/components/QueueToolbar.tsx`
- Create: `apps/desktop/ui/src/components/QueuePanel.test.tsx`
- Modify: `apps/desktop/ui/src/App.tsx`

**Step 1: Write the failing test**

```tsx
it('filters failed jobs and triggers retry all failed', async () => {
  // seed jobs in store
  // click Failed filter
  // click Retry Failed
  // assert backend invoke called with failed ids
});
```

**Step 2: Run test to verify it fails**

Run: `cd apps/desktop/ui && npm test -- QueuePanel.test.tsx`  
Expected: FAIL.

**Step 3: Write minimal implementation**

- Render queue list with:
  - filters (all/running/failed/completed)
  - progress and status labels
  - retry/export/clear actions.

**Step 4: Run test to verify it passes**

Run: `cd apps/desktop/ui && npm test -- QueuePanel.test.tsx`  
Expected: PASS.

**Step 5: Commit**

```bash
git add apps/desktop/ui/src/components/QueuePanel.tsx apps/desktop/ui/src/components/QueueToolbar.tsx apps/desktop/ui/src/components/QueuePanel.test.tsx apps/desktop/ui/src/App.tsx
git commit -m "feat(ui): add queue panel filters and batch actions"
```

### Task 8: Implement Workspace Tabs (Markdown/OCR/Diff/Logs/Metadata)

**Files:**
- Create: `apps/desktop/ui/src/components/WorkspaceTabs.tsx`
- Create: `apps/desktop/ui/src/components/WorkspaceTabs.test.tsx`
- Create: `apps/desktop/ui/src/components/MarkdownEditor.tsx`
- Create: `apps/desktop/ui/src/components/LogView.tsx`
- Modify: `apps/desktop/ui/src/App.tsx`

**Step 1: Write the failing test**

```tsx
it('switches tabs and shows markdown + logs content', async () => {
  render(<WorkspaceTabs job={jobFixture} />);
  await user.click(screen.getByRole('tab', { name: /logs/i }));
  expect(screen.getByText(/llm_request/i)).toBeInTheDocument();
});
```

**Step 2: Run test to verify it fails**

Run: `cd apps/desktop/ui && npm test -- WorkspaceTabs.test.tsx`  
Expected: FAIL.

**Step 3: Write minimal implementation**

- Add right-pane tabs:
  - Markdown (editable preview)
  - OCR text
  - Diff
  - Logs
  - Metadata.

**Step 4: Run test to verify it passes**

Run: `cd apps/desktop/ui && npm test -- WorkspaceTabs.test.tsx`  
Expected: PASS.

**Step 5: Commit**

```bash
git add apps/desktop/ui/src/components/WorkspaceTabs.tsx apps/desktop/ui/src/components/WorkspaceTabs.test.tsx apps/desktop/ui/src/components/MarkdownEditor.tsx apps/desktop/ui/src/components/LogView.tsx apps/desktop/ui/src/App.tsx
git commit -m "feat(ui): add workspace tabs and markdown/log views"
```

### Task 9: Implement Settings Center and Encrypted Profile UX

**Files:**
- Create: `apps/desktop/ui/src/components/SettingsDialog.tsx`
- Create: `apps/desktop/ui/src/components/ProfileEditor.tsx`
- Create: `apps/desktop/ui/src/components/SettingsDialog.test.tsx`
- Modify: `apps/desktop/src-tauri/src/commands.rs`
- Modify: `apps/desktop/src-tauri/src/state.rs`

**Step 1: Write the failing test**

```tsx
it('prompts unlock password then loads profiles', async () => {
  render(<SettingsDialog />);
  await user.type(screen.getByLabelText(/master password/i), 'pass');
  await user.click(screen.getByRole('button', { name: /unlock/i }));
  expect(invoke).toHaveBeenCalledWith('load_profiles', { password: 'pass' });
});
```

**Step 2: Run test to verify it fails**

Run: `cd apps/desktop/ui && npm test -- SettingsDialog.test.tsx`  
Expected: FAIL.

**Step 3: Write minimal implementation**

- Add settings dialog with:
  - unlock/lock flow
  - profile list editor
  - provider-specific fields (api key/base url/model)
  - auto-lock timeout setting.

**Step 4: Run test to verify it passes**

Run: `cd apps/desktop/ui && npm test -- SettingsDialog.test.tsx`  
Expected: PASS.

**Step 5: Commit**

```bash
git add apps/desktop/ui/src/components/SettingsDialog.tsx apps/desktop/ui/src/components/ProfileEditor.tsx apps/desktop/ui/src/components/SettingsDialog.test.tsx apps/desktop/src-tauri/src/commands.rs apps/desktop/src-tauri/src/state.rs
git commit -m "feat(ui): add encrypted profile settings center"
```

### Task 10: Accessibility, Colorblind Mode, and End-to-End Validation

**Files:**
- Create: `apps/desktop/ui/src/styles/accessibility.css`
- Create: `apps/desktop/ui/src/components/AccessibilitySettings.tsx`
- Create: `apps/desktop/ui/e2e/workbench.spec.ts`
- Modify: `apps/desktop/ui/src/styles/tokens.css`
- Modify: `README.md`

**Step 1: Write the failing E2E test**

```ts
test('colorblind mode toggles semantic status labels and keyboard navigation works', async ({ page }) => {
  // open settings
  // enable colorblind mode
  // assert status includes icon+text, not color-only
  // tab through core actions
});
```

**Step 2: Run test to verify it fails**

Run: `cd apps/desktop/ui && npm run e2e -- workbench.spec.ts`  
Expected: FAIL.

**Step 3: Write minimal implementation**

- Add colorblind mode toggle and semantic status rendering.
- Add visible focus styles and keyboard traversal support.
- Update docs with desktop build/run/test commands.

**Step 4: Run test to verify it passes**

Run: `cd apps/desktop/ui && npm run e2e -- workbench.spec.ts`  
Expected: PASS.

**Step 5: Commit**

```bash
git add apps/desktop/ui/src/styles/accessibility.css apps/desktop/ui/src/components/AccessibilitySettings.tsx apps/desktop/ui/e2e/workbench.spec.ts apps/desktop/ui/src/styles/tokens.css README.md
git commit -m "feat(accessibility): add colorblind mode and keyboard-first UX"
```

### Final Verification Checklist (Before PR/Merge)

Run:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd apps/desktop/ui && npm run lint && npm test && npm run build
cd apps/desktop/ui && npm run e2e
```

Expected:

- all tests pass,
- no lint/clippy warnings,
- desktop build succeeds for macOS and Windows targets.

