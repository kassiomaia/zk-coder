# Frontend Tool Widgets — Enhancement Plan

**Status**: DRAFT
**Created**: 2026-04-19
**Driver**: The current tool call cards (`ToolCallMessage`) show generic outcomes ("Changes applied successfully", "Read N lines") with no inline visual content. Reference screenshots (`docs/screenshots/tools/*.png`) demonstrate a significantly richer pattern: grouped operations, inline unified diffs, compact file lists, and terminal output inline within the card body — all making the agent's activity readable at a glance without expanding raw JSON.

---

## Context

Tool call messages are the primary surface through which the user observes what the agent is doing. Today the pattern is:

- One `ToolCallMessage` card per tool invocation (no grouping)
- Expandable header → generic outcome text → raw JSON toggle
- No visual content: no diffs, no file lists, no terminal output inline
- Only `web_search` has a custom `BodyComponent`; all others use the fallback

Reference screenshots (Copilot-style, `docs/screenshots/tools/`):

| Screenshot | What it demonstrates |
|---|---|
| `view-files.png` | Compact path list: "View 4 files" collapsed to a clean row-per-file list with line ranges |
| `edit-files.png` | Inline unified diff per file: line number columns, red/green row coloring, per-file collapsible sections |
| `created-file.png` | New-file diff (all additions), "Show all N lines" truncation pill |
| `system-call.png` | Terminal output inline in the bash card body, truncation notice, exit code |
| `agent-indicator.png` | Minimal working indicator (contrast with our `AnsweringIndicator`; no action needed) |

HTML prototypes for the three body components have been produced and validated:

- `.prototypes/tool-widgets/bash-body.html`
- `.prototypes/tool-widgets/read-body.html`
- `.prototypes/tool-widgets/inline-diff.html`

---

## Problem inventory

All file:line references verified against current tree.

| ID | Problem | Evidence | Workstream |
|----|---------|----------|------------|
| P1 | `bashRenderer` has no `BodyComponent`. Terminal output flows through separate `TerminalOutput` messages (a different `message.kind`), not inside the tool card. The card body shows only "Command finished successfully". | `src-ui/src/lib/components/chat/messages/tool-renderers/bashRenderer.ts:1-30` (no BodyComponent); `src-ui/src/lib/components/chat/messages/TerminalOutput.svelte` (separate path) | **A** |
| P2 | `fsReadRenderer` has no `BodyComponent`. Expanding the card shows only a "Target" param row, then the raw JSON toggle. File content is not presented visually; there is no file path list. | `src-ui/src/lib/components/chat/messages/tool-renderers/fsRenderers.ts:17-42` | **B** |
| P3 | `fsWriteRenderer` has no `BodyComponent`. The unified diff for write operations exists in `TouchedFile.diff` (populated via `file_touched` agent events) but is not surfaced in `ToolCallMessage` — `toolResult` contains only a success/failure marker. | `src-ui/src/lib/components/chat/messages/tool-renderers/fsRenderers.ts:43+`; `src-ui/src/lib/types/agent.ts:403-409` (`TouchedFile.diff`); `src-ui/src/lib/types/agent.ts:574-581` (`file_touched` event) | **C** |
| P4 | No lightweight diff renderer exists. `DiffViewer.svelte` uses Monaco editor — one instance per tool card is too heavy (full editor per row, language server load, large bundle). | `src-ui/src/lib/components/agent/DiffViewer.svelte` (Monaco-based, used in ApprovalMessage) | **C** |
| P5 | Consecutive same-type tool calls are rendered as individual cards. The reference shows "Edit 4 files" as a single grouped card containing per-file sections. Grouping requires detecting adjacency across `messages[]` in `ChatMessagesList`. | `src-ui/src/lib/components/chat/ChatMessagesList.svelte` (flat loop); `src-ui/src/lib/types/message.ts` (no group concept) | **D** |

---

## Goal

Progressively enrich tool call card bodies so the agent's actions are immediately readable:

1. Bash cards show streaming stdout in a styled terminal box (P1).
2. Read cards show a compact, scannable file path list (P2).
3. Write/create cards show the actual unified diff inline with line numbers and red/green coloring (P3, P4).
4. Multiple same-tool invocations collapse into a single grouped card (P5) — deferred, largest architectural lift.

---

## Non-goals

- No changes to the card header UX, status indicators, or expand/collapse behaviour.
- No backend changes for workstreams A and B (streaming content and file paths are already available on the frontend).
- No redesign of `AnsweringIndicator` (the reference "Copilot is working…" is simpler than ours; ours is intentionally richer).
- No Monaco dependency changes (we stay off Monaco for inline diffs).
- No changes to `TerminalOutput` as a message kind (it continues to exist; workstream A adds an *additional* inline view inside the bash card body).

---

## Success metrics

Measured by opening a real agent session that reads, edits, and executes commands:

- Bash tool cards display the command's stdout in a terminal-styled scrollable box within the expanded card body.
- Read tool cards display a compact file path list (icon + dir + filename + optional line range + arrow), not raw JSON.
- Write/create tool cards display the actual unified diff (old/new line numbers, red/green row colors, "Show all N lines" truncation) inline.
- Grouped cards (workstream D): "Edit N files" header with per-file collapsible sections — deferred, measured separately.
- `npm run check` passes (no TypeScript errors).
- No visual regression in `webSearch` BodyComponent (only file affected is `ToolCallMessage.svelte`, which must still route `BodyComponent` correctly).

---

## Workstream A — Bash inline terminal output

**Fixes**: P1  
**Backend changes**: none — `agentData.streamingContent` is already populated during bash execution  
**Risk**: low — self-contained new `BodyComponent`; falls back to existing outcome text if `streamingContent` is absent  
**Blast radius**: `bashRenderer.ts`, new `BashBody.svelte`, new `BashBody.module.css`

### Rationale

`agentData.streamingContent` carries the live stdout/stderr string during and after tool execution. The bash renderer ignores it today; the generic `ToolCallMessage` body shows only the outcome text. The prototype (`bash-body.html`) demonstrates the target: a dark terminal box with a truncation notice when content exceeds a threshold, and a styled exit code line at the bottom.

### A.1 — New `BashBody.svelte` component

File: `src-ui/src/lib/components/chat/messages/BashBody.svelte`

Props (via the `BodyComponent` contract in `types.ts`):

```ts
interface Props {
  args:   { command?: string };
  result: { exit_code?: number } | undefined;
  status: ToolCallStatus;
  // Passed by ToolCallMessage — the raw streaming/result string
  streamingContent?: string;
}
```

Rendering logic:

- If `streamingContent` is empty and status is `'pending'`: show a pulsing placeholder line ("Waiting for output…").
- If `streamingContent` is present:
  - Truncate at **2 000 characters**. Show a notice banner above the box: "Output too large (N KB). Showing first 2 000 chars."
  - Render the content in a `<pre>` with `font-family: var(--font-mono)`, `font-size: var(--text-2xs)`, dark bg (`var(--color-neutral-925)`), `max-height: 240px`, `overflow-y: auto`.
  - Append a separator line and an exit code line: `<command exited with code N>` — muted/italic, matching the screenshot style.
- If `streamingContent` is absent and status is `'success'` or `'error'`: do not render the body (fall through to the generic outcome text). This avoids a blank body when the result is trivial.

### A.2 — Wire `streamingContent` into the `BodyComponent` call

File: `src-ui/src/lib/components/chat/messages/ToolCallMessage.svelte`

The current `BodyComponent` invocation (line ~159-163) passes `args`, `result`, `status`. Add `streamingContent={agentData?.streamingContent}` as a prop. Since `BodyComponent` is typed as `Component<{ args; result; status }>` in `types.ts`, extend the interface to include optional `streamingContent?: string`.

### A.3 — Register `BodyComponent` in `bashRenderer.ts`

```ts
import BashBody from '../BashBody.svelte';

export const bashRenderer: AnyToolRenderer = {
  icon: Terminal,
  getSummary: ...,
  getOutcome: ...,
  BodyComponent: BashBody,
};
```

### A.4 — Tests / verification

- Open a real agent session; trigger a `cargo build` or `ls -la` tool call.
- Verify: card body shows scrollable terminal output, truncation banner appears for large output.
- Verify: pending state shows the placeholder; error state (non-zero exit) still renders the output box.
- `npm run check` passes.

---

## Workstream B — Read compact file path list

**Fixes**: P2  
**Backend changes**: none — file path is in `toolArgs.filePath` or `toolArgs.path`  
**Risk**: low — self-contained new `BodyComponent`; does not change how `toolResult` is used  
**Blast radius**: `fsRenderers.ts`, new `FsReadBody.svelte`, new `FsReadBody.module.css`

### Rationale

The `fsReadRenderer` already extracts the file path from args (`args?.filePath ?? args?.path`) to build the subtitle. The prototype (`read-body.html`) shows that the body only needs this path — no content is displayed, just a scannable row. The current expanded body shows the "Target" param row and the raw JSON toggle; this workstream replaces the param row with the styled path row.

Note: when multiple files are read in sequence (P5, workstream D), each is a separate card today. This workstream improves the single-file case; the multi-file grouped version is deferred to D.

### B.1 — New `FsReadBody.svelte` component

File: `src-ui/src/lib/components/chat/messages/FsReadBody.svelte`

Props:

```ts
interface Props {
  args:   { filePath?: string; path?: string; startLine?: number; endLine?: number };
  result: { content?: string } | undefined;
  status: ToolCallStatus;
}
```

Rendering logic:

- Extract `filePath = args?.filePath ?? args?.path ?? ''`.
- Split into directory and filename for the two-tone path display (dir muted, filename brighter) — matching the reference and prototype.
- If `args.startLine` and `args.endLine` are present, show a line range badge: `:N–M` in monospace muted style.
- Show a file icon (Lucide `FileText`) with a language-colored dot derived from the extension. Support at minimum: `.rs` (rust orange `#f74c00`), `.ts`/`.tsx` (TS blue `#3178c6`), `.svelte` (orange `#ff3e00`), `.toml` (green), `.md` (gray), others fall back to `--color-text-muted`.
- Show a `›` arrow on the right (non-interactive, visual affordance only).
- If `result?.content` is present, show a muted badge: "N lines" derived from newline count — this replaces the current outcome text in a more integrated position.
- Do not render the raw file content inline.

### B.2 — Register `BodyComponent` in `fsRenderers.ts`

```ts
import FsReadBody from '../FsReadBody.svelte';

export const fsReadRenderer: ToolRenderer<FsReadArgs, FsReadResult> = {
  icon: FileText,
  getSummary: ...,
  getOutcome: ...,
  BodyComponent: FsReadBody,
};
```

### B.3 — Tests / verification

- Trigger a read tool call; verify the card body shows the compact path row, language dot, optional line range.
- Verify: a path without line range omits the badge; the `›` arrow is visible.
- Verify: the existing outcome text ("Read N lines") is no longer duplicated (it was shown in the header meta; the new body integrates it).
- `npm run check` passes.

---

## Workstream C — Inline unified diff for write operations

**Fixes**: P3, P4  
**Backend changes**: none — diff is available in the `file_touched` event / `TouchedFile.diff` (frontend agent store already receives it)  
**Risk**: medium — requires plumbing `touchedFiles` from the chat/agent store into `ToolCallMessage`; diff data correlation by `sessionId + filePath`  
**Blast radius**: new `InlineDiff.svelte`, new `FsWriteBody.svelte`, `ToolCallMessage.svelte` (pass `touchedFiles`), `fsRenderers.ts`, agent store access in body components

### Rationale

The diff string (unified diff format, `---/+++ @@ ...`) is produced by the backend for every write action and emitted as `file_touched { diff?: string }`. The `TouchedFile` type at `src-ui/src/lib/types/agent.ts:403-409` captures it. The `ToolCallMessage` body receives only `toolResult`, which for writes is a minimal success/error marker — no diff. To render the diff inline, we need to:

1. Build a lightweight `InlineDiff` component (no Monaco).
2. Pass the relevant `TouchedFile.diff` string from the agent session into the `FsWriteBody` component.

### C.1 — New `InlineDiff.svelte` component

File: `src-ui/src/lib/components/chat/messages/InlineDiff.svelte`

This is a pure HTML/CSS table-based diff renderer. No external libraries. Reference: `.prototypes/tool-widgets/inline-diff.html`.

Props:

```ts
interface Props {
  diff: string;            // Unified diff string (--- +++ @@ format)
  filePath: string;        // For language detection
  maxRows?: number;        // Truncation threshold (default: 80)
  collapsible?: boolean;   // Whether the file section header is clickable
  initiallyOpen?: boolean; // Default: true
}
```

Parsing the unified diff:

- Split on `\n`. For each line:
  - Starts with `@@` → hunk header row (muted, no line numbers)
  - Starts with `-` → deletion row: old line# (from hunk header parsing), no new line#, red-tinted bg
  - Starts with `+` → addition row: no old line#, new line# (from hunk header parsing), green-tinted bg
  - Else → context row: both line numbers, neutral bg
- Track old/new line counters from `@@ -A,B +C,D @@` hunk headers using a small parser.

Color tokens (from design system, already defined):

```css
/* Use existing syntax tokens — do NOT hardcode */
--color-syntax-addition-bg: rgba(143, 180, 122, 0.09)  /* bg for added rows */
--color-syntax-deletion-bg: rgba(200, 122, 130, 0.09)  /* bg for removed rows */
--color-editor-green: #8fb47a   /* + gutter + line numbers for additions */
--color-editor-red:   #c87a82   /* - gutter + line numbers for deletions */
```

Truncation: if the rendered rows exceed `maxRows`, clip and show a "Show all N lines" pill button centered below the table. Clicking expands to show all rows (local `$state`).

Table layout: `<table>` with four fixed columns: old-ln (36 px) | new-ln (36 px) | gutter (18 px) | code (auto). Use `table-layout: fixed` to prevent layout thrash. Monospace font, `font-size: var(--text-2xs)`, `line-height: 18px`.

### C.2 — New `FsWriteBody.svelte` component

File: `src-ui/src/lib/components/chat/messages/FsWriteBody.svelte`

Props:

```ts
interface Props {
  args:         { filePath?: string; path?: string };
  result:       unknown;
  status:       ToolCallStatus;
  touchedFile?: TouchedFile;  // Injected by ToolCallMessage (see C.3)
}
```

Rendering logic:

- If `touchedFile?.diff` is present: render `<InlineDiff diff={touchedFile.diff} filePath={resolvedPath} />`.
- If `touchedFile` is present but `diff` is absent (e.g. binary file or diff not yet available): show a muted "File modified — diff unavailable" note.
- If `touchedFile` is absent and status is `'pending'`: show a pulsing placeholder ("Applying changes…").
- If status is `'error'`: do not render the body (the header status indicator handles it).

### C.3 — Pass `touchedFile` into `BodyComponent` from `ToolCallMessage`

File: `src-ui/src/lib/components/chat/messages/ToolCallMessage.svelte`

The component already receives `message: Message`. To correlate with `TouchedFile`:

- Import from the agent session store: `import { agentStore } from '$lib/stores/agent'`.
- Derive `touchedFile`:

  ```ts
  const touchedFile = $derived(
    agentData?.sessionId
      ? agentStore.getTouchedFile(agentData.sessionId, resolvedFilePath)
      : undefined
  );
  ```

- Add `getTouchedFile(sessionId: string, filePath: string): TouchedFile | undefined` selector to the agent store's public API (reads from `state.sessions.get(sessionId)?.touchedFiles`).

- Pass `touchedFile` as a prop to `BodyComponent`:

  ```svelte
  <svelte:component this={renderer.BodyComponent} {args} {result} {status} {streamingContent} {touchedFile} />
  ```

  Components that do not use `touchedFile` ignore the prop; Svelte 5 does not warn on extra props passed to components.

### C.4 — Register `BodyComponent` in `fsRenderers.ts`

```ts
import FsWriteBody from '../FsWriteBody.svelte';

export const fsWriteRenderer: AnyToolRenderer = {
  icon: FilePenLine,
  getSummary: ...,
  getOutcome: ...,
  BodyComponent: FsWriteBody,
};
```

Apply the same for `fsCreateRenderer` if it exists separately; otherwise the canonical `fs_write_file` renderer covers file creation (backend uses the same tool for create + overwrite).

### C.5 — Tests / verification

- Trigger a write tool call in a real agent session; verify the card body renders the actual diff.
- Verify: truncation pill appears when the diff exceeds 80 rows; clicking it expands to full diff.
- Verify: pending state shows the placeholder; the diff appears once the `file_touched` event arrives.
- Verify: binary-file writes (no diff) show the fallback note, not a blank body.
- Verify: `InlineDiff` correctly parses the `@@` hunk header and assigns line numbers.
- `npm run check` passes.

---

## Workstream D — Grouped tool call cards (deferred)

**Fixes**: P5  
**Backend changes**: none  
**Risk**: high — architectural change to `ChatMessagesList` flat loop; risk of breaking message ordering, keyboard navigation, and streaming updates  
**Status**: DEFERRED — do not start until A + B + C are shipped and stable

### Rationale

The reference shows "Edit 4 files" and "View 4 files" as a single card. This requires detecting consecutive same-tool-name runs across `messages[]` and rendering them as a virtual group node. The core challenge is that messages arrive via streaming — the "group" can grow as new tool calls arrive, requiring a reactive grouping layer that handles:

- Insertion at the tail of an in-progress group
- Group boundary detection (tool name change, non-tool message in between, session change)
- Streaming status propagation (the group header must reflect the aggregate status: pending if any member is pending, error if any errored)

### D.1 — Message pre-processing in `ChatMessagesList`

Replace the flat `messages` loop with a `groupedMessages` derived list:

```ts
type MessageGroup =
  | { type: 'single'; message: Message }
  | { type: 'tool-group'; toolName: string; messages: Message[]; sessionId: string };
```

Grouping rule: consecutive `kind === 'tool_call'` messages with the same normalized `toolName` (using the renderer registry's canonical name) and the same `sessionId` form a group. Any non-tool-call message between them breaks the group.

Canonical name normalization: reuse `getToolRenderer(toolName).getSummary(...).title` as the group label, or add a `canonicalName` export to the renderer registry.

### D.2 — New `GroupedToolCallMessage.svelte`

Consumes `messages: Message[]` (the group members). Renders:

- A single card header: `Edit N files` or `Read N files` with an aggregate status indicator (success only when all members succeed).
- Inside the expanded body: one `InlineDiff` (write) or one file-row (read) per member, each collapsible.

Reuses `InlineDiff.svelte` and `FsReadBody.svelte` rather than duplicating them.

### D.3 — Sequencing constraint

D must follow C because `GroupedToolCallMessage` reuses `InlineDiff` directly. Attempting D without C means the grouped write card has no diff renderer.

### D.4 — Tests / verification

- Trigger an agent that edits 3 files consecutively; verify a single "Edit 3 files" card appears.
- Verify: reading 1 file → single card (no grouping artifact); reading 4 files → grouped card.
- Verify: streaming — group header updates its status as each member completes.
- Verify: a narration message between two write calls breaks the group (two separate cards).

---

## Sequencing

```
Phase 1 (Tier 1):  A — Bash terminal body   (no dependencies, lowest risk)
                   B — Read file list body   (no dependencies, lowest risk)
Phase 2 (Tier 2):  C — Inline diff          (depends on: InlineDiff component; agent store selector)
Phase 3 (Tier 3):  D — Grouped cards        (depends on: C; architectural; own planning cycle)
```

A and B can be implemented in parallel — they are fully independent. C can begin once A and B are merged (or in parallel — it shares no files with A/B). D requires C.

---

## Risk register

| Risk | Mitigation |
|------|------------|
| `getTouchedFile` selector reads stale data before `file_touched` event arrives (C.3) | `FsWriteBody` renders a pending placeholder until `touchedFile?.diff` is available; no race error, just a delayed diff reveal |
| `InlineDiff` hunk-header parser fails on malformed diffs from the backend | Wrap line parsing in `try/catch`; on error, render a "Diff parse error — view raw" fallback that shows the raw diff string in a `<pre>` |
| Passing `touchedFile` prop to all `BodyComponent` instances breaks components that don't expect it | Svelte 5 ignores unknown props at runtime; TypeScript check: the base `BodyComponent` interface uses `[key: string]: unknown` index signature for extensibility |
| `BashBody` renders `streamingContent` that is not yet finalized — partial stdout mid-stream | Intentional — shows live progress. The `exit_code` line is only rendered when `status !== 'pending'` |
| Long diffs cause layout thrash (large `<table>`) | Default `maxRows: 80` limits initial render; expand is lazy (toggled by user click) |
| Workstream D grouping breaks message ordering during streaming | D is explicitly deferred; must be planned with a full streaming-state audit before implementation |

---

## Open questions

1. Does `TouchedFile.diff` include the file path header lines (`--- a/... +++ b/...`)? If so, `InlineDiff` should strip them before rendering (they are redundant with the card's file path display). Verify against a live `file_touched` event payload.
2. Does the agent store expose `getTouchedFile` or an equivalent selector today? If not, the selector must be added in C.3 — check `src-ui/src/lib/stores/agent/derived.ts`.
3. For binary files (images, compiled artifacts), does the backend emit a `file_touched` event with `diff: undefined` or skip the event entirely? This determines whether the "diff unavailable" fallback in `FsWriteBody` is reachable.
4. Is `fsCreateRenderer` a separate registry entry from `fsWriteRenderer`, or does file creation flow through the same tool name? Check `src-ui/src/lib/components/chat/messages/tool-renderers/registry.ts`.

---

## Files to create

| File | Workstream |
|------|------------|
| `src-ui/src/lib/components/chat/messages/BashBody.svelte` | A |
| `src-ui/src/lib/components/chat/messages/BashBody.module.css` | A |
| `src-ui/src/lib/components/chat/messages/FsReadBody.svelte` | B |
| `src-ui/src/lib/components/chat/messages/FsReadBody.module.css` | B |
| `src-ui/src/lib/components/chat/messages/InlineDiff.svelte` | C |
| `src-ui/src/lib/components/chat/messages/InlineDiff.module.css` | C |
| `src-ui/src/lib/components/chat/messages/FsWriteBody.svelte` | C |
| `src-ui/src/lib/components/chat/messages/FsWriteBody.module.css` | C |

## Files to modify

| File | Workstream | Change |
|------|------------|--------|
| `src-ui/src/lib/components/chat/messages/tool-renderers/bashRenderer.ts` | A | Add `BodyComponent: BashBody` |
| `src-ui/src/lib/components/chat/messages/tool-renderers/fsRenderers.ts` | B, C | Add `BodyComponent` to `fsReadRenderer` and `fsWriteRenderer` |
| `src-ui/src/lib/components/chat/messages/tool-renderers/types.ts` | A | Extend `BodyComponent` props interface with optional `streamingContent?: string` |
| `src-ui/src/lib/components/chat/messages/ToolCallMessage.svelte` | A, C | Pass `streamingContent` and `touchedFile` into `BodyComponent` invocation |
| `src-ui/src/lib/stores/agent/derived.ts` (or equivalent) | C | Add `getTouchedFile(sessionId, filePath)` selector |

---

## Deliverables

- PR per workstream (A+B can be one PR given their size).
- Each PR includes a screenshot of the new card body in a real agent session.
- Update this plan's **Status** to `IN PROGRESS` when A starts; to `COMPLETE` when C ships; note D's own planning doc when it is opened.
