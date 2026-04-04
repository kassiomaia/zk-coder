# Filesystem Tools Implementation Plan

**Status**: DRAFT
**Last reviewed**: 2026-04-04
**Purpose**: Step-by-step TDD-first implementation plan for the new filesystem tools and their agent/runtime integration

---

## Scope

This plan covers implementation of the proposed filesystem tools:

- `fs_stat_path`
- `fs_glob_files`
- `fs_replace_in_file`
- `fs_fuzzy_find_files`
- `fs_apply_patch`

The goal is to deliver them incrementally while following the same pattern for every tool:

1. add or confirm dependencies
2. write tests first
3. implement the minimum code to pass tests
4. refactor
5. integrate into permissions, telemetry, and agent availability
6. verify through focused test coverage

## Delivery Strategy

### Working pattern

Each tool will follow this exact loop:

1. dependency readiness
2. tool contract test
3. runtime integration test
4. implementation
5. agent exposure verification
6. regression coverage

### Definition of done for each tool

A tool is considered complete only when all of the following are true:

- its Rust implementation exists under `src-tauri/src/infrastructure/tools/fs/`
- it is registered in `FsCategory`
- it appears in `ToolRegistry::definitions()`
- `infer_action_type()` classifies it correctly
- permissions resolve correctly for `Trusted`, `Untrusted`, and `Restricted`
- approval previews exist for write-like operations
- telemetry behavior is covered by tests
- the tool is available to agents through the registry and allowed tool lists
- its tests pass

## Dependency Plan

## Phase 0: Dependency Preparation

### Goal

Prepare the supporting crates before implementing the tools that need them.

### Planned dependency additions

| Crate | Use | Tools blocked on it |
|---|---|---|
| `ignore` | recursive walking with `.gitignore` support | `fs_glob_files`, `fs_fuzzy_find_files` |
| `globset` | glob compilation and matching | `fs_glob_files` |
| `nucleo-matcher` | fuzzy path ranking | `fs_fuzzy_find_files` |
| `diffy` | patch parsing and application | `fs_apply_patch` |

### TDD tasks

1. Add dependency presence tests indirectly by writing integration tests that require the crate-backed behavior.
2. Only add the dependency immediately before the first tool that needs it.
3. Run `cargo test` after each dependency addition to catch version conflicts early.

### Validation

- `cargo test`
- `cargo clippy`

## Shared Backend Work

## Phase 1: Shared Runtime And Type Extensions

### Goal

Create the common infrastructure needed by several tools.

### Planned changes

1. Extend `ActionType` with:
   - `PatchFile { path: String, estimated_hunks: u32 }`
   - `SearchWorkspace { query: String, base_path: String }`
   - `InspectPath { path: String }`
2. Update `PermissionResolver::default_for_action()`.
3. Extend `infer_action_type()` to recognize all new filesystem tools.
4. Extend tool call/result feedback helpers for the new tool names.
5. Decide and document `FileTouched` behavior for metadata/search tools versus write tools.

### TDD tasks

Write tests first for:

- `ActionType` mapping from each new tool name
- permission defaults for each new action type
- feedback text generation for each new tool
- registry definitions containing new tools once registered

### Exit criteria

- shared classification and permission tests exist and pass

## Tool Rollout Order

The rollout order is chosen to minimize risk and maximize immediate capability gain.

1. `fs_stat_path`
2. `fs_glob_files`
3. `fs_replace_in_file`
4. `fs_fuzzy_find_files`
5. `fs_apply_patch`

## Tool Plan: `fs_stat_path`

### Why first

- no new dependency required
- read-only
- simplest new tool
- useful for later tools and better agent planning

### Tests to write first

1. metadata schema test
2. valid file path returns file metadata
3. valid directory path returns directory metadata
4. symlink behavior with `follow_symlinks=true`
5. symlink behavior with `follow_symlinks=false`
6. missing path returns structured error
7. workspace traversal rejection
8. `infer_action_type("fs_stat_path", ...)` returns `InspectPath`
9. permission resolver maps `InspectPath` to `AutoRun`

### Implementation tasks

1. Add `fs/stat_path.rs`
2. Reuse existing workspace-safe path resolution pattern
3. Return:
   - existence
   - file type
   - size
   - readonly
   - symlink flag
   - timestamps
   - platform-safe permission representation
4. Register in `FsCategory`
5. Update tool feedback and action inference

### Verification

- targeted `cargo test` for `fs::stat_path`
- full `cargo test`

## Tool Plan: `fs_glob_files`

### Why second

- read-only
- unlocks precise discovery
- provides traversal substrate reusable by fuzzy search

### Dependency step

Add:

- `ignore`
- `globset`

### Tests to write first

1. schema test
2. single-pattern match
3. multiple-pattern match
4. exclude-pattern filtering
5. `max_depth` enforcement
6. `limit` and truncation behavior
7. workspace traversal rejection for `base_path`
8. ignore-file behavior when `respect_ignore_files=true`
9. include-directories behavior
10. normalized relative path output
11. `infer_action_type("fs_glob_files", ...)` returns `SearchWorkspace`
12. permission resolver maps `SearchWorkspace` to `AutoRun`
13. tool registry definitions include `fs_glob_files`

### Implementation tasks

1. Add `fs/glob_files.rs`
2. Build a shared walker utility around `ignore::WalkBuilder`
3. Compile include/exclude sets with `globset`
4. Normalize output to workspace-relative paths
5. Return deterministic ordering
6. Register in `FsCategory`
7. Update feedback helpers

### Verification

- targeted tests for globbing and ignore behavior
- full `cargo test`
- `cargo clippy`

## Tool Plan: `fs_replace_in_file`

### Why third

- high-value partial editing
- no new dependency required
- safer than patch application

### Tests to write first

1. schema test
2. single exact replacement succeeds
3. multiple replacements in one call succeed atomically
4. zero matches fails
5. multiple matches fail when `allow_multiple_occurrences=false`
6. multiple matches succeed when explicitly allowed
7. `expected_replacements` mismatch fails
8. unchanged replacement returns `changed=false` or a clear no-op contract
9. workspace traversal rejection
10. diff preview generation test for approval flow
11. `infer_action_type("fs_replace_in_file", ...)` returns `PatchFile` or `WriteFile`, depending on final classification
12. permission resolver requires approval
13. successful execution emits/persists file-touch write semantics

### Implementation tasks

1. Add `fs/replace_in_file.rs`
2. Read target content into memory
3. Validate all replacements before writing
4. Generate before/after diff preview
5. Write only once after all replacements succeed
6. Register in `FsCategory`
7. Update:
   - action inference
   - approval preview generation
   - tool feedback
   - file-touch persistence logic if needed

### Verification

- targeted replacement tests
- runtime/approval tests
- full `cargo test`

## Tool Plan: `fs_fuzzy_find_files`

### Why fourth

- depends on having a stable traversal substrate
- valuable once the agent needs path recovery and discovery

### Dependency step

Add:

- `nucleo-matcher`

### Tests to write first

1. schema test
2. exact path query ranks exact match highest
3. filename-only query returns path candidates
4. typo-tolerant query returns expected candidate
5. deterministic limit behavior
6. ignore-file behavior with shared walker
7. directory inclusion toggle
8. stable ordering for equal scores
9. workspace traversal rejection for `base_path`
10. `infer_action_type("fs_fuzzy_find_files", ...)` returns `SearchWorkspace`
11. permission resolver maps it to `AutoRun`
12. registry definition visibility

### Implementation tasks

1. Add `fs/fuzzy_find_files.rs`
2. Reuse walker utility from `fs_glob_files`
3. Rank normalized relative paths with `nucleo-matcher`
4. Add filename boost on top of path score if needed
5. Register in `FsCategory`
6. Update tool feedback

### Verification

- targeted fuzzy-ranking tests
- full `cargo test`
- `cargo clippy`

## Tool Plan: `fs_apply_patch`

### Why last

- highest complexity
- most approval-sensitive write tool
- easiest to get subtly wrong

### Dependency step

Add:

- `diffy`

### Tests to write first

1. schema test
2. clean single-hunk apply succeeds
3. clean multi-hunk apply succeeds
4. dry-run validates without writing
5. invalid patch format fails
6. patch conflict fails cleanly
7. patch against wrong file content fails
8. empty patch fails
9. workspace traversal rejection
10. approval preview generation test
11. `infer_action_type("fs_apply_patch", ...)` returns `PatchFile`
12. permission resolver requires approval
13. successful execution emits/persists write telemetry
14. registry definition visibility

### Implementation tasks

1. Add `fs/apply_patch.rs`
2. Restrict v1 to one file per call
3. Parse and apply patch in-memory with `diffy`
4. Support `dry_run`
5. Generate normalized diff preview for approval
6. Register in `FsCategory`
7. Update:
   - action inference
   - approval previews
   - tool feedback

### Verification

- targeted patch-application tests
- runtime approval tests
- full `cargo test`
- `cargo clippy`

## Agent Availability Plan

Ensuring the tools are implemented is not enough. They must also be reachable by agents.

### Required integration points

For each tool:

1. register it in `FsCategory`
2. verify `ToolRegistry::definitions()` includes it
3. verify the code agent’s allowed-tool configuration includes it
4. verify any task-mode filtering still allows it in the intended modes
5. verify the frontend/runtime can display its tool calls and results without special casing

### Tests to add

1. registry test for expected tool names
2. agent catalog/config test for allowed tools
3. task mode allowed-tools test if mode-specific filtering applies
4. runtime tool execution test through the orchestrated path, not only direct unit invocation

## Suggested Work Breakdown By PR

### PR 1

- dependency prep for `ignore` and `globset`
- shared runtime/type changes
- `fs_stat_path`
- `fs_glob_files`

### PR 2

- `fs_replace_in_file`
- approval preview integration for partial edits

### PR 3

- dependency add for `nucleo-matcher`
- `fs_fuzzy_find_files`

### PR 4

- dependency add for `diffy`
- `fs_apply_patch`

### PR 5

- cleanup
- regression tests
- documentation updates to tool reference
- changelog update

## Test Execution Discipline

For every phase:

1. write failing tests first
2. confirm the tests fail for the expected reason
3. implement the minimal change to pass
4. refactor while preserving green tests
5. run the focused test module
6. run the broader backend suite
7. run `cargo clippy`

Recommended command sequence during implementation:

```bash
cargo test <targeted_test_name>
cargo test
cargo clippy
```

If dependency resolution or compilation requires network access, request approval at that point and keep dependency additions isolated so failures are easy to diagnose.

## Documentation Updates Required During Implementation

These docs must be updated as tools land:

- `docs/tools-and-agent-integration.md`
- `docs/filesystem-tools-proposal.md`
- `CHANGELOG.md`

When the full implementation is complete:

- mark this plan `COMPLETE`
- move it to `docs/archive/` if it is no longer the active implementation guide

## Risks

### Patch complexity risk

`fs_apply_patch` can become a time sink if the accepted patch contract is too broad.

Mitigation:

- keep v1 single-file only
- require clean apply semantics
- use `diffy`

### Search consistency risk

`fs_glob_files` and `fs_fuzzy_find_files` may drift if they use different walkers.

Mitigation:

- build one shared workspace traversal helper

### Permission drift risk

New tools may accidentally inherit the wrong action classification.

Mitigation:

- add explicit `infer_action_type()` tests per tool
- add permission resolver tests per new action type

## Immediate Next Step

Start with Phase 0 and PR 1:

1. add shared classification tests
2. add `ignore` and `globset`
3. write tests for `fs_stat_path`
4. implement `fs_stat_path`
5. write tests for `fs_glob_files`
6. implement `fs_glob_files`

Do not begin `fs_replace_in_file` until the read-side tools and shared runtime extensions are green.
