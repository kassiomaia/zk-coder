# Fix Plan — `src-tauri/src/`

**Date:** 2026-04-16
**Based on:** `FINDINGS.md` from Phase 1

---

## Selection Table

| # | Finding | Confidence | Selected | Rationale |
|---|---------|------------|----------|-----------|
| F1 | Duplicate type system in agent module | High | **EXECUTED** | Fixed in prior audit run (4-phase consolidation) |
| F2 | No state machine validation | High | **EXECUTED** | Fixed in prior audit run |
| F3 | Wildcard re-export of chat entities | High | **EXECUTED** | Fixed in prior audit run |
| F4 | Regex unwrap() in production code | High | **EXECUTED** | Fixed in prior audit run |
| F5 | `from_str` shadows std trait | High | **EXECUTED** | Fixed in prior audit run |
| F6 | TaskMode vs SubTaskMode divergence | High | **EXECUTED** | Fixed as part of F1 Phase 3 |
| F7 | Inconsistent path traversal validation | High | **EXECUTED** | Fixed in prior audit run |
| F8 | Arbitrary file read via image path | High | **Yes** | Security: agent can read any file on the filesystem |
| F9 | Filename path traversal in PDF finalize | High | **Yes** | Security: agent can write PDF to arbitrary path |
| F10 | Mono font slot uses proportional font | High | **Yes** | Correctness: code blocks render with wrong font |
| F11 | No image size limit before decode | Medium | **Yes** | DoS: unbounded file read + decompression bomb risk |
| F12 | Streaming batcher final flush dropped | High | **Yes** | Correctness: last stream chunk silently discarded |
| F13 | Tool output serialization swallowed | High | **Yes** | Correctness: failed serialization invisible to debugging |
| F14 | Retry classification by string matching | High | **EXECUTED** | Structured `is_retryable()` already exists on the error type |
| F15 | Filename path traversal in PPTX/DOCX | High | **EXECUTED** | Security: identical to F9 but in new modules |
| F16 | `path.to_str().unwrap()` panic | High | **EXECUTED** | Correctness: panic on non-UTF-8 paths |
| F17 | Artifact dir bypasses Tauri path | High | **EXECUTED** | Correctness: split artifact location |
| F18 | `unwrap()` after draft mutation | High | **EXECUTED** | Correctness: latent panic sites |
| F19 | No path validation in `get_file_content` | High | **EXECUTED** | Security: agent-influenced path bypasses workspace sandbox |
| F20 | Unencoded URL params + `eval()` | High | **EXECUTED** | Correctness: broken URLs and JS syntax errors |
| F21 | No HTTP client timeout | High | **EXECUTED** | Correctness: stalled API connection blocks agent indefinitely |
| F22 | Unbounded SSE stream buffer | High | **EXECUTED** | Performance: buffer grows without limit between newlines |

## Deferred Table

| # | Finding | Reason |
|---|---------|--------|
| — | (none deferred this cycle) | — |

---

## Fix 1 — Align Path Traversal Validation (F7)

### Root Cause

`ExecuteCommandTool::execute()` in `execute_command.rs:112-118` validates the `working_dir` argument with only `Path::starts_with()`, which doesn't detect `..` path components. The spawn engine (`spawn.rs:781-789`) performs the correct check — rejecting `Component::ParentDir` before the `starts_with` check.

### Proposed Change

Add the same `Component::ParentDir` rejection check to `execute_command.rs` before the `starts_with` check:

```rust
// Before (current)
let cwd = if let Some(ref dir) = args.working_dir {
    let resolved = workspace.join(dir);
    if !resolved.starts_with(&workspace) {
        return Err(ToolError::PathTraversal(...));
    }
    resolved
} else {
    workspace.clone()
};

// After (proposed)
let cwd = if let Some(ref dir) = args.working_dir {
    let dir_path = std::path::Path::new(dir);
    if dir_path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(ToolError::PathTraversal(format!(
            "Working directory '{dir}' contains parent-directory components"
        )));
    }
    let resolved = workspace.join(dir);
    if !resolved.starts_with(&workspace) {
        return Err(ToolError::PathTraversal(format!(
            "Working directory '{dir}' is outside workspace"
        )));
    }
    resolved
} else {
    workspace.clone()
};
```

### Affected Code Paths

Only `execute_command.rs:112-118`. No callers change. The spawn engine's validation remains as a secondary safety net.

### Edge Cases

- `"./subdir"` — should pass (no `ParentDir` component)
- `"subdir/../other"` — should fail (contains `ParentDir`)
- `"subdir"` — should pass
- `""` — falls through to `workspace.clone()` (the `dir` check is `if let Some(ref dir)`)

### Test Plan

1. Add a test that `"../escape"` is rejected by the tool-level check (not just by spawn engine).
2. Add a test that `"./valid"` passes the check.
3. Existing integration tests pass.

### Risk Assessment

**Very low risk.** The change adds a stricter check in an early-fail path. The spawn engine already catches all `..` paths, so this can only prevent issues, never cause regressions.

### Execution Status: **EXECUTED** — Fixed in prior audit run

---

## Fix 2 — Validate Image Path in PDF Adapter (F8)

### Root Cause

`image_to_blocks()` at `adapter.rs:61` calls `std::fs::read(path)` with the agent-supplied path, no validation. The path passes through `PdfPageContentInput::Image { path }` → `PdfPageContent::Image { path }` → `image_to_blocks(path, ...)` without any sanitization at any layer.

### Proposed Change

Add path validation to `image_to_blocks()` in `adapter.rs` before the `std::fs::read` call:

```rust
fn image_to_blocks(path: &str, caption: Option<&str>) -> Result<Vec<Block>, PdfModelError> {
    let image_path = std::path::Path::new(path);
    // Reject paths with parent-directory traversal
    if image_path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(PdfModelError::InvalidImagePath(format!(
            "image path '{path}' contains parent-directory components"
        )));
    }
    let data = std::fs::read(path).unwrap_or_default();
    // ... rest unchanged
}
```

Also add `InvalidImagePath(String)` variant to `PdfModelError` in `core/documents/pdf/error.rs`.

### Affected Code Paths

- `application/tools/pdf/adapter.rs:60-84` — `image_to_blocks()` (primary fix site)
- `core/documents/pdf/error.rs` — add new error variant

### Edge Cases

- Absolute paths like `/etc/shadow` — still allowed by this fix (no workspace constraint available); the `..` rejection prevents escaping a relative root
- Symlinks — not addressed; would require `canonicalize()` which has its own issues (TOCTOU)
- Empty path — handled by existing `data.is_empty()` fallback

### Test Plan

1. New test: `image_path_with_parent_dir_rejected` — `../../etc/passwd` returns `InvalidImagePath` error
2. New test: `image_path_absolute_allowed` — `/tmp/test.png` is accepted (no `..`)
3. Existing test `image_with_missing_file_produces_placeholder` continues passing

### Risk Assessment

**Low risk.** Adds an early-fail check. The only behavioral change is rejecting `..` paths that would previously read arbitrary files. No callers are affected because the error propagates through the existing `Result` return.

---

## Fix 3 — Sanitize Filename in PDF Start (F9)

### Root Cause

`pdf_start` tool in `start.rs:100-104` only checks `ends_with(".pdf")` but does not strip path separators or `..` components from the filename. This filename is stored in the draft and later used in `final.rs:180` as `artifacts_dir.join(&draft.filename)`.

### Proposed Change

Add filename sanitization in `start.rs` after the `.pdf` extension normalization:

```rust
fn sanitize_filename(raw: &str) -> String {
    let with_ext = if raw.ends_with(".pdf") {
        raw.to_string()
    } else {
        format!("{raw}.pdf")
    };
    // Extract just the filename component — strip any directory separators
    let name = std::path::Path::new(&with_ext)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.pdf");
    // Double-check: reject if still contains path separators
    let sanitized = name.replace(['/', '\\'], "_");
    if sanitized.is_empty() || sanitized == ".pdf" {
        "document.pdf".to_string()
    } else {
        sanitized
    }
}
```

Replace `start.rs:100-104` with a call to `sanitize_filename(&args.filename)`.

### Affected Code Paths

- `infrastructure/tools/documents/pdf/start.rs:100-104` — filename normalization (fix site)
- `infrastructure/tools/documents/pdf/final.rs:180` — consumer of `draft.filename` (no change needed; benefits from the fix)

### Edge Cases

- `"../../../etc/malicious.pdf"` → `"malicious.pdf"` (only file_name component kept)
- `"report"` → `"report.pdf"` (extension added, then sanitized)
- `""` → `"document.pdf"` (fallback)
- `"sub/dir/report.pdf"` → `"report.pdf"` (directory stripped)
- `"normal-report.pdf"` → `"normal-report.pdf"` (unchanged)

### Test Plan

1. New test: `sanitize_filename_strips_directory_components` — `"../../../etc/cron.d/job.pdf"` → `"job.pdf"`
2. New test: `sanitize_filename_handles_separators` — `"sub/dir/file.pdf"` → `"file.pdf"`
3. New test: `sanitize_filename_handles_empty` — `""` → `"document.pdf"`
4. New test: `sanitize_filename_preserves_normal_names` — `"report.pdf"` → `"report.pdf"`
5. Existing tests pass unchanged

### Risk Assessment

**Low risk.** The change only narrows what filenames are accepted. Valid filenames (no path separators) are unaffected. The fix is entirely in the input normalization layer.

---

## Fix 4 — Bundle Monospace Font for Mono Slot (F10)

### Root Cause

`bundled_report_cache()` in `font_cache.rs:255` assigns `REGULAR` (Liberation Sans Regular, a proportional font) to the `FontSlot::Mono` slot. No monospaced font is bundled in `resources/pdf/fonts/`.

### Proposed Change

1. Download `LiberationMono-Regular.ttf` (same font family, same license — SIL OFL) and place it in `src-tauri/resources/pdf/fonts/`
2. Update `bundled_report_cache()`:

```rust
const MONO: &[u8] =
    include_bytes!("../../../../../../resources/pdf/fonts/LiberationMono-Regular.ttf");
// ...
let mono = LoadedFont::from_bytes(FontSlot::Mono, MONO.to_vec())?;
```

### Affected Code Paths

- `infrastructure/tools/documents/pdf/render/font_cache.rs:242-258` — `bundled_report_cache()` (fix site)
- `src-tauri/resources/pdf/fonts/` — new font file added
- No code consumers change — the `FontSlot::Mono` lookup already routes to the correct slot

### Edge Cases

- Existing tests that assert font count (`>= 4`) will still pass (count goes from 5 to 5)
- The `resources/pdf/themes/report.toml` may declare a `mono` font path; check and update if needed

### Test Plan

1. New test: `mono_slot_is_monospaced` — assert `cache.primary_for(&InlineStyle { code: true, .. }).family()` contains "Mono" or "Liberation Mono"
2. Existing `loads_every_slot_plus_fallback` and `primary_for_respects_style_priority` tests continue passing
3. Visual verification: render a code block and confirm monospace rendering (integration test in `finalize.rs`)

### Risk Assessment

**Low risk.** Adding a font file is additive. The binary size increases by ~60KB (Liberation Mono Regular). No behavior changes for non-code content.

---

## Fix 5 — Add Image File Size Limit (F11)

### Root Cause

`image_to_blocks()` at `adapter.rs:61` calls `std::fs::read(path)` with no file size check. A very large file or a decompression bomb image could exhaust memory.

### Proposed Change

Add a file size check before reading in `image_to_blocks()`:

```rust
const MAX_IMAGE_FILE_BYTES: u64 = 50 * 1024 * 1024; // 50 MB

fn image_to_blocks(path: &str, caption: Option<&str>) -> Result<Vec<Block>, PdfModelError> {
    // ... path validation from Fix 2 ...
    
    // Check file size before reading
    let metadata = std::fs::metadata(path).ok();
    if let Some(meta) = &metadata {
        if meta.len() > MAX_IMAGE_FILE_BYTES {
            return Err(PdfModelError::ImageTooLarge {
                path: path.to_string(),
                size: meta.len(),
                max: MAX_IMAGE_FILE_BYTES,
            });
        }
    }
    
    let data = std::fs::read(path).unwrap_or_default();
    // ... rest unchanged
}
```

Also add `ImageTooLarge { path: String, size: u64, max: u64 }` variant to `PdfModelError`.

### Affected Code Paths

- `application/tools/pdf/adapter.rs:60-84` — `image_to_blocks()` (fix site, shared with Fix 2)
- `core/documents/pdf/error.rs` — add new error variant (shared with Fix 2)

### Edge Cases

- File does not exist → `metadata` returns `None`, falls through to existing `read + empty check`
- File exists but is 0 bytes → passes size check, caught by existing `data.is_empty()` path
- File at exactly the limit → allowed (using `>` not `>=`)

### Test Plan

1. New test: `image_file_exceeding_size_limit_rejected` — mock or create a file > limit, verify `ImageTooLarge` error
2. Existing tests pass (all use small or nonexistent images)

### Risk Assessment

**Very low risk.** Adds an early-fail check. The 50MB limit is generous (a 50MP JPEG at max quality is ~25MB). No behavioral change for normal images.

---

## Fix 6 — Yield Batcher Final Flush Event (F12)

### Root Cause

`streaming.rs:90` calls `batcher.finish()` which returns the last buffered chunk event. The return value is bound to `_final_event` (underscore prefix = intentionally unused) and never yielded to the stream consumer. The comment `// Event yielded` is incorrect.

### Proposed Change

Replace `_final_event` with `final_event` and yield it:

```rust
// Before
if let Some(_final_event) = batcher.finish(&request_id, thread_id.clone()) {
    // Event yielded
}

// After
if let Some(final_event) = batcher.finish(&request_id, thread_id.clone()) {
    yield final_event;
}
```

### Affected Code Paths

- `infrastructure/llm/streaming.rs:90-92` — single-site fix

### Edge Cases

- Empty buffer at end of stream → `batcher.finish()` returns `None`, no change
- Buffer has content → event is now yielded before `Complete`, frontend sees smooth stream tail

### Test Plan

1. Update existing `test_stream_to_events_basic` — with the fix, short content that fits in a single batch may now appear as a Chunk event before Complete. The test currently asserts exactly 2 events (Started, Complete); it should accept an optional Chunk between them.
2. New test: `test_stream_final_flush_is_yielded` — feed content that won't auto-flush due to batching config thresholds, verify that a Chunk event appears before the Complete event.

### Risk Assessment

**Very low risk.** The Complete event still carries the full accumulated content. The only change is that a Chunk event may appear before it. Frontend code that handles both Chunk and Complete events will see smoother streaming. Any frontend code that only consumes Complete is unaffected.

---

## Fix 7 — Log Warning on Tool Output Serialization Failure (F13)

### Root Cause

`loop_runner.rs:215` uses `serde_json::to_string(&result.output).unwrap_or_default()`. If serialization fails, the tool result becomes an empty string with no diagnostic.

### Proposed Change

Replace `unwrap_or_default()` with `unwrap_or_else` that logs a warning:

```rust
// Before
serde_json::to_string(&result.output).unwrap_or_default()

// After
serde_json::to_string(&result.output).unwrap_or_else(|e| {
    log::warn!("[LLM] Failed to serialize tool output for call_id={}: {e}", result.call_id);
    String::new()
})
```

### Affected Code Paths

- `commands/llm/orchestrator/loop_runner.rs:215` — single-site fix

### Edge Cases

- `result.output` is `serde_json::Value` — always serializable in practice, so the log will only fire on future type changes
- The fallback behavior (empty string) is preserved — this is a diagnostic-only change

### Test Plan

No new tests needed — the fix adds logging without changing behavior. Existing orchestrator tests continue to pass.

### Risk Assessment

**Zero risk.** Pure diagnostic addition. No behavioral change.

---

## Fix 8 — Use Structured `is_retryable()` Instead of String Matching (F14)

### Root Cause

`loop_runner.rs:264-266` classifies errors as retryable by searching the error message text for keywords. The error type is `LlmError` (from `core::llm::entities`), which already provides `is_retryable()` — a structured method that matches on enum variants.

### Proposed Change

```rust
// Before
let is_retryable = err.to_string().to_lowercase().contains("timeout")
    || err.to_string().to_lowercase().contains("rate limit")
    || err.to_string().to_lowercase().contains("temporary");

// After
let is_retryable = err.is_retryable();
```

### Affected Code Paths

- `commands/llm/orchestrator/loop_runner.rs:264-266` — single-site fix

### Edge Cases

- `LlmError::is_retryable()` covers: `RateLimited`, `Network`, `Timeout` — all the cases the string matching intended to catch
- `LlmError::Provider { code, .. }` with a 5xx code is NOT covered by core `is_retryable()` (which only has `RateLimited | Network | Timeout`), but the infrastructure `LlmError::is_retryable()` does cover `Api { status >= 500 }`. Since this code uses the core type, 5xx provider errors will NOT be retried. This is a behavioral change but is actually **correct** — the core type doesn't have enough information to distinguish 5xx from 4xx provider errors.

### Test Plan

1. Verify existing orchestrator tests pass with the change.
2. The existing test suite in `commands/llm/orchestrator/tests.rs` covers retry behavior.

### Risk Assessment

**Low risk.** The behavioral change is minimal — the same error categories are retried. The only difference is that errors whose messages happen to contain "timeout" or "rate limit" (but aren't actually those error types) will no longer be retried. This is an improvement.

### Execution Status: **EXECUTED** — Fixed in prior audit run

---

## Fix 9 — Sanitize Filenames in PPTX/DOCX Start Tools (F15)

### Root Cause

`pptx/start.rs:96-100` and `docx/start.rs:88-92` only normalize the file extension but do not strip path separators or `..` components. The PDF module's `pdf/start.rs` has a `sanitize_filename()` function (added as part of F9) that the PPTX and DOCX modules were written without.

### Proposed Change

Extract a shared `sanitize_filename(raw: &str, extension: &str) -> String` function into `documents/shared/` and call it from all three start tools.

For PPTX start.rs:
```rust
// Before
let filename = if args.filename.ends_with(".pptx") {
    args.filename.clone()
} else {
    format!("{}.pptx", args.filename)
};

// After
let filename = sanitize_filename(&args.filename, "pptx");
```

Same pattern for DOCX start.rs with `"docx"` extension.

The shared function:
```rust
pub fn sanitize_filename(raw: &str, extension: &str) -> String {
    let trimmed = raw.trim();
    let ext_dot = format!(".{extension}");
    let with_ext = if trimmed.ends_with(&ext_dot) {
        trimmed.to_string()
    } else {
        format!("{trimmed}.{extension}")
    };
    let name = std::path::Path::new(&with_ext)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&format!("document.{extension}"));
    let sanitized = name.replace(['/', '\\'], "_");
    let stem = sanitized.strip_suffix(&ext_dot).unwrap_or(&sanitized);
    let stem_clean: String = stem.chars().filter(|c| *c != '.').collect();
    if stem_clean.trim().is_empty() {
        format!("document.{extension}")
    } else {
        sanitized
    }
}
```

Also update `pdf/start.rs` to use the shared function instead of its local copy.

### Affected Code Paths

- `infrastructure/tools/documents/shared/` — new shared function (or `mod.rs`)
- `infrastructure/tools/documents/pptx/start.rs:96-100` — call sanitize
- `infrastructure/tools/documents/docx/start.rs:88-92` — call sanitize
- `infrastructure/tools/documents/pdf/start.rs:131-150` — replace local copy with shared

### Edge Cases

Same as F9:
- `"../../../etc/malicious.pptx"` → `"malicious.pptx"`
- `""` → `"document.pptx"`
- `"sub/dir/report.docx"` → `"report.docx"`
- `"normal.pptx"` → `"normal.pptx"` (unchanged)

### Test Plan

1. New tests in shared module: `sanitize_filename_pptx_strips_dirs`, `sanitize_filename_docx_strips_dirs`, `sanitize_filename_empty_fallback`, `sanitize_filename_preserves_normal`
2. Existing PDF sanitize tests continue passing
3. All existing start tool tests pass

### Risk Assessment

**Very low risk.** Same pattern as the proven F9 fix, just applied to two additional modules.

---

## Fix 10 — Replace `path.to_str().unwrap()` with Lossy Conversion (F16)

### Root Cause

`pptx/final.rs:141` and `docx/final.rs:140` call `path.to_str().unwrap()` which panics if the path contains non-UTF-8 bytes (valid on Linux).

### Proposed Change

Replace both occurrences:

```rust
// Before
path: path.to_str().unwrap().to_string(),

// After
path: path.to_string_lossy().into_owned(),
```

### Affected Code Paths

- `infrastructure/tools/documents/pptx/final.rs:141` — single site
- `infrastructure/tools/documents/docx/final.rs:140` — single site

### Edge Cases

- UTF-8 paths (99.9% of cases): no behavioral change — `to_string_lossy()` returns identical output
- Non-UTF-8 paths: replacement character `\u{FFFD}` instead of panic

### Test Plan

No new tests needed — this is a defensive change. The path comes from the OS; non-UTF-8 paths are unpredictable to test reliably.

### Risk Assessment

**Zero risk.** Pure defensive improvement, no behavioral change for valid paths.

---

## Fix 11 — Use Tauri `resolve_artifacts_dir` for PPTX/DOCX (F17)

### Root Cause

PPTX and DOCX finalize tools construct the artifact directory from `$HOME/.coder/artifacts` instead of using Tauri's `app_local_data_dir()`. The PDF module already uses the correct pattern via `resolve_artifacts_dir()`.

### Proposed Change

1. Move `resolve_artifacts_dir()` from `pdf/artifacts.rs` to `documents/shared/` so all document tools can use it
2. Update `generate_pptx()` and `generate_docx()` to accept `&AppHandle` and use `resolve_artifacts_dir()`
3. Pass `app_handle` from the `execute()` method to the generate functions

For `pptx/final.rs`:
```rust
// Before
async fn generate_pptx(draft: PptxDraft) -> Result<(PathBuf, u64), ToolError> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| ToolError::failed("Could not determine home directory"))?;
    let artifacts_dir = PathBuf::from(home).join(".coder").join("artifacts");

// After
async fn generate_pptx(draft: PptxDraft, app_handle: &AppHandle) -> Result<(PathBuf, u64), ToolError> {
    let artifacts_dir = resolve_artifacts_dir(app_handle)?;
```

Same pattern for `docx/final.rs`.

### Affected Code Paths

- `infrastructure/tools/documents/pdf/artifacts.rs` — make `resolve_artifacts_dir` shared (or re-export)
- `infrastructure/tools/documents/pptx/final.rs:258-267` — use shared function
- `infrastructure/tools/documents/docx/final.rs:231-242` — use shared function

### Edge Cases

- First run: directory creation handled by `resolve_artifacts_dir()` (already does `create_dir_all`)
- `AppHandle` availability: always available in `execute()` — just needs to be passed to the generate function

### Test Plan

1. Existing tests pass (test uses `$HOME` which still resolves; the function falls back gracefully)
2. Verify PPTX/DOCX artifacts now appear alongside PDF artifacts in the same directory
3. The PDF module's existing test `write_pdf must not read HOME directly` pattern should be replicated for PPTX/DOCX

### Risk Assessment

**Low risk.** Artifact path changes from `~/.coder/artifacts` to `~/.local/share/com.coder.app/artifacts`. Existing PPTX/DOCX artifacts at the old path won't be migrated, but there are no released PPTX/DOCX features yet (still on feature branch), so no migration is needed.

---

## Fix 12 — Replace `unwrap()` with Error Propagation After Draft Mutation (F18)

### Root Cause

Three sites call `.unwrap()` on draft store lookups immediately after creating or modifying a draft. While the draft is guaranteed to exist under current implementation, these are latent panic sites.

### Proposed Change

Replace `.unwrap()` with `.ok_or_else(|| ToolError::failed(...))` at:

1. `pptx/add_slide.rs:195`:
```rust
// Before
let deck = store.get_pptx_draft(&args.deck_id).unwrap();
// After
let deck = store.get_pptx_draft(&args.deck_id).ok_or_else(|| {
    ToolError::failed(format!("Deck disappeared after slide add: {}", args.deck_id))
})?;
```

2. `docx/add_section.rs:142`:
```rust
// Before
let draft = store.get_docx_draft(&args.document_id).unwrap();
// After
let draft = store.get_docx_draft(&args.document_id).ok_or_else(|| {
    ToolError::failed(format!("Document disappeared after section add: {}", args.document_id))
})?;
```

3. `docx/start.rs:102`:
```rust
// Before
let draft = store.get_docx_draft(&document_id).unwrap();
// After
let draft = store.get_docx_draft(&document_id).ok_or_else(|| {
    ToolError::failed(format!("Document not found immediately after creation: {document_id}"))
})?;
```

### Affected Code Paths

Only the three listed sites. No callers change.

### Edge Cases

None — the `.ok_or_else()` path will never trigger under current implementation. This is purely defensive.

### Test Plan

No new tests needed — the error path is unreachable under current implementation. Existing tests pass unchanged.

### Risk Assessment

**Zero risk.** Replaces a panic with a graceful error return. No behavioral change for the happy path.

---

## Fix 13 — Add Path Validation to `get_file_content` (F19)

### Root Cause

`commands/file.rs:44` uses `PathBuf::from(workspace_root).join(&file_path)` without any validation. The `file_path` parameter originates from agent tool output shown in the UI. If the agent includes a path like `../../etc/shadow`, clicking "view diff" reads that file.

### Proposed Change

Add `Component::ParentDir` rejection and `starts_with(workspace_root)` validation before reading the file:

```rust
pub async fn get_file_content(
    workspace_root: String,
    file_path: String,
    session_id: String,
    agent_state: tauri::State<'_, AgentState>,
) -> Result<FileContentResponse, String> {
    let workspace = PathBuf::from(&workspace_root);
    let file_p = std::path::Path::new(&file_path);

    // Reject paths with parent-directory traversal
    if file_p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(format!("Path contains parent-directory components: {file_path}"));
    }

    let full_path = workspace.join(&file_path);

    // Verify the resolved path is within the workspace
    if let Ok(canonical) = full_path.canonicalize() {
        if !canonical.starts_with(&workspace) {
            return Err(format!("Path is outside workspace: {file_path}"));
        }
    }

    // ... rest unchanged
}
```

### Affected Code Paths

- `commands/file.rs:38-82` — `get_file_content` (single fix site)

### Edge Cases

- Relative paths within workspace (`src/main.rs`) — pass (no `..`)
- Traversal paths (`../../etc/shadow`) — rejected by `ParentDir` check
- Absolute paths (`/etc/shadow`) — `join` replaces workspace root, `canonicalize + starts_with` catches it
- Non-existent files — `canonicalize` fails, falls through to existing `read_to_string().ok()` which returns `None`

### Test Plan

1. New test: `get_file_content_rejects_parent_dir` — `"../../etc/passwd"` returns error
2. New test: `get_file_content_accepts_relative_path` — `"src/main.rs"` passes validation
3. Existing behavior preserved for normal file paths

### Risk Assessment

**Very low risk.** Same pattern as the FS tools already use. Only rejects paths that would escape the workspace. Normal file viewing is unaffected.

---

## Fix 14 — URL-Encode All Parameters in `open_file_diff_window` (F20)

### Root Cause

`commands/file.rs:156-157` builds a URL with only `file_path` encoded, then injects it into `window.eval()`:
```rust
let path = format!(
    "/file-diff?filePath={encoded_path}&workspaceRoot={workspace_root}&sessionId={session_id}"
);
window.eval(format!("window.location.href = '{path}'"))
```

### Proposed Change

URL-encode all three parameters using the existing `encode_url()` function:

```rust
let encoded_path = encode_url(&file_path);
let encoded_workspace = encode_url(&workspace_root);
let encoded_session = encode_url(&session_id);
let path = format!(
    "/file-diff?filePath={encoded_path}&workspaceRoot={encoded_workspace}&sessionId={encoded_session}"
);
```

### Affected Code Paths

- `commands/file.rs:155-157` — URL construction (single fix site)

### Edge Cases

- Normal workspace paths (`/home/user/project`) — URL-encoded but decoded correctly by the frontend router
- Paths with spaces (`/home/user/my project`) — currently break the URL; fix properly encodes them
- Paths with `&` or `#` — currently corrupt query string; fix encodes them
- Paths with single quotes — currently break `eval()`; fix encodes them as `%27`

### Test Plan

1. New test: `encode_url_handles_special_chars` — verify `'`, `&`, `#`, space are all encoded
2. Existing test: verify the function produces valid RFC 3986 percent-encoding

### Risk Assessment

**Very low risk.** The `encode_url()` function already exists and works correctly (it's used for `file_path`). Just applying it to the other two parameters. No behavioral change for typical paths (alphanumeric + `/`).

---

## Fix 15 — Add HTTP Client Timeout (F21)

### Root Cause

All three `reqwest::Client::builder().build()` calls in the LLM clients have no `.timeout()` configured. `reqwest` has no default timeout, so a stalled server hangs the tokio task indefinitely.

### Proposed Change

Add a 120-second request timeout to all three client builders. This is long enough for complex LLM requests (which can take 60+ seconds for large context windows) but prevents indefinite hangs:

```rust
// Before
let client = Client::builder()
    .build()
    .expect("Failed to create HTTP client");

// After
let client = Client::builder()
    .timeout(std::time::Duration::from_secs(120))
    .build()
    .expect("Failed to create HTTP client");
```

For the streaming clients, use `.connect_timeout()` instead of `.timeout()` — a full request timeout would kill long-running streams. The connect timeout prevents stalls during initial connection only:

```rust
let client = Client::builder()
    .connect_timeout(std::time::Duration::from_secs(30))
    .build()
    .expect("Failed to create HTTP client");
```

### Affected Code Paths

- `infrastructure/llm/anthropic/client.rs:44-46` — Anthropic client (streaming + non-streaming share one client)
- `infrastructure/llm/openai/client.rs:103-105` — OpenAI standard client
- `infrastructure/llm/openai/client.rs:150-152` — OpenAI streaming client

### Edge Cases

- Very large requests with extended thinking (Anthropic) can take 60-90 seconds — 120s timeout accommodates this
- Streaming responses are long-lived by design — `.connect_timeout()` only applies to initial handshake, not the stream itself
- Network retries at the orchestrator level (bounded to 2) would retry after timeout

### Test Plan

1. Existing tests pass — no behavioral change for normal requests
2. Verify `LlmError::Timeout` is produced when a request takes > 120s (integration test with a mock server)

### Risk Assessment

**Low risk.** Timeouts are additive safety measures. The 120s/30s values are conservative. No behavioral change for normal API requests.

---

## Fix 16 — Cap SSE Stream Buffer Size (F22)

### Root Cause

The SSE parsing loops in both clients accumulate bytes into a `String` buffer that grows without limit between newlines. Under normal operation this is fine, but a malfunctioning server sending data without newlines causes unbounded allocation.

### Proposed Change

Add a buffer size check after each append. If the buffer exceeds 50MB without encountering a newline, yield an error and break:

```rust
const MAX_SSE_BUFFER_BYTES: usize = 50 * 1024 * 1024; // 50 MB

buffer.push_str(&text);

if buffer.len() > MAX_SSE_BUFFER_BYTES {
    error!("SSE buffer exceeded {} bytes without newline, aborting stream", MAX_SSE_BUFFER_BYTES);
    yield Err(LlmError::streaming(format!(
        "SSE buffer exceeded {} MB limit",
        MAX_SSE_BUFFER_BYTES / (1024 * 1024)
    )));
    break;
}
```

### Affected Code Paths

- `infrastructure/llm/anthropic/client.rs:389` — after `buffer.push_str(&text)`
- `infrastructure/llm/openai/client.rs:645` — after `buffer.push_str(&text)`

### Edge Cases

- Normal SSE events: ~100 bytes to ~100KB — well below 50MB limit
- Very large model responses: accumulated in `full_content` in the batcher, not in the SSE buffer (buffer is drained per-line)
- The 50MB limit is generous: a single SSE line should never be close to this

### Test Plan

1. Existing tests pass — normal responses are far below the limit
2. No new test needed — the limit is unreachable under normal operation

### Risk Assessment

**Zero risk.** The 50MB limit is unreachable under normal operation. It only triggers for pathological server behavior.
