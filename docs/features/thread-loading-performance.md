# Thread Loading Performance

**Status**: ACTIVE
**Last reviewed**: 2026-04-23
**Purpose**: Document the rendering pipeline, doc-gen contract, and perf budget that keep long-chat thread opens responsive.

---

## Problem

Opening a thread with a long chat used to freeze the frontend for 1–3+ seconds. The backend was not the bottleneck — `chat_messages` has a composite index on `(thread_id, timestamp ASC)` (`src-tauri/src/infrastructure/sqlite/migrations/chat_schema.rs`), so SQL is a fast indexed scan. All the time was spent on the frontend between IPC return and first paint.

Root causes, in order of impact:

1. **`hljs.highlightAuto` per untagged code block.** `highlight.js` runs every registered grammar against the buffer to detect language; agent output frequently emits untagged code, so this ran hundreds of times per thread open.
2. **Markdown pipeline ran twice per message.** `MarkdownRenderer` rendered HTML, then materialised it into a temp DOM via `document.createElement + innerHTML` just to split code blocks from prose — a full reparse of every message.
3. **No virtualization.** `ChatMessagesList` mounted every message on a single frame with a flat `{#each}`.
4. **Reactive fan-out per streaming token.** `displayItems` (filter + group) and a per-message `$effect` that iterated all messages for doc-gen tracking both re-ran on every reactive update, including every token during streaming.

## Rendering pipeline

### Markdown: sentinel-based code block extraction

`src-ui/src/lib/utils/markdown.ts` exposes two entry points:

- `renderMarkdown(content)` — returns a single HTML string with code blocks rendered inline as `<pre class="code-block">…</pre>`. Used by callers that don't need separately-rendered code blocks.
- `renderMarkdownWithBlocks(content): { html, codeBlocks }` — returns HTML with `<!--CODEBLOCK:N-->` sentinels in place of each fenced code block, plus a parallel `codeBlocks[]` array. Used by `MarkdownRenderer.svelte`, which can then split the HTML on the sentinel regex and render each code block with the `CodeBlock` component without re-parsing the HTML through the DOM.

The `code(token)` custom renderer is shared between both paths; the entry point swaps a per-call `currentBlocks` side-array via a module closure to control which emission format is used.

`CODE_BLOCK_SENTINEL_RE` (`/<!--CODEBLOCK:(\d+)-->/g`) is the canonical split boundary, exported for tests and split-helpers.

### Highlighting: slim `hljs` + cheap heuristic

`src-ui/src/lib/utils/hljs.ts` owns a single slim `highlight.js/lib/core` instance with only the languages the app actually uses registered (`bash`, `json`, `typescript`, `javascript`, `python`, `rust`, `sql`, `yaml`, `xml`, `markdown`, `plaintext`, plus common aliases). `ToolCallMessage.svelte` and `ToolCallItem.svelte` import from here too, so the full 190-grammar bundle never ships.

`markdown.ts` never calls `hljs.highlightAuto`. For untagged code blocks, a one-shot first-line heuristic picks `json` (leading `{` / `[`) or `bash` (`$ `, `#!/`, common command prefixes) when obvious; everything else falls through to escaped plaintext.

Tagged code blocks larger than `MAX_HIGHLIGHT_BYTES` (10 KB) are also rendered as escaped plaintext — highlighting cost on a single large buffer outweighs readability gain.

### Per-component markdown memoization

`MarkdownRenderer.svelte` caches `{ content, renderedMarkdown }` locally. Streaming only mutates the trailing message's `content`, so non-streaming messages hit the cache on every re-render caused by surrounding reactivity and skip `marked` + `hljs` entirely. The cache is per-instance rather than a global LRU to avoid cross-thread memory accumulation and sidestep cache-churn during streaming.

### `groupToolMessages` memoization

`src-ui/src/lib/utils/groupToolMessages.ts` keeps a module-level `(entries, contents)` pair. On each call it walks the prior entries, reusing any whose source-message `(id, content)` tuples still match the head of the new input. Mismatches and the tail are recomputed. Net effect: during streaming only the last display item (the one containing the streaming message) gets a fresh identity; Svelte's keyed `{#each}` then skips rerendering subtrees for every other item.

### Progressive rendering

`ChatMessagesList.svelte` renders the trailing `visibleCount` of `displayItems`:

- On initial render for a thread, `visibleCount = min(total, 50)`.
- Subsequent `requestAnimationFrame` ticks grow it by 50 until all items are rendered.
- `await tick()` is used between chunks so Svelte has reconciled before the next growth step.
- A cancel token guards against thread-switch races: if a new thread becomes selected mid-loop, the previous loop stops immediately.

`ChatArea.svelte` handles scroll preservation via two callbacks:

- `onBeforeProgressiveChunk` captures `anchor = scrollHeight - scrollTop` (distance from top to bottom of content).
- `onAfterProgressiveChunk` restores `scrollTop = scrollHeight - anchor` after the chunk has reconciled. Preserves distance-from-bottom: users at the bottom stay at the bottom; users mid-scroll keep their visual anchor.

`firstVisibleIndex = displayItems.length - visibleCount` gives the absolute index into the full list; `shouldShowGroupHeader(index)` and `shouldShowAgentHeader(index)` are fed the absolute index so header-gap computations are unaffected by windowing.

## Doc-gen tracking contract

Previously, `ChatMessagesList.svelte` ran a per-render `$effect` that iterated every `messages[i]` and called `documentGenerationStore.processMessage(m)`. That scan was O(N) per reactive update — including every streaming token.

The current contract:

- **Append path.** `createMessageMethods.addMessage` (`src-ui/src/lib/stores/chat/message-methods.ts`) calls `documentGenerationStore.processMessage(message)` once per appended message.
- **Update path.** `createMessageMethods.updateMessage` builds the merged next-value and calls `processMessage(merged)` once. Tool-call status/result updates arrive through this path from the agent event bus and advance doc-gen phase transitions.
- **Bulk load.** `chatStore.selectThread` (`src-ui/src/lib/stores/chat.ts`) calls `documentGenerationStore.rebuildFromMessages(loadedMessages)` after the thread's messages return from IPC. This replays history into the tracker in a single batched pass, reconstructing any visible doc-gen card state exactly as the old `$effect` would have.

`DocumentGenerationTracker.processMessage` (`src-ui/src/lib/utils/documentGenerationTracker.ts`) is idempotent for the same input, so the replay-on-load path is safe.

## Performance budget

For the supported thread-size range, first-paint target from click to last newest-message rendered:

| Thread size | Target first-paint |
|---|---|
| ≤ 100 messages | < 100 ms |
| 101–1000 messages | < 200 ms |
| 1001–3000 messages | < 400 ms (progressive fill-in completes within ~1 s) |

Above ~3000 messages, windowed virtualization would be the next step (deferred — see `docs/planning` if the need lands).

## Critical files

| File | Responsibility |
|---|---|
| `src-ui/src/lib/utils/markdown.ts` | Markdown rendering, sentinel emission, highlight-size cap, heuristic language detection |
| `src-ui/src/lib/utils/hljs.ts` | Shared slim `highlight.js` instance with curated languages |
| `src-ui/src/lib/components/markdown/MarkdownRenderer.svelte` | Per-instance content memoization; sentinel split; `CodeBlock` rendering |
| `src-ui/src/lib/utils/groupToolMessages.ts` | Stable-prefix memoization of tool-call grouping |
| `src-ui/src/lib/components/chat/ChatMessagesList.svelte` | Progressive rendering; absolute index into `displayItems` |
| `src-ui/src/lib/components/chat/ChatArea.svelte` | Scroll-preservation callbacks for progressive chunks |
| `src-ui/src/lib/stores/documentGenerationStore.ts` | `rebuildFromMessages` batch entry point |
| `src-ui/src/lib/stores/chat/message-methods.ts` | Single-append doc-gen hook for `addMessage` / `updateMessage` |
| `src-ui/src/lib/stores/chat.ts` | `selectThread` replays history into `documentGenerationStore` once |

## Regression guards

- `src-ui/src/lib/utils/markdown.test.ts` — tagged and untagged code highlighting paths, heuristic dispatch, 10 KB size-cap fallback, 100×5 KB perf guard (< 500 ms), sentinel emission, reference-link resolution across code blocks, `splitOnSentinels` correctness.
- `src-ui/src/lib/utils/groupToolMessages.test.ts` — grouping semantics and memoization invariants (prefix reuse, tail invalidation, thread-switch reset).
