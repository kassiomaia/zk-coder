# Tool System And Agent Integration

**Status**: ACTIVE
**Last reviewed**: 2026-04-04
**Purpose**: Reference for how backend tools are implemented, how the agent invokes them, and what parameters each built-in tool accepts

---

## Overview

The tool system is implemented in Rust under `src-tauri/src/infrastructure/tools/`. It gives the LLM and the agent runtime a typed catalog of callable operations such as reading files, executing commands, searching, and creating document artifacts.

At runtime, tools sit between:

- the LLM response format (`ToolCall`, `ToolDefinition`, `ToolResult`)
- the agent execution pipeline
- the permission and approval system
- frontend event streams and persisted session telemetry

The current built-in registry exposes 31 tools across 8 categories:

| Category | Count | Purpose |
|---|---:|---|
| `system` | 2 | App metadata and shell execution |
| `fs` | 3 | Workspace file reads, listing, and writes |
| `conversation` | 1 | Search chat history |
| `search` | 3 | Workspace grep and production / dev-only web search |
| `agent` | 5 | Status, narration, and subtask management |
| `docx` | 4 | Incremental Word document drafting |
| `pptx` | 8 | Incremental PowerPoint drafting |
| `pdf` | 5 | Incremental PDF drafting |

## Core Implementation

### Registry

`ToolRegistry` in `src-tauri/src/infrastructure/tools/mod.rs` is the central entry point.

Responsibilities:

1. Register tool categories and concrete tool implementations.
2. Expose LLM-facing definitions through `definitions()`.
3. Execute tools by name through `execute()`.
4. Enforce feature flags and workspace requirements before execution.

Each tool contributes a `ToolMeta` value:

| Field | Meaning |
|---|---|
| `id` | Category/name pair, exposed as `category_name` |
| `description` | Natural-language description sent to the model |
| `input_schema` | JSON Schema used for tool-call arguments |
| `requires_workspace` | Whether the runtime must provide a workspace root |
| `experimental` | Whether the tool is unstable |
| `feature_flag` | Optional environment flag required for exposure/execution |

### Tool Contract

All tools implement the `Tool` trait in `src-tauri/src/infrastructure/tools/traits.rs`.

Execution signature:

- `ctx: ToolExecutionContext`
- `arguments: serde_json::Value`
- `app_handle: tauri::AppHandle`
- `workspace_root: Option<PathBuf>`

The tool returns either:

- `Ok(JsonValue)` for structured success output
- `Err(ToolError)` for structured failure

### Execution Context

`ToolExecutionContext` carries request-scoped data:

| Field | Source | Typical use |
|---|---|---|
| `conversation_id` | Session/request | Conversation search and telemetry correlation |
| `thread_id` | Agent session | Frontend event correlation |
| `workspace_root` | Session or registry | Workspace-bound tools |
| `session_id` | Agent session | Streaming events, persistence, approvals |

## End-To-End Flow

### 1. Tool definitions are sent to the LLM

During context resolution, the agent runtime gathers tool definitions from `ToolRegistry::definitions()`. Those definitions are then filtered by the active subtask mode before being sent with the LLM request.

Relevant code:

- `src-tauri/src/infrastructure/tools/mod.rs`
- `src-tauri/src/application/agent/execution/context_phase.rs`

### 2. The LLM returns structured tool calls

The LLM layer parses tool calls into `crate::core::llm::entities::ToolCall`:

| Field | Meaning |
|---|---|
| `id` | Provider-generated call identifier |
| `name` | Full tool name such as `fs_read_file` |
| `arguments` | JSON object matching the registered schema |

Relevant code:

- `src-tauri/src/core/llm/entities.rs`
- `src-tauri/src/infrastructure/llm/openai/client.rs`

### 3. Response handling decides whether to execute tools

If the LLM response contains tool calls, the response phase:

1. records the LLM step
2. checks for repetitive loops
3. streams any preamble text
4. returns control to the tool phase

Relevant code:

- `src-tauri/src/application/agent/execution/response_phase.rs`

### 4. Tool phase executes the requested calls

The tool phase:

1. appends the assistant message containing the tool calls to LLM context
2. runs permission-aware tool execution
3. converts each tool result into a `tool` role message
4. appends those results back into LLM context for the next model call

Relevant code:

- `src-tauri/src/application/agent/execution/tool_phase.rs`

### 5. Permission layer decides auto-run vs approval

`PermissionResolver` evaluates each tool call using:

1. trust mode
2. per-tool overrides
3. per-category overrides
4. default action type rules

Default behavior in `Untrusted` mode:

- reads/searches auto-run
- file writes and command execution require approval
- document draft tools auto-run

Relevant code:

- `src-tauri/src/core/agent/permissions.rs`
- `src-tauri/src/infrastructure/agent/runtime/permissions.rs`

### 6. Runtime emits events and persists telemetry

Before and after every call, the runtime persists telemetry and emits frontend events:

| Event | Purpose |
|---|---|
| `ToolCall` | A tool invocation started |
| `ToolResult` | A tool finished |
| `ApprovalRequired` | User confirmation is needed |
| `CommandOutput` | Streaming stdout/stderr |
| `CommandFinished` | Command exit code available |
| `FileTouched` | File read/write/delete tracked |
| `StatusPhrase` | Transient spinner message |
| `AgentNarration` | Persistent user-facing progress update |
| `SubtaskCreated` / `SubtaskUpdated` | Agent task progress |
| `ArtifactCreated` | Finalized document artifact became available |

Persisted runtime telemetry currently includes:

- tool calls
- file touches
- terminal output
- status phrases
- execution steps
- task and subtask state

Relevant code:

- `src-tauri/src/infrastructure/agent/events.rs`
- `src-tauri/src/infrastructure/agent/runtime/permissions.rs`
- `src-tauri/src/application/agent/service/runtime_queries.rs`
- `docs/features/agent-session-runtime-persistence.md`

## Special Behavior By Tool Type

### Runtime-intercepted agent tools

Some `agent_*` tools are registered like normal tools, but their real side effects are handled directly inside the runtime instead of delegating to the generic tool registry.

Examples:

- `agent_plan_subtasks`
- `agent_advance_to_next_subtask`
- `agent_mark_subtask_status`
- `agent_narrate`

`agent_emit_status` is also observed specially after execution so the runtime can emit `StatusPhrase` events and persist status phrases.

### Workspace safety

Workspace tools resolve paths relative to the session workspace and guard against path traversal by verifying resolved paths remain inside the workspace root.

Applies to:

- `fs_read_file`
- `fs_list_files`
- `fs_write_file`
- `search_grep`
- `system_execute_command` working directory resolution

### Command spawning

`system_execute_command` now routes through the shared backend spawn service in `src-tauri/src/infrastructure/agent/runtime/spawn.rs`.

The service enforces:

1. workspace-bound working directories
2. per-session single-flight protection unless explicitly allowed
3. a global concurrency limit
4. shell execution policy checks
5. output truncation limits
6. cancellation and timeout handling
7. runtime trace events for `SpawnStarted`, `SpawnFinished`, `SpawnDenied`, `SpawnFailed`, `SpawnTimedOut`, `SpawnCancelled`, `SpawnKilled`, and `SpawnBusy`

Prefer direct `program` + `args` launches when the caller does not need shell features. Use `command` only when shell syntax is required.

### Rollback integration

Before `fs_write_file` and `system_execute_command`, the runtime captures file or workspace snapshots so changes can be inspected or rolled back later.

### Document tools use draft stores, not the workspace

The `docx_*`, `pptx_*`, and `pdf_*` tools operate on in-memory draft stores plus artifact output, not on arbitrary workspace files. They are therefore not marked as `requires_workspace` and default to auto-run.

## Tool Catalog

## `system`

| Tool | Description | Parameters |
|---|---|---|
| `system_get_app_info` | Returns backend package metadata. | None |
| `system_execute_command` | Executes a shell command in the workspace and streams output. | `command: string` required. `working_dir: string` optional, relative to workspace root. |

## `fs`

| Tool | Description | Parameters |
|---|---|---|
| `fs_read_file` | Reads UTF-8 file content from the workspace. | `path: string` required. `max_bytes: integer` optional, default `16000`, max `200000`. |
| `fs_list_files` | Lists files under a workspace path with bounded recursion. | `path: string` optional, default workspace root. `max_depth: integer` optional, default `3`, max `8`. `limit: integer` optional, default `200`, max `500`. |
| `fs_write_file` | Writes or overwrites a workspace file. | `path: string` required. `content: string` required. `create_dirs: boolean` optional, default `true`. |

## `conversation`

| Tool | Description | Parameters |
|---|---|---|
| `conversation_search_messages` | Case-insensitive search over messages in the current conversation. | `query: string` required. `limit: integer` optional, default `20`, max `100`. |

## `search`

| Tool | Description | Parameters |
|---|---|---|
| `search_grep` | Regex search across workspace files. | `query: string` required. `max_results: integer` optional, default `50`, max `200`. `case_sensitive: boolean` optional. `path: string` optional subpath scope. |
| `search_google_scrape` | Experimental HTML-scraping Google search. Exposed only when `CODER_LLM_TOOL_GOOGLE_SCRAPE=1`. | `query: string` required. `num_results: integer` optional, default `5`, max `10`. `language: string` optional. |
| `search_web_search` | Production Google Custom Search JSON API wrapper with per-workspace domain policy, SQLite-backed result cache (24 h TTL), system-locale fallback for `hl`, and structured `error_code` bodies. Gated by `CODER_LLM_TOOL_WEB_SEARCH=1` and DB-stored credentials managed in Settings → Tools → Web Search. Spec: `docs/tools/web_search/spec.md`. | `query: string` required (1–512 chars). `num_results: integer` optional, default `5`, max `10`. `language: string` optional (BCP-47); defaults to the host system locale. `site: string` optional; rejected with `filtered_by_policy` when the active workspace's allow/deny list excludes it. |

## `agent`

| Tool | Description | Parameters |
|---|---|---|
| `agent_emit_status` | Emits a short transient spinner/status phrase. | `message: string` required. Intended for 2-6 word status phrases. |
| `agent_narrate` | Sends a persistent natural-language progress update into chat history. | `message: string` required. Intended for 1-2 sentence updates. |
| `agent_plan_subtasks` | Creates an ordered subtask plan for the current task. | `subtasks: array` required, 1-10 items. Each item requires `title: string`; `description: string` optional. |
| `agent_advance_to_next_subtask` | Completes the current subtask and advances to the next one. | No parameters. |
| `agent_mark_subtask_status` | Manually sets a specific subtask status. | `index: integer` required, zero-based. `status: string` required, one of `pending`, `in_progress`, `completed`, `failed`. `rationale: string` optional. |

## `docx`

| Tool | Description | Parameters |
|---|---|---|
| `docx_start` | Starts a new Word document draft. | `filename: string` required. `title: string` required. `template: string` optional. |
| `docx_add_section` | Adds one section to a Word draft. | `document_id: string` required. `content: object` required; see detailed shapes below. |
| `docx_list_sections` | Lists document sections and optional content previews. | `document_id: string` required. `include_content: boolean` optional, default `false`. |
| `docx_finalize` | Generates the `.docx` artifact and closes the draft. | `document_id: string` required. `auto_save: boolean` optional, default `true`. |

### `docx_add_section` content shapes

`content` must be one of:

- `Heading`: `type`, `level` (1-6), `text`, `page_break_before`
- `Paragraph`: `type`, `text`, `style`
- `BulletList`: `type`, `items`
- `NumberedList`: `type`, `items`, `start_at`
- `Table`: `type`, `headers`, `rows`, `caption`
- `Image`: `type`, `path`, `caption`, `width`
- `PageBreak`: `type`
- `HorizontalRule`: `type`

## `pptx`

| Tool | Description | Parameters |
|---|---|---|
| `pptx_start` | Starts a new PowerPoint draft. | `filename: string` required. `title: string` required. `template: string` optional. `theme: string` optional. |
| `pptx_add_slide` | Adds a slide to a presentation draft. | `deck_id: string` required. `title: string` required. `layout: string` optional. `content: array` optional. `notes: string` optional. `position: string` optional, `start` or `end`. |
| `pptx_list_slides` | Lists slides with layout/title summaries. | `deck_id: string` required. `include_content: boolean` optional, default `false`. |
| `pptx_get_slide` | Returns one slide by number or ID. | `deck_id: string` required. `slide: string` required. `include_notes: boolean` optional, default `false`. |
| `pptx_search_slides` | Searches slide titles/content and optionally notes. | `deck_id: string` required. `query: string` required. `search_in_notes: boolean` optional, default `false`. |
| `pptx_update_slide` | Updates selected fields on an existing slide. | `deck_id: string` required. `slide: string` required. `layout: string` optional. `title: string` optional. `content: array` optional. `notes: string|null` optional. |
| `pptx_delete_slide` | Deletes a slide by number or ID. | `deck_id: string` required. `slide: string` required. |
| `pptx_finalize` | Generates the `.pptx` artifact and closes the draft. | `deck_id: string` required. `auto_save: boolean` optional, default `true`. |

### `pptx_add_slide` and `pptx_update_slide` content item shapes

`content` items currently support:

- `title`
- `subtitle`
- `text_box`: `text`, `position` optional
- `bullet_list`: `items`, `level` optional
- `numbered_list`: `items`
- `table`: `headers`, `rows`
- `image`: `path`, `caption` optional
- `chart_placeholder`: `title`, `data_type`

Supported layout strings:

- `TitleSlide`
- `TitleAndContent`
- `SectionHeader`
- `TwoColumn`
- `TwoContent`
- `Comparison`
- `ContentWithCaption`
- `Blank`

## `pdf`

| Tool | Description | Parameters |
|---|---|---|
| `pdf_start` | Starts a new PDF draft. | `filename: string` required. `title: string` required. `page_size: string` optional, one of `A4`, `Letter`, `Legal`, `A3`. |
| `pdf_add_page` | Adds a page to a PDF draft. | `pdf_id: string` required. `content: object` required. `position: string` optional, `start` or `end`. |
| `pdf_list_pages` | Lists page summaries and optional previews. | `pdf_id: string` required. `include_content: boolean` optional, default `false`. |
| `pdf_get_page` | Returns one page by number or page ID. | `pdf_id: string` required. `page: string` required. |
| `pdf_delete_page` | Deletes a page by number or page ID. | `pdf_id: string` required. `page: string` required. |
| `pdf_finalize` | Generates the `.pdf` artifact and closes the draft. | `pdf_id: string` required. `auto_save: boolean` optional, default `true`. |

### `pdf_add_page` content shapes

`content` must be one of:

- `markdown`: `content`
- `html`: `content`, `styles` optional
- `text`: `content`
- `image`: `path`, `caption` optional

## Notable Output Contracts

### Generic execution contract

Regardless of tool type, the registry returns `ToolResult`:

| Field | Meaning |
|---|---|
| `call_id` | Original LLM tool-call ID |
| `output` | Arbitrary JSON result or JSON error payload |
| `is_error` | Whether execution failed |

### Session runtime inspection contract

Persisted tool history is re-exposed through `ToolCallView`:

| Field | Meaning |
|---|---|
| `id` | Internal persisted tool-call row ID |
| `sessionId` | Agent session ID |
| `toolName` | Tool name such as `fs_write_file` |
| `arguments` | Original JSON arguments |
| `result` | JSON result, if completed |
| `isError` | Whether the call failed |
| `durationMs` | Duration when available |
| `subtaskId` | Linked subtask, if any |
| `timestamp` | Persisted timestamp |

This is the contract used when rehydrating historical sessions in the frontend.

## Implementation File Map

| Concern | Primary files |
|---|---|
| Tool registry and traits | `src-tauri/src/infrastructure/tools/mod.rs`, `src-tauri/src/infrastructure/tools/traits.rs`, `src-tauri/src/infrastructure/tools/context.rs` |
| Tool implementations | `src-tauri/src/infrastructure/tools/**` |
| Agent execution phases | `src-tauri/src/application/agent/execution/context_phase.rs`, `response_phase.rs`, `tool_phase.rs` |
| Permission and approval flow | `src-tauri/src/core/agent/permissions.rs`, `src-tauri/src/infrastructure/agent/runtime/permissions.rs`, `src-tauri/src/infrastructure/agent/approval.rs` |
| Event emission | `src-tauri/src/infrastructure/agent/events.rs` |
| Session telemetry rehydration | `src-tauri/src/application/agent/service/runtime_queries.rs`, `src-tauri/src/application/agent/read_models.rs` |

## Current Limitations And Notes

- `search_google_scrape` is experimental and hidden behind a feature flag.
- Document finalize tools still emit `ArtifactCreated` with placeholder `session_id: "current"` in some paths; this is marked in code with `TODO`.
- Some document tool schemas are broader in Rust types than the simplified JSON schema snippets used in metadata.
- `agent_emit_status` and `agent_narrate` are first-class tools for the model, but their user-visible effects are driven by runtime event handling rather than by generic tool output alone.

## Planned Extensions

- `docs/filesystem-tools-proposal.md` - Proposal for partial-edit, glob, metadata, and fuzzy-search filesystem tools
