# Rust Warnings Audit and Cleanup

**Date**: 2026-04-19  
**Status**: In Progress  
**Initial Count**: 43 warnings  
**Current Count**: 19 warnings  
**Fixed**: 24 warnings (56% reduction)

## Summary

This document tracks the systematic review and cleanup of Rust compiler warnings in the zk-coder project. The warnings were identified using `cargo check --lib` and categorized for prioritized fixing.

## Progress

### ✅ Completed (24 warnings fixed)

#### PR 1: Remove Unused Imports and Constants (4 warnings)
- Removed unused import `PdfThemeRepository` in `pdf_theme.rs`
- Removed unused constant `MAX_TOKENS` in `context_manager.rs`
- Removed unused constants `MIN_CACHE_TTL_SECS` and `MAX_CACHE_TTL_SECS` in `web_search/cache.rs`

**Impact**: Low-risk cleanup, no behavior changes

#### PR 2: Add Missing Documentation (15 warnings)
Added documentation for:
- Trait `ConfigRepository` in `config/repositories.rs`
- Modules `harness_events` and `yolo` in `agent/execution/mod.rs`
- 4 struct fields in `memory_refresh.rs`
- Associated function `new` in `yolo.rs`
- 2 struct fields in `tool_credentials.rs`
- 2 struct fields and 3 functions in `workspace_policy.rs`

**Impact**: Improved code documentation, no behavior changes, satisfies `missing_docs = "warn"` lint

#### PR 3: Suppress Intentional Dead Code (5 warnings)
Added `#[allow(dead_code)]` annotations to:
- Serde fields in document tools (`title`, `template` in DOCX; `position` in PPTX; `template`, `theme` in PPTX start)
- Defensive error variants (`DisabledByFlag`, `UpstreamError` tuple field in web_search errors)

**Rationale**: These items are part of the wire format/API or kept for defensive programming and should not be removed.

**Impact**: Suppressed legitimate warnings for intentional dead code

### 🔍 Remaining Warnings (19 warnings)

The remaining warnings require careful review and decisions about the codebase architecture. They fall into the following categories:

#### 1. Test-Only Functions (7 warnings)
**Location**: `src/commands/llm/helpers.rs`

Items:
- `FEEDBACK_SYSTEM_PROMPT` (constant)
- `FEEDBACK_LIMIT` (constant)
- `resolve_model_id` (function) - **Note**: Duplicate of function in `orchestrator/loop_runner.rs`
- `convert_messages` (function)
- `convert_to_core_message` (function)
- `build_feedback_summary` (function)
- `build_system_prompt_with_feedback` (function)

**Analysis**: These functions are used only in the `tests.rs` module (which is conditionally compiled with `#[cfg(test)]`). The constants are used by the unused functions.

**Options**:
1. Move these functions to a `#[cfg(test)]` module or the tests file itself
2. Remove the duplicate `resolve_model_id` and keep only the one in orchestrator
3. If these are intended for future production use, add `#[allow(dead_code)]`

**Recommendation**: Move to test module and remove the duplicate `resolve_model_id`

#### 2. Unused Runtime Methods (6 warnings)

##### a. Agent Service Methods (5 warnings)
**Location**: `src/application/agent/service.rs:115`

Methods: `runtime`, `clear_execution`, `execution_repo`, `trace_repo`, `to_agents_session_id`

**Analysis**: These appear to be accessor methods that may be planned for future features or testing.

**Options**:
1. Remove if truly unused and no plans to use
2. Add `#[allow(dead_code)]` if planned for future features
3. Add unit tests that exercise these methods

##### b. Session Scope Method
**Location**: `src/infrastructure/agent/runtime/session_scope.rs:80`

Method: `try_mark_decomposed`

##### c. Context Method
**Location**: `src/infrastructure/agent/runtime/context.rs:800`

Method: `retryable_unchanged`

##### d. Permissions Trait
**Location**: `src/infrastructure/agent/runtime/permissions.rs:47`

Trait: `PendingActionExt`

**Recommendation**: Review each for removal or future-use annotation

#### 3. Database Helper Functions (3 warnings)
**Location**: `src/infrastructure/sqlite/repositories/config/models.rs`

Functions:
- `save_single_model` (line 46)
- `delete_provider_models` (line 69)
- `save_models` (line 83)

**Analysis**: `save_models` IS used in tests. These appear to be low-level helpers that are called by higher-level repository methods.

**Note**: The comment in the code says "for internal use" and "for tests".

**Recommendation**: Move to `#[cfg(test)]` or verify they're needed for production

#### 4. Conversion Functions (3 warnings)
**Location**: `src/application/llm/service/conversions.rs:24`

Functions: `convert_to_core_role`, `convert_to_core_content_block`, `convert_to_core_message`

**Analysis**: Likely helper functions for converting between different message representations.

**Recommendation**: Either use them, remove them, or add `#[allow(dead_code)]` if planned for future use

#### 5. Unused Struct Fields (2 warnings)
**Location**: `src/infrastructure/agent/runtime/session_scope.rs:30`

Fields: `session_id`, `decomposed_subtasks`

**Analysis**: These fields are defined but never read. Either they should be used or removed.

**Recommendation**: Review if these are part of a future feature or dead code

#### 6. Other Unused Items (3 warnings)

##### a. File System Function
**Location**: `src/infrastructure/tools/fs/read_file.rs:146`

Function: `workspace_relative_path`

##### b. Google Search Builder Method
**Location**: `src/infrastructure/tools/search/web_search/provider/google.rs:64`

Method: `with_base_url`

**Recommendation**: Review for removal or future-use annotation

#### 7. Visibility Issue (1 warning) ⚠️
**Location**: `src/infrastructure/agent/runtime/permissions.rs:86`

Issue: Type `RuntimeSessionScope` is more private than the item `process_tool_calls_with_permissions`

**Analysis**: This is a Rust visibility error - a public function is using a private type in its signature.

**Options**:
1. Make `RuntimeSessionScope` public
2. Make `process_tool_calls_with_permissions` private (or `pub(crate)`)
3. Refactor to use a public trait or type

**Recommendation**: This needs careful consideration of the module's public API design

## Recommendations for Next Steps

### Immediate Actions (PR 4)
1. **Move test-only code**: Relocate functions in `llm/helpers.rs` to test module
2. **Remove duplicates**: Remove duplicate `resolve_model_id` function
3. **Mark defensive code**: Add `#[allow(dead_code)]` to session_scope fields if they're part of planned serialization

### Review Required (PR 5)
1. **Audit runtime methods**: Decide which methods to keep, remove, or test
2. **Database helpers**: Clarify their role and either move to tests or document why they're needed
3. **Conversion functions**: Either use them or remove them

### API Design (PR 6)
1. **Fix visibility issue**: Review and fix the `RuntimeSessionScope` visibility problem

## Lessons Learned

1. **Test-only code**: Functions used only in tests should be in test modules or marked with `#[cfg(test)]`
2. **Serde fields**: Optional API fields should be marked with `#[allow(dead_code)]` to avoid false positives
3. **Duplicate code**: Watch for duplicated helper functions across modules
4. **Visibility**: Public APIs should only reference public types
5. **Documentation**: The `missing_docs` lint is valuable for public APIs

## Impact

- **Code Quality**: ✅ Improved (better documentation, less clutter)
- **Build Performance**: ➖ Neutral (same compilation)
- **Maintenance**: ✅ Improved (clearer which code is actually used)
- **API Clarity**: ✅ Improved (better documentation)

## Next Review

The remaining 19 warnings should be reviewed with the development team to decide on the appropriate action for each item based on:
- Planned features and roadmap
- Testing strategy
- Public API design principles
- Code maintenance philosophy
