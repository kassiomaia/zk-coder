# Frontend Performance During Agent Runs — Execution Plan

**Status**: IN PROGRESS (A + B.1 + C.1 + C.3 shipped 2026-04-19; B.2 / B.3 / C.2 deferred; D deferred — see rationale below)
**Created**: 2026-04-19
**Driver**: UI becomes unresponsive / noticeably janky while an agent is running. Investigation attributes a per-step main-thread cost of ~380–650 ms (≈160–200 backend events × cascading Svelte updates) to a small number of concrete hotspots.

## Progress

- Workstream **A — Lazy-load the FileTree**: DELIVERED. Backend `list_workspace_dir` replaces `list_workspace_files`; the old eager 10 K walk is gone. `FileTree.svelte` is now a `dirCache` state machine; touched-file auto-expand (debounced 200 ms) respects user collapses. Per-dir cap 1 000, per-folder inline truncation message. 15 backend tests + 22 FileTree tests + 5 sessionActions tests + 5 AgentPanelFilesSection tests green; 41 pre-existing agentStore tests restored (needed `vi.hoisted`).
- Workstream **B.1 — Coarsen streaming chunks**: DELIVERED. `STREAMING_CHUNK_BYTES` 96 → 384 (4× fewer streaming events per LLM response). `STREAMING_CHUNK_DELAY_MS` removed entirely — the frontend's 16 ms `STREAM_THROTTLE_MS` already paces visible updates, so the additional 24 ms-per-chunk backend delay was pure latency. Round-trip + perf-guard tests added.
- Workstream **B.2 / B.3 — `HarnessStateUpdated` debounce + payload shrink**: DEFERRED. Call-site audit showed `emit_harness_state_updated` is invoked at lifecycle transitions (bootstrap, park, resume, terminal) — **not per step**. Per-run cost is <100 ms total. Debouncing would require threading a new `Arc`'d debouncer through `ExecutionDeps` and managing per-session timer lifecycles — disproportionate to the win. Revisit if tracing shows this path warming up in practice.
- Workstream **C.1 — Isolate `statusPhrases` from the sessions store**: DELIVERED. New `src-ui/src/lib/stores/agent/statusStore.ts` (`SvelteMap<sessionId, string[]>`) holds phrases side-channel. `handleStatusPhrase` no longer mutates `state.sessions`; the store listener in `store.ts::initialize` short-circuits `status_phrase` events entirely (no `set()`, no map clone, no subscriber cascade). `statusPhrases` field removed from `AgentSession`; `threadAgentState.ts` reads from the status store directly; `SvelteMap` reactivity means `$derived` blocks rerun only when the accessed session's phrases change. 41 agentStore + 10 threadAgentState + 8 streamingEvents tests still green.
- Workstream **C.3 — Collapse overlapping `AnsweringIndicator` scrambles**: DELIVERED. A new scramble now cancels any in-flight RAF loop (previously bursts of StatusPhrase events stacked overlapping loops that fought for frame budget). Honours `prefers-reduced-motion`: when set, the label snaps to target instead of animating. `animationFrame` tracked explicitly (0 = none) so on-completion cleanup is deterministic.
- Workstream **C.2 — Backend-side status phrase coalescing**: DEFERRED. With C.1 + C.3 shipped, each StatusPhrase now costs a single SvelteMap write + one scramble cancellation + one chat-store callback — well under 1 ms. Backend-side coalescing would drop event count further but adds an async debouncer to the tool-call hot path for a small remaining win. Revisit if tracing shows StatusPhrase IPC still dominates a step.
- Workstream **D — Fine-grained store updates**: DEFERRED, needs dedicated planning. Original premise was that `new Map(state.sessions)` allocation at `handler.ts:134` was a significant per-step cost. Reinvestigation during this session shows:
  - The Map allocation itself is ~10 µs/event — negligible cumulatively.
  - The real cost was the **subscriber cascade** (downstream `$derived` blocks re-running across 17+ files that import `agentStore`).
  - C.1's side-channel short-circuit already removes the worst offender (`status_phrase`, 25+/step). Remaining per-step events (`file_touched` 20–50, `streaming_content` ~25 post-B.1, `tool_call`/`tool_result` 3–5) each trigger one cascade per event, costing maybe 30–80 ms/step combined.
  - Moving each remaining high-frequency field to a side-channel store (the C.1 pattern) recovers that cost, but each one touches backend hydration (the server returns `filesTouched`, `toolCalls`, `executionSteps` inside the session payload on `loadSession`) and 4–6 call sites + tests. At ~30 min of risk per field for ~15 ms of saved time, the ROI is weak within this plan's budget.
  - The clean structural fix is migrating the store to Svelte 5 `$state` with nested reactivity (D.1 Option 2 in the original plan). That's a dedicated 8–16 h refactor touching `store.ts`, `derived.ts`, all 17 consumers, and ~10 test files. It deserves its own plan doc.
  - **Recommended next step**: if a trace confirms the residual 30–80 ms/step is user-visible, open a `docs/planning/agent-store-state-migration/plan.md` and tackle the `$state` migration as a single cohesive workstream. Do not fragment this into per-field side-channel moves.

## Goal

Cut per-step main-thread work on the frontend by **≥70%** and eliminate visible long tasks (>100 ms) during normal agent execution. Keep the scroll system, streaming semantics, and backend event contract observable so regressions are detectable.

## Non-goals

- No redesign of the agent event bus or session store shape.
- No change to streaming *semantics* (final content must still reconcile; chunk ordering preserved).
- No new virtualization libraries beyond what's already in `src-ui/` unless a phase explicitly calls for it.
- No backend behavioural changes to the agent itself (prompting, tool set, planner) — only the **emission cadence and payload** of events.

---

## Problem inventory

All file:line references verified against the current tree.

| ID | Problem | Evidence (file:line) | Cost/step | Fix workstream |
|----|---------|----------------------|-----------|----------------|
| P1 | Backend eagerly walks up to 10 K workspace entries at session bootstrap and hands the flat list to the UI; `FileTree` then rebuilds the full tree from that list on every `filesTouched` event (O(n log n) sort per subtree, ~20–50 rebuilds/step). Root cause: the tree shouldn't be built eagerly at all — collapsed folders should not be loaded. | `src-tauri/src/commands/workspace_files.rs:21,49` (eager walk, 10 K cap); `src-ui/src/lib/components/agent/FileTree.svelte:39` (`$derived(buildFileTree(...))`); `:54–68` full rebuild; `:67` recursive sort; `:97` linear `children.find()`. | 240–360 ms | **A** |
| P2 | Streaming chunks are too small and gated with a delay: 96-byte chunks + 24 ms per chunk → ~104 events per 10 KB response. | `src-tauri/src/application/agent/execution/termination.rs:19–20` (`STREAMING_CHUNK_BYTES=96`, `STREAMING_CHUNK_DELAY_MS=24`). | 50–100 ms | **B** |
| P3 | `StatusPhrase` emissions fire per-call (context build, planning, tool phases — ~25/step) and each triggers a full sessions-Map clone + thinking-indicator re-render. | `src-tauri/.../runtime/permissions.rs` (status emit sites); `src-ui/src/lib/stores/agent/events/streamingEvents.ts:112–145`. | 25–50 ms | **C** |
| P4 | Every event clones the sessions Map: `{ ...state, sessions: new Map(state.sessions) }` runs ~150+×/step. | `src-ui/src/lib/stores/agent/events/handler.ts:134`. | 15–25 ms | **D** |
| P5 | `AnsweringIndicator` scramble animation uses a RAF loop per status change; overlapping scrambles fight for frame budget. | `src-ui/src/lib/components/chat/AnsweringIndicator.svelte:62–176`. | 30–50 ms (visible jank) | **C** |
| P6 | `HarnessStateUpdated` fetches the full task + subtask tree from the repo on every state transition. | `src-tauri/src/application/agent/execution/harness_events.rs:10–42`. | 5–15 ms | **B** |
| P7 | `TaskUpdated` / `SubtaskUpdated` handlers do a full `tasks.map(...)` clone per subtask update. | `src-ui/src/lib/stores/agent/events/taskEvents.ts:59–128`. | 12–18 ms | **D** |
| P8 | `insertPath` uses linear `children.find()` per path segment → ~40K lookups per FileTree rebuild. | `src-ui/src/lib/components/agent/FileTree.svelte:97`. | 3–5 ms (multiplier on P1) | **A** |

Total observed per step: **~380–650 ms of main-thread work**, aligned with the reported freezes.

---

## Success metrics

Measured on a reference run ("MVP planning + 6 edits + 1 build" task on a 10K-entry workspace):

- P50 long-task duration during streaming: **< 50 ms** (baseline: 250–400 ms).
- Total scripting time per agent step: **≤ 150 ms** (baseline: ~500 ms).
- No frame > 100 ms during `StatusPhrase` bursts.
- FileTree rebuild cost: **≤ 15 ms** amortised (baseline: 240–360 ms).
- Streaming throughput (chars/second rendered): unchanged or faster.

Capture: Chrome DevTools Performance trace, one before/after per workstream, archived under `docs/planning/frontend-perf-agent-runs/traces/`.

---

## Workstream A — Lazy-load the FileTree

**Fixes**: P1, P8 (at the root — the 10K-entry build never happens).
**Ships alone**: yes. Largest single win.
**Risk**: low-to-medium — introduces a new IPC command and changes the component's data shape, but behaviour stays user-visible equivalent (VS Code-style tree).
**Blast radius**: new `list_workspace_dir` command, `FileTree.svelte` rewrite, session-action change, one new store slice, deprecation of `list_workspace_files`.

### Rationale

The current design (walk up to 10K entries at session start, hand the flat list to the UI, rebuild a full tree on every touched-file overlay update) is the root cause of the 240–360 ms/step stall. Memoising the build helps, but the build shouldn't exist at all. Collapsed folders don't need to be loaded. Lazy-loading on expand matches the VS Code / JetBrains model and scales past the 10K cap.

### A.1 — New backend command: `list_workspace_dir`

Add to `src-tauri/src/commands/workspace_files.rs`:

```rust
#[tauri::command]
pub async fn list_workspace_dir(
    root: String,
    path: String, // "" for root; otherwise a forward-slash relative path
) -> Result<WorkspaceDirResponse, String>
```

Behaviour:
- Reads **only immediate children** of `root / path` (no recursion).
- Returns `{ entries: Vec<WorkspaceEntry>, truncated: bool, total_seen: usize }` with a **per-directory cap of 1_000**. Rationale: this repo contains folders with 31 K (`src-tauri/target/debug/deps`) and 7 K entries (`src-ui/node_modules/lucide-svelte/dist/icons`). Any cap gets hit on those; the cap's real job is keeping the post-expand Svelte render <50 ms without virtualization. 1 000 keyed `{#each}` nodes render in ~25 ms; 2 000 starts stuttering. Raise later if users complain — lowering after adoption is harder.
- `total_seen`: if truncated, the count observed so far (cap + 1 is enough to signal; an exact count would require a full walk, which we are trying to avoid). UI shows "Showing first 1 000 of this folder" — omits the exact total, stays accurate.
- Rejects any `path` that escapes `root` (no `..`, no absolute paths). Validate with `Path::components()` exactly like the existing walker handles `.git`.
- Skips `.git` at the root level for parity with current UX.
- Sorts entries server-side: directories first, then files, both case-insensitive locale-aware. Moves the sort off the main thread and removes `sortTree` from the component.
- Register in `src-tauri/src/lib.rs:266`, **replacing** `list_workspace_files`. The old command has exactly one real caller (`src-ui/src/lib/stores/agent/actions/sessionActions.ts:353`) and is deleted in the same PR along with its tests — git is the rollback, a deprecation period adds complexity with no payoff.

### A.2 — Session bootstrap: load only the root

Change `src-ui/src/lib/stores/agent/actions/sessionActions.ts:353` to call `list_workspace_dir` with `path: ''`. Store the result as `session.workspaceRoot: { entries, truncated }` instead of `workspaceFiles: WorkspaceEntry[]`.

Remove `workspaceFiles` / `workspaceFilesTruncated` fields from `src-ui/src/lib/types/agent.ts:140-142`. Update `AgentPanel.svelte:207,211,213` to pass the new shape.

### A.3 — FileTree state: per-directory load cache

Replace the `FileTreeNode` recursive literal structure with a shallow-by-default store:

```ts
type DirState =
  | { status: 'unloaded' }
  | { status: 'loading' }
  | { status: 'loaded'; entries: WorkspaceEntry[]; truncated: boolean }
  | { status: 'error'; message: string };

// Keyed by directory path ('' for root).
let dirCache = $state<Map<string, DirState>>(new Map());
```

On expand:
1. If `dirCache.get(path)?.status === 'loaded'` → render immediately.
2. Else set `'loading'` and `invoke('list_workspace_dir', { root, path })`.
3. On resolve, set `'loaded'`. On reject, set `'error'` with a retryable message.

Collapsed folders stay `'unloaded'` — they cost nothing.

### A.4 — Touched-file overlay (the tricky bit)

Two orthogonal requirements:

1. **Overlay a touched file that is inside an already-loaded directory**: render the M/A/D badge via `touchedByPath.get(entry.path)` at the leaf. Pure lookup — zero tree work.
2. **Overlay a touched file inside a collapsed ancestor**: auto-expand every ancestor on the path, triggering loads. Matches VS Code: when a build touches a file, it reveals.

Auto-expand policy (avoid thrashing):

- On every new unique touched path (not already visible), add every ancestor `dir` of that path to `expandedPaths` and enqueue loads for dirs whose state is `'unloaded'`.
- Debounce the auto-expand reaction by 200 ms (`setTimeout` trailing) so a burst of 20–50 `FileTouched` events in one agent step triggers **one** batch of expansions.
- User-collapsed folders are respected: track a `userCollapsed: Set<string>` that auto-expand will not override. Re-expansion happens only if the user expands manually, or a new touch arrives after they collapsed it.

### A.5 — Cache invalidation on file-system changes

When a `FileTouched` event reports `action: 'write'` with `modified: false` (a newly created file) or `action: 'delete'`, invalidate the **parent directory's** cache entry so the next expand re-fetches. Writes to an already-existing file need no invalidation (membership unchanged).

Implement as a single handler in the FileTree component (or a small selector) subscribed to the `FileTouched` event. Does not touch the session store.

### A.6 — Delete the deep-walk code path (same PR)

Same-PR deletion, not a deprecation cycle:

- Remove `list_workspace_files`, its `#[tauri::command]` registration, `WorkspaceFilesResponse`, and `MAX_ENTRIES=10_000` from `src-tauri/src/commands/workspace_files.rs` and `src-tauri/src/lib.rs:266`.
- Remove `workspaceFiles` and `workspaceFilesTruncated` fields from `src-ui/src/lib/types/agent.ts:140-142`.
- Delete obsolete assertions in `src-ui/src/lib/stores/agent/actions/sessionActions.test.ts:31,41,59,60,98,105` and rewrite them against the new bootstrap.
- Rewrite `src-ui/src/lib/components/agent/FileTree.test.ts` around `dirCache` fixtures (injected via props) instead of `allFiles`.

Global truncation footer at `src-ui/src/lib/components/agent/FileTree.svelte:189-191` is removed. Replaced by a **per-directory** inline truncation line rendered inside a `'loaded'` directory when `truncated === true`: `"Showing first 1 000 of this folder"`. Accurate, discoverable, ~5 lines of template.

### A.7 — Tests / verification

- **Backend unit**: `list_workspace_dir` returns only immediate children; rejects `..`; handles missing dir; sorts correctly; applies per-dir cap.
- **Frontend unit**: `dirCache` state machine (unloaded → loading → loaded / error); auto-expand respects `userCollapsed`; cache invalidation on create/delete.
- **Integration**: `FileTree.test.ts` rewritten around the new shape. Cover: empty root, load-on-expand, touched file inside collapsed folder auto-expands, user collapse sticks.
- **Performance**: Chrome trace during an agent step touching 30 files across 10 dirs. Assert no long task > 50 ms attributable to FileTree. (Baseline: 240–360 ms.)
- **Scaling**: Manual test on a 100K-entry monorepo (currently truncated at 10K). Expand a deep path end to end; each expand should feel instant (<50 ms).

### A.8 — Resolved design decisions

1. **Search-across-all-files UX**: grepped codebase — no consumer depends on a flat search of `session.workspaceFiles`. Only the count display at `src-ui/src/lib/components/agent/AgentPanel.svelte:207` reads `.length`. Replace with a lightweight rolling count fed by `dirCache` (sum of loaded entries). Users who want "how many files total" get the *loaded* count; exact workspace total is no longer a thing (and arguably was never accurate past the 10 K cap anyway).
2. **Initial depth**: load only the root on bootstrap. Pre-loading top-level directories (`src/`, `src-ui/`, `src-tauri/`) is tempting but fragile — it assumes a layout that not every workspace has and adds waterfall complexity. First expand will load on demand; with a 1 000-entry cap and local-FS latency it's <30 ms. Revisit only if a perf trace shows the first expand is a user-visible pain.
3. **Persistence of `expandedPaths`**: component-local, ephemeral (lost on session unmount). Session-scoped persistence is out of scope; a later workstream can add it if asked. Touched-file auto-expand (A.4) already handles the agent-driven common case.

---

## Workstream B — Reduce event volume at the source

**Fixes**: P2, P6.
**Ships alone**: yes.
**Risk**: medium — changes the *cadence* observable by downstream consumers; must preserve final-chunk semantics.
**Blast radius**: `termination.rs`, `harness_events.rs`.

### B.1 — Coarsen streaming chunks

- `STREAMING_CHUNK_BYTES`: 96 → **384** (≈4× fewer events; still well under one TCP MSS and well under a paint frame).
- `STREAMING_CHUNK_DELAY_MS`: 24 → **0** for intra-chunk emission; rely on the **frontend** throttle (`STREAM_THROTTLE_MS=16`) for pacing.
  - Rationale: the 24 ms backend delay is additive to the frontend's 16 ms throttle — we pay both. Removing the backend delay does not starve the UI because the frontend already caps update rate.
- Keep `is_final` semantics identical. Regression-test that the chat reconciles to the exact final string.

### B.2 — Debounce `HarnessStateUpdated`

Add a 300 ms trailing debouncer in `harness_events.rs::emit_harness_state_updated`. Coalesce multiple state-churn transitions within the window into one fetch + one emit. The terminal state (Completed/Failed/StepBudgetExhausted) **must** flush immediately — add a `flush: bool` argument or a separate `emit_harness_state_updated_immediate` for the terminal path.

### B.3 — Shrink the `HarnessStateUpdated` payload

Audit the payload shape: does it include full subtask bodies (descriptions, contracts, framing)? Those rarely change mid-run. Emit only `{ session_id, tasks: [{ id, status, subtasks: [{ id, status, order }] }] }` and let the frontend keep bodies cached from the last `TaskUpdated`/`SubtaskUpdated`.

### B.4 — Tests / verification

- Rust unit test: debouncer collapses 10 state changes in 100 ms into 1 emission; terminal flush bypasses.
- Integration: run a full agent step and count `harness_state_updated` events on the frontend — expect ≤ 4/step (baseline: 15+).
- Streaming correctness: snapshot test that concatenation of all `streaming_content` chunks equals the persisted final message.

---

## Workstream C — Isolate status + thinking indicator

**Fixes**: P3, P5.
**Ships alone**: yes (independent of A/B/D).
**Risk**: low.
**Blast radius**: `streamingEvents.ts`, `AnsweringIndicator.svelte`, new `statusStore.ts`.

### C.1 — Move `statusPhrases` out of the session store

Currently every `StatusPhrase` event clones the sessions Map and cascades to **every** subscriber of the session store. Move the phrase buffer to a dedicated, scoped store keyed by `sessionId`:

- New file: `src-ui/src/lib/stores/agent/statusStore.ts` — `writable<Map<sessionId, string[]>>`.
- `handleStatusPhrase` writes only to this store; does not touch `state.sessions`.
- `AnsweringIndicator` and `ThinkingMessage` subscribe to the scoped store.

### C.2 — Backend: coalesce status phrases

In `permissions.rs` (and any other site emitting status), add a 200 ms trailing coalescer: only the latest phrase in the window is sent. This matches the way humans read them — intermediate phrases are overwritten visually anyway.

### C.3 — Collapse overlapping scrambles

`AnsweringIndicator.scrambleText` spawns a new RAF loop per status change. If a new status arrives while one is in flight:

- Cancel the in-flight RAF loop cleanly (track the `animationFrameId`, call `cancelAnimationFrame`).
- Skip scrambling entirely if `reduced-motion` is set (respect `prefers-reduced-motion`).
- Cap scramble to one concurrent instance per indicator.

### C.4 — Tests / verification

- Unit: status store receives N phrases, subscribers fire ≤ N times (not N × sessions).
- Manual: run a streaming-heavy step, watch `performance.measure('status-update')` — target < 2 ms per update.
- A11y: verify reduced-motion honoured.

---

## Workstream D — Fine-grained store updates

**Fixes**: P4, P7.
**Ships alone**: partially — depends on C for full benefit (C removes the largest source of Map clones).
**Risk**: medium — touches hot store code used by the entire agent UI.
**Blast radius**: `stores/agent/events/handler.ts`, `stores/agent/events/taskEvents.ts`, consumers.

### D.1 — Stop cloning the sessions Map on every event

Two options, pick **option 2** unless an audit finds external consumers relying on reference identity of `state.sessions`:

1. **Option 1** (minimally invasive): mutate `state.sessions` in place and publish a version counter (`state.sessionsVersion++`) that selectors subscribe to. Requires auditing subscribers.
2. **Option 2** (preferred): migrate the session store to Svelte 5 `$state` with nested reactivity. Replace `Map` with a plain object keyed by `sessionId`; reactivity becomes per-field. Remove the `{ ...state, sessions: new Map(...) }` return at `handler.ts:134`.

Deliverable: after the migration, a `StatusPhrase` event triggers re-render of the indicator only, not of every session-dependent component.

### D.2 — Update tasks/subtasks by reference, not by clone

Replace `tasks.map(t => t.id === target ? { ...t, subtasks: t.subtasks.map(...) } : t)` with a direct mutation on the reactive `$state` structure. Consumers that care about a specific subtask subscribe via `$derived(tasks.find(...).subtasks.find(...))`.

### D.3 — Selector library

Introduce `src-ui/src/lib/stores/agent/selectors/` (if not already present) and funnel components through narrow selectors:

- `selectSessionStatus(sessionId)`
- `selectStreamingContent(sessionId, messageId)`
- `selectTouchedFiles(sessionId)` (debounced — consumed by FileTree)
- `selectActiveStatusPhrase(sessionId)`

Components that need only a subset subscribe to the selector, not the whole session.

### D.4 — Tests / verification

- Regression: existing agent-UI tests pass.
- Svelte DevTools: visible re-render count on a `StatusPhrase` event drops from N (session subscribers) to 1 (indicator only).

---

## Sequencing

```
Phase 1 (week 1):  A  (FileTree — largest single win, lowest risk)
Phase 2 (week 1):  B  (backend event volume — independent)
Phase 3 (week 2):  C  (status store + indicator — unlocks D benefit)
Phase 4 (week 2):  D  (fine-grained reactivity — biggest refactor)
```

A and B can ship in parallel (different codebases, no shared files). C must precede D to keep the D refactor focused. Each workstream is independently mergeable and independently beneficial.

## Risk register

| Risk | Mitigation |
|------|------------|
| Backend chunk coarsening (B.1) breaks a test that asserts per-byte chunk boundaries. | Grep for `STREAMING_CHUNK_BYTES` usages before change; update any fixture. |
| Debouncing `HarnessStateUpdated` delays the visible terminal state. | `flush: true` on terminal transitions (Completed/Failed/StepBudgetExhausted/Cancelled). |
| D.1 Option 2 migration is larger than scoped. | Timebox to 2 days; if blocked, fall back to Option 1 (version counter). |
| FileTree memoisation keeps stale touched overlays after new agent step. | Overlay pass is cheap; re-run on every `touched` update but gated by A.3 debounce. |
| Reduced-motion users lose the scramble effect. | Intentional — C.3 explicitly honours `prefers-reduced-motion`. |

## Open questions

1. Is there an external consumer (e.g. telemetry) counting `streaming_content` events? If so, B.1's chunk-size change alters its metric.
2. Does `HarnessStateUpdated` already have a backend consumer? If yes, B.3's payload shrink needs a matching deserializer update.
3. Are any Svelte 5 `$state`-incompatible subscribers still using the `.subscribe()` pattern? D.1 Option 2 may need a compatibility shim.

## Deliverables

- PR per workstream, each with:
  - Before/after performance trace (`.json`) archived under `docs/planning/frontend-perf-agent-runs/traces/`.
  - One-paragraph perf-note in the PR description citing the relevant metric.
- Update this plan's **Status** to `IN PROGRESS` when A starts, to `COMPLETE` when D ships.
