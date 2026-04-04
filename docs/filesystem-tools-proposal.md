# Filesystem Tools Proposal

**Status**: DRAFT
**Last reviewed**: 2026-04-04
**Purpose**: Proposal for expanding filesystem tooling with partial-edit, globbing, metadata, and fuzzy-search capabilities

---

## Goal

Add a focused second wave of filesystem tools so the agent can:

- edit files without rewriting the entire file body
- find files by glob pattern
- inspect file size and filesystem attributes
- find likely paths when the exact path is unknown

The proposal is intentionally limited to tools that improve agent precision and reduce unnecessary full-file writes.

## Why We Need These Tools

The current filesystem toolset is:

- `fs_read_file`
- `fs_list_files`
- `fs_write_file`

That is enough for basic workflows, but it forces the agent into poor behavior in several common cases:

1. Small edits require reading a whole file and then rewriting the entire file.
2. File discovery is directory-oriented, not pattern-oriented.
3. There is no way to inspect file metadata without opening file contents.
4. When the user only remembers part of a path, the agent has no dedicated fuzzy path lookup.

## Proposed Tool Set

This proposal recommends five new tools:

| Tool | Category | Main use |
|---|---|---|
| `fs_replace_in_file` | write | Deterministic string replacement in an existing file |
| `fs_apply_patch` | write | Multi-hunk line-based edits without whole-file rewrite |
| `fs_glob_files` | read/search | Match files by glob patterns under a workspace path |
| `fs_stat_path` | read | Return file size, type, timestamps, and permissions |
| `fs_fuzzy_find_files` | read/search | Ranked fuzzy path search |

## Design Principles

### 1. Prefer deterministic edits over free-form edits

The safest write tools should:

- fail loudly on ambiguity
- return structured counts and previews
- support approval previews before mutation

### 2. Keep read/search tools cheap and approval-free

Globbing, metadata inspection, and fuzzy path search should remain auto-run by default in `Untrusted` mode.

### 3. Reuse existing workspace safety rules

All new filesystem tools should:

- require a workspace
- resolve paths relative to the workspace root
- reject path traversal
- emit `FileTouched` or equivalent telemetry when relevant

### 4. Avoid redundant dependencies

Use the standard library where it is sufficient. Add crates only where they materially improve correctness, matching quality, or implementation complexity.

## Tool Proposals

## `fs_replace_in_file`

### Purpose

Perform one or more exact or bounded replacements in an existing file without sending the whole final file body back to the tool.

### Why this tool first

This is the lowest-risk partial-edit primitive. It is deterministic and much easier to validate than arbitrary patches.

### Proposed input

```json
{
  "path": "src/main.rs",
  "replacements": [
    {
      "old": "let mode = \"dev\";",
      "new": "let mode = \"prod\";"
    }
  ],
  "expected_replacements": 1,
  "allow_multiple_occurrences": false
}
```

### Proposed parameters

| Field | Type | Required | Notes |
|---|---|---|---|
| `path` | `string` | yes | Workspace-relative or absolute path inside workspace |
| `replacements` | `array` | yes | One or more replacement operations |
| `replacements[].old` | `string` | yes | Existing text to match |
| `replacements[].new` | `string` | yes | Replacement text |
| `expected_replacements` | `integer` | no | Optional total count guard |
| `allow_multiple_occurrences` | `boolean` | no | Default `false`; fail on ambiguity by default |

### Proposed output

```json
{
  "path": "src/main.rs",
  "applied_replacements": 1,
  "bytes_written": 18,
  "changed": true
}
```

### Implementation notes

- Read the target file.
- Validate every requested replacement.
- Reject zero-match and ambiguous-match cases unless explicitly allowed.
- Write back only after all validations succeed.
- Generate approval preview using the existing diff flow.

### Permission model

- Action type: write
- Default permission: approval required

### Library requirement

No new crate required.

Use:

- `std::fs`
- existing `similar` crate for diff previews

## `fs_apply_patch`

### Purpose

Apply a structured line-oriented patch to an existing file so the agent can make multi-location edits without re-uploading the whole file.

### Why we need it

`fs_replace_in_file` will not cover:

- several nearby edits in one tool call
- insertions/deletions around line ranges
- edits where the agent knows the intended diff better than an exact string replacement

### Proposed input

```json
{
  "path": "src/main.rs",
  "patch": "@@ -10,3 +10,4 @@\n let value = old();\n+log::info!(\"updated\");\n return value;\n",
  "format": "unified",
  "dry_run": false
}
```

### Proposed parameters

| Field | Type | Required | Notes |
|---|---|---|---|
| `path` | `string` | yes | Target file |
| `patch` | `string` | yes | Unified diff hunk body or full patch text |
| `format` | `string` | no | Initially only `unified` |
| `dry_run` | `boolean` | no | Validate and preview without writing |

### Proposed output

```json
{
  "path": "src/main.rs",
  "applied": true,
  "dryRun": false,
  "hunksApplied": 1,
  "bytesWritten": 31
}
```

### Implementation notes

- Restrict scope to a single target file per call.
- Apply patches in-memory first.
- Fail if the patch does not apply cleanly.
- Return structured validation errors for rejected hunks.
- Generate unified diff preview for approval.

### Permission model

- Action type: write
- Default permission: approval required

### Library requirement

Recommended: add `diffy`.

Reason:

- it provides patch creation and patch application primitives
- it is documented and designed specifically for diff/patch manipulation
- it keeps patch application logic out of custom handwritten code

Fallback:

- implement a restricted custom hunk applier

Recommendation:

- do not write a custom patch engine unless `diffy` proves insufficient

## `fs_glob_files`

### Purpose

Return files matching one or more glob patterns under a base directory.

### Example use cases

- `**/*.test.ts`
- `src/**/*.svelte`
- `docs/**/agent*.md`

### Proposed input

```json
{
  "base_path": "src-ui",
  "patterns": ["**/*.svelte", "**/*.ts"],
  "exclude": ["**/node_modules/**", "**/*.test.ts"],
  "max_depth": 8,
  "limit": 200,
  "respect_ignore_files": true
}
```

### Proposed parameters

| Field | Type | Required | Notes |
|---|---|---|---|
| `base_path` | `string` | no | Defaults to workspace root |
| `patterns` | `array[string]` | yes | One or more include patterns |
| `exclude` | `array[string]` | no | Optional exclude patterns |
| `max_depth` | `integer` | no | Bound traversal cost |
| `limit` | `integer` | no | Bound result size |
| `respect_ignore_files` | `boolean` | no | Default `true` |
| `include_directories` | `boolean` | no | Default `false` |

### Proposed output

```json
{
  "basePath": "src-ui",
  "count": 42,
  "truncated": false,
  "matches": ["src/lib/App.svelte", "src/lib/stores/agentStore.ts"]
}
```

### Implementation notes

- Use workspace-safe base path resolution.
- Respect `.gitignore` and `.ignore` by default.
- Return normalized workspace-relative paths.
- Bound traversal using `max_depth` and `limit`.

### Permission model

- Action type: read/search
- Default permission: auto-run

### Library requirement

Recommended: add `ignore` and `globset`.

Reason:

- `ignore` gives recursive walking with `.gitignore` and related filters
- `globset` gives cross-platform glob matching and multiple-pattern matching
- this is more maintainable than a custom walker plus ad hoc filtering

Alternative considered:

- `globwalk`

Why not preferred:

- it is useful, but its crate page currently states it is in maintenance mode and suggests newer users may prefer other options

## `fs_stat_path`

### Purpose

Return filesystem metadata without reading file contents.

### Example use cases

- check file size before reading
- inspect whether a path is a file, directory, or symlink
- show timestamps for debugging/generated files
- inspect permissions or readonly state

### Proposed input

```json
{
  "path": "src-tauri/Cargo.toml",
  "follow_symlinks": true
}
```

### Proposed parameters

| Field | Type | Required | Notes |
|---|---|---|---|
| `path` | `string` | yes | Target path |
| `follow_symlinks` | `boolean` | no | Default `true` |

### Proposed output

```json
{
  "path": "src-tauri/Cargo.toml",
  "exists": true,
  "fileType": "file",
  "sizeBytes": 1584,
  "readonly": false,
  "isSymlink": false,
  "modifiedAt": "2026-04-04T12:00:00Z",
  "accessedAt": "2026-04-04T12:10:00Z",
  "createdAt": null,
  "permissions": {
    "unixMode": 420
  }
}
```

### Implementation notes

- Use `std::fs::metadata` or `std::fs::symlink_metadata`.
- Convert timestamps to ISO 8601.
- Include platform-safe fields only when available.
- Keep the contract stable by using nullable fields when the platform does not expose a timestamp.

### Permission model

- Action type: read/inspect
- Default permission: auto-run

### Library requirement

No new crate is required for the initial read-only version.

Optional future addition:

- `filetime` if we later want richer timestamp extraction or timestamp mutation APIs

## `fs_fuzzy_find_files`

### Purpose

Return ranked candidate paths for an approximate query when the exact filename or path is not known.

### Example use cases

- `agent store`
- `thread lock`
- `main svlt`
- `cargo toml`

### Proposed input

```json
{
  "query": "agent store",
  "base_path": "src-ui/src",
  "limit": 20,
  "respect_ignore_files": true
}
```

### Proposed parameters

| Field | Type | Required | Notes |
|---|---|---|---|
| `query` | `string` | yes | Fuzzy query |
| `base_path` | `string` | no | Defaults to workspace root |
| `limit` | `integer` | no | Default 20, max 100 |
| `respect_ignore_files` | `boolean` | no | Default `true` |
| `include_directories` | `boolean` | no | Default `false` |

### Proposed output

```json
{
  "query": "agent store",
  "count": 3,
  "matches": [
    { "path": "src/lib/stores/agentStore.ts", "score": 188 },
    { "path": "src/lib/stores/agentSessionStore.ts", "score": 165 },
    { "path": "src/lib/components/agent/AgentStorePanel.svelte", "score": 141 }
  ]
}
```

### Implementation notes

- First gather candidate paths using the same traversal substrate as `fs_glob_files`.
- Score only normalized workspace-relative paths.
- Prefer path-aware scoring over plain substring scoring.
- Cap candidate set and results to keep runtime predictable.

### Permission model

- Action type: read/search
- Default permission: auto-run

### Library requirement

Recommended: add `nucleo-matcher`.

Reason:

- more actively maintained and more recently tested than older fuzzy-matching crates
- path-aware and high-performance matching
- better Unicode handling

Fallback:

- `fuzzy-matcher`

Why not preferred:

- simpler API, but older and less recently updated

## Recommended Implementation Stack

### Reuse existing code

Keep using:

- `std::fs` for reads, writes, and metadata
- existing path traversal protections from current `fs_*` tools
- existing `similar` crate for approval diff previews
- existing telemetry/event pipeline

### Add these crates

| Crate | Proposed use | Recommendation |
|---|---|---|
| `ignore` | workspace walking with `.gitignore` support | yes |
| `globset` | compiling and applying glob patterns | yes |
| `diffy` | parsing and applying unified patches | yes |
| `nucleo-matcher` | ranked fuzzy path matching | yes |

### Avoid adding for now

| Crate | Why not now |
|---|---|
| `globwalk` | useful, but current docs indicate maintenance mode and suggest newer users may prefer alternatives |
| `filetime` | not needed for read-only metadata inspection |
| `fuzzy-matcher` | workable fallback, but weaker recommendation than `nucleo-matcher` |

## Permission And Runtime Changes Needed

The tool implementation itself is only part of the work. To integrate these cleanly, we should also extend the runtime model.

### `ActionType` additions

Recommended additions in `src-tauri/src/core/agent/types.rs`:

- `PatchFile { path: String, estimated_hunks: u32 }`
- `SearchWorkspace { query: String, base_path: String }`
- `InspectPath { path: String }`

Why:

- `fs_replace_in_file` and `fs_apply_patch` are more specific than generic `WriteFile`
- `fs_glob_files` and `fs_fuzzy_find_files` are better classified as workspace search, not file read
- `fs_stat_path` is metadata inspection, not content read

### Permission defaults

Recommended defaults:

- `PatchFile` -> `ApprovalRequired`
- `SearchWorkspace` -> `AutoRun`
- `InspectPath` -> `AutoRun`

### Approval previews

For write-like tools:

- `fs_replace_in_file`: preview unified diff
- `fs_apply_patch`: preview normalized patch or resulting diff

### Telemetry

Recommended telemetry behavior:

- `fs_replace_in_file` and `fs_apply_patch` should emit `FileTouched` with `write`
- `fs_stat_path`, `fs_glob_files`, and `fs_fuzzy_find_files` should be persisted as tool calls but should not add noisy per-file `FileTouched` entries

## Proposed Delivery Order

### Phase 1

- `fs_stat_path`
- `fs_glob_files`

Reason:

- both are read-only
- both immediately improve discovery and planning
- both reduce unnecessary `fs_read_file` calls

### Phase 2

- `fs_replace_in_file`

Reason:

- biggest practical improvement for safe edits
- simpler than arbitrary patch application

### Phase 3

- `fs_fuzzy_find_files`

Reason:

- high UX value once traversal substrate exists
- can reuse walker implementation from Phase 1

### Phase 4

- `fs_apply_patch`

Reason:

- highest complexity
- depends on deciding the exact diff/patch contract we want to support long term

## Testing Requirements

Each new tool should ship with:

- unit tests for schema parsing and path validation
- workspace boundary tests
- ambiguity/failure-path tests
- integration-style tests for permission inference

Tool-specific test focus:

- `fs_replace_in_file`: zero matches, multiple matches, exact count mismatch
- `fs_apply_patch`: clean apply, rejected hunk, dry-run mode
- `fs_glob_files`: include/exclude semantics, ignore-file behavior, limit truncation
- `fs_stat_path`: file vs directory vs symlink behavior
- `fs_fuzzy_find_files`: ranking stability and deterministic limits

## Open Decisions

These should be resolved before implementation starts:

1. Should `fs_replace_in_file` support regex, or stay exact-text only in v1?
2. Should `fs_apply_patch` accept only a single-file patch in v1?
3. Should globbing respect `.gitignore` by default, or make that opt-out?
4. Should fuzzy search rank only by path, or also include filename-weight boosts?
5. Do we want new `ActionType` variants now, or do we temporarily map these tools onto existing read/write categories?

## Recommended Decision Set

For the first implementation pass:

1. Keep `fs_replace_in_file` exact-text only.
2. Keep `fs_apply_patch` single-file only.
3. Respect `.gitignore` and `.ignore` by default.
4. Rank fuzzy matches on normalized relative paths, with filename boost.
5. Add the new `ActionType` variants now instead of overloading `ReadFile` and `WriteFile`.

## External References

These references were used to evaluate implementation options:

- `ignore`: https://docs.rs/ignore/latest/ignore/
- `ignore::WalkBuilder`: https://docs.rs/ignore/latest/ignore/struct.WalkBuilder.html
- `globset`: https://docs.rs/globset/latest/globset/
- `diffy`: https://docs.rs/diffy/latest/diffy/
- `nucleo-matcher`: https://docs.rs/nucleo-matcher/latest/nucleo_matcher/
- `globwalk`: https://docs.rs/crate/globwalk/latest
- `fuzzy-matcher`: https://docs.rs/crate/fuzzy-matcher/latest
- `filetime`: https://docs.rs/crate/filetime/latest

## Implementation Follow-up

- `docs/filesystem-tools-implementation-plan.md` - TDD-first phased implementation plan for all proposed filesystem tools
