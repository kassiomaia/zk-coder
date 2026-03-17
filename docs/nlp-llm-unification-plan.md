# NLP LLM Unification Plan

Date: March 17, 2026
Status: Proposed
Owner: Backend

## 1. Problem Statement

The project currently has two LLM integration paths:

1. Chat path uses the generic provider-agnostic layer in `src-tauri/src/llm/*` with config-driven provider/model resolution.
2. NLP path uses `src-tauri/src/ai/claude.rs` directly from `src-tauri/src/commands/nlp.rs`.

This creates:

1. Architectural duplication.
2. Provider lock-in in NLP.
3. Inconsistent runtime behavior between chat and NLP.
4. A hardcoded development key path (`new_dev("dev-api-key")`) in NLP state initialization.

## 2. Goals

1. Make NLP use the same generic LLM layer used by chat.
2. Remove direct dependency on `ClaudeClient` in NLP components.
3. Remove `src-tauri/src/ai/claude.rs` and `src-tauri/src/ai/mod.rs` after migration.
4. Keep current NLP behavior and API contracts stable for frontend callers.
5. Use configured providers/models from `ConfigStore`, not hardcoded runtime config.

## 3. Non-Goals

1. No tool-calling implementation in this effort.
2. No NLP prompt redesign beyond minimal compatibility adjustments.
3. No frontend API changes unless strictly necessary for error messaging.
4. No changes to chat streaming behavior.

## 4. Current-State Inventory

## Generic LLM path (already in use by chat)

1. `src-tauri/src/llm/mod.rs`
2. `src-tauri/src/llm/openai.rs`
3. `src-tauri/src/llm/anthropic.rs`
4. `src-tauri/src/commands/llm.rs`

## NLP-specific legacy path

1. `src-tauri/src/ai/claude.rs`
2. `src-tauri/src/commands/nlp.rs` creates `ClaudeClient` directly
3. `src-tauri/src/nlp/classifier.rs` depends on `ClaudeClient`
4. `src-tauri/src/nlp/context_manager.rs` depends on `ClaudeClient`
5. `src-tauri/src/nlp/multi_detector.rs` depends on `ClaudeClient`

## 5. Target Architecture

NLP components should depend on a small NLP-facing gateway abstraction backed by `crate::llm::LlmClient`.

Suggested shape:

1. New module: `src-tauri/src/nlp/llm_gateway.rs`
2. New trait for NLP needs only:
   - `complete(system: &str, user: &str) -> Result<String, NlpLlmError>`
3. Concrete implementation:
   - Resolves provider/model from `ConfigStore`
   - Creates `LlmClient` via `create_client(...)`
   - Sends a non-streaming `ChatRequest`
4. NLP modules (`classifier`, `context_manager`, `multi_detector`) depend on this trait, not provider-specific clients.

This keeps NLP decoupled from provider payload differences while reusing the common LLM layer.

## 6. Model Selection Strategy for NLP

Primary recommendation:

1. Use operation default `analysis` for NLP model resolution.
2. Fallback to operation default `code` only when `analysis` is not configured.
3. Return explicit configuration errors when neither is available.

Rationale:

1. NLP tasks (intent classification, multi-intent decomposition, summarization) are analysis-oriented.
2. Preserves user/admin control through existing operation defaults.

## 7. Error Handling Strategy

Map LLM/runtime errors into NLP-friendly behavior:

1. `classify_intent`:
   - Keep existing pattern-first fast path.
   - If LLM call fails, return deterministic fallback classification (`unknown`, confidence 0.5) with log warning.
2. `detect_multi_intent`:
   - If LLM call fails, fallback to single-intent heuristic path.
3. `context_manager` compression summary:
   - If LLM summarization fails, keep uncompressed recent messages and continue safely.
4. Surface actionable configuration errors only when there is no usable fallback.

## 8. Phased Implementation Plan

### Phase 0: Baseline and Safety Net

1. Add/refresh tests for current NLP behavior:
   - `IntentClassifier` pattern path
   - `MultiIntentDetector` conjunction path fallback
   - `ContextManager` context retrieval without AI dependency
2. Document expected fallback behavior for AI failures.

Deliverable:

1. Green baseline test suite before architecture changes.

### Phase 1: Introduce NLP LLM Gateway

1. Add `src-tauri/src/nlp/llm_gateway.rs`.
2. Define `NlpLlmGateway` trait and concrete config-driven implementation.
3. Add model/provider resolution helper:
   - prefer `analysis`
   - fallback `code`
4. Add typed error enum for NLP LLM gateway.

Deliverable:

1. Gateway compiles and can execute a minimal completion call via generic `llm` layer.

### Phase 2: Refactor NLP Components to Gateway

1. Refactor:
   - `src-tauri/src/nlp/classifier.rs`
   - `src-tauri/src/nlp/context_manager.rs`
   - `src-tauri/src/nlp/multi_detector.rs`
2. Replace `Arc<ClaudeClient>` with `Arc<dyn NlpLlmGateway + Send + Sync>`.
3. Update error enums to remove `ClaudeError` coupling.

Deliverable:

1. NLP modules compile without importing `crate::ai::ClaudeClient`.

### Phase 3: Refactor NLP State Initialization

1. Update `src-tauri/src/commands/nlp.rs`:
   - remove `new(api_key: &str, ...)`
   - remove `new_dev("dev-api-key")`
   - initialize with config-aware gateway
2. Update app bootstrap in `src-tauri/src/lib.rs`:
   - stop using `NlpState::new_dev(...)`
   - build `NlpState` from config DB path and context DB path

Deliverable:

1. NLP is config-driven and no hardcoded API key path remains.

### Phase 4: Remove Legacy AI Module

1. Delete:
   - `src-tauri/src/ai/claude.rs`
   - `src-tauri/src/ai/mod.rs`
2. Remove `pub mod ai;` from `src-tauri/src/lib.rs`.
3. Remove leftover imports/usages.

Deliverable:

1. No references to `crate::ai::*` in codebase.

### Phase 5: Hardening and Regression

1. Add integration tests for:
   - missing provider config
   - missing API key
   - missing analysis default with code fallback
2. Validate startup behavior with clean DB and partially configured DB.
3. Run:
   - `cargo fmt`
   - `cargo clippy`
   - `cargo test`

Deliverable:

1. Regression-safe migration complete.

## 9. File-Level Change Plan

## New files

1. `src-tauri/src/nlp/llm_gateway.rs`

## Modified files

1. `src-tauri/src/nlp/mod.rs` (export gateway types)
2. `src-tauri/src/nlp/classifier.rs`
3. `src-tauri/src/nlp/context_manager.rs`
4. `src-tauri/src/nlp/multi_detector.rs`
5. `src-tauri/src/commands/nlp.rs`
6. `src-tauri/src/commands/config.rs` (optional helper reuse for analysis model resolution)
7. `src-tauri/src/lib.rs` (state initialization)

## Deleted files (final phase)

1. `src-tauri/src/ai/claude.rs`
2. `src-tauri/src/ai/mod.rs`

## 10. Compatibility and Rollout

No frontend contract changes are expected for this migration.

Rollout strategy:

1. Merge in phases with compile-safe checkpoints.
2. Keep fallback behavior in NLP commands to avoid user-facing regressions during provider outages.
3. Remove legacy module only after all NLP compilation and tests pass.

## 11. Risks and Mitigations

1. Risk: NLP output drift after provider/model change.
   - Mitigation: keep prompts unchanged initially; add golden behavioral tests for representative inputs.
2. Risk: missing config blocks NLP.
   - Mitigation: deterministic fallback paths for classification/detection; clear error for summarization when unavoidable.
3. Risk: startup failures due to strict initialization.
   - Mitigation: lazy gateway initialization or resilient initialization with deferred config errors.
4. Risk: hidden dependency on `ClaudeError` types.
   - Mitigation: replace with NLP-local error enum and map underlying `LlmError`.

## 12. Acceptance Criteria

1. No `crate::ai::ClaudeClient` imports remain.
2. No hardcoded NLP API keys remain.
3. NLP LLM calls run through `crate::llm::{create_client, LlmClient, ChatRequest}`.
4. `src-tauri/src/ai/claude.rs` and `src-tauri/src/ai/mod.rs` are removed.
5. `cargo test` passes.
6. `cargo clippy` passes under current lint profile.

## 13. Execution Checklist

1. Phase 0 tests in place.
2. Gateway module added.
3. NLP modules migrated.
4. NlpState/bootstrap migrated.
5. Legacy AI module deleted.
6. Full validation run complete.
7. Changelog entry added if behavior/user-visible error messages changed.

## 14. Task Breakdown (Parallel Execution)

This section decomposes phases into atomic tasks with explicit dependency and parallelization rules.

### Task IDs

1. `T0.1` Baseline NLP behavior tests (pattern paths + fallback behavior).
2. `T0.2` Baseline test harness cleanup (test fixtures/builders for NLP modules).
3. `T1.1` Create `nlp/llm_gateway.rs` trait + error types.
4. `T1.2` Implement config-driven gateway over `crate::llm` (provider/model/api key resolution + completion call).
5. `T1.3` Implement model resolution policy (`analysis` default, `code` fallback).
6. `T2.1` Refactor `IntentClassifier` to use gateway abstraction.
7. `T2.2` Refactor `MultiIntentDetector` to use gateway abstraction.
8. `T2.3` Refactor `ContextManager` summarization to use gateway abstraction.
9. `T2.4` Replace NLP module error types to remove `ClaudeError` coupling.
10. `T3.1` Refactor `commands/nlp.rs` (`NlpState` wiring, remove hardcoded dev key path).
11. `T3.2` Refactor app bootstrap in `lib.rs` to new NLP state initialization.
12. `T4.1` Delete legacy `ai` module (`ai/claude.rs`, `ai/mod.rs`, exports/imports cleanup).
13. `T5.1` Add integration tests for missing config/API key and fallback behavior.
14. `T5.2` Run final validation (`cargo fmt`, `cargo clippy`, `cargo test`) and fix regressions.
15. `T5.3` Documentation/changelog update for architectural migration and operational notes.

### Dependency Graph

1. `T0.1` depends on none.
2. `T0.2` depends on none.
3. `T1.1` depends on none.
4. `T1.2` depends on `T1.1`.
5. `T1.3` depends on `T1.1` (can be merged into `T1.2` if preferred).
6. `T2.1` depends on `T1.2` and `T1.3`.
7. `T2.2` depends on `T1.2` and `T1.3`.
8. `T2.3` depends on `T1.2` and `T1.3`.
9. `T2.4` depends on `T2.1`, `T2.2`, `T2.3`.
10. `T3.1` depends on `T2.4`.
11. `T3.2` depends on `T3.1`.
12. `T4.1` depends on `T3.2`.
13. `T5.1` depends on `T4.1`.
14. `T5.2` depends on `T5.1`.
15. `T5.3` depends on `T5.2`.

### Parallelization Lanes

1. Lane A (can start immediately): `T0.1`, `T0.2`, `T1.1`
2. Lane B (after gateway exists): `T1.2`, `T1.3`
3. Lane C (parallel refactors after gateway): `T2.1`, `T2.2`, `T2.3`
4. Lane D (consolidation): `T2.4`, `T3.1`, `T3.2`
5. Lane E (cleanup + quality): `T4.1`, `T5.1`, `T5.2`, `T5.3`

### Tasks That Can Be Executed Truly In Parallel

1. `T0.1` and `T0.2` can run in parallel.
2. `T2.1`, `T2.2`, and `T2.3` can run in parallel once `T1.2`/`T1.3` land.
3. `T5.1` test authoring can start while `T4.1` is in review, but should only merge after `T4.1`.

### Per-Task Commit Strategy

One commit per task is recommended to preserve reviewability and rollback granularity.

1. `test(nlp): add baseline behavior tests for classifier/detector/context` (`T0.1`)
2. `test(nlp): add shared fixtures for nlp module tests` (`T0.2`)
3. `feat(nlp): add llm gateway abstraction for nlp components` (`T1.1`)
4. `feat(nlp): implement config-driven llm gateway over generic llm layer` (`T1.2`)
5. `feat(nlp): add analysis->code model resolution policy` (`T1.3`)
6. `refactor(nlp): migrate intent classifier to llm gateway` (`T2.1`)
7. `refactor(nlp): migrate multi-intent detector to llm gateway` (`T2.2`)
8. `refactor(nlp): migrate context manager summarization to llm gateway` (`T2.3`)
9. `refactor(nlp): remove claude-specific error coupling from nlp modules` (`T2.4`)
10. `refactor(commands): rewire nlp state initialization to config-driven gateway` (`T3.1`)
11. `refactor(app): update bootstrap to new nlp initialization path` (`T3.2`)
12. `chore(ai): remove legacy claude ai module` (`T4.1`)
13. `test(nlp): add integration coverage for config and fallback scenarios` (`T5.1`)
14. `chore: run fmt clippy test and resolve regressions after nlp unification` (`T5.2`)
15. `docs: document nlp llm unification migration and operational notes` (`T5.3`)
