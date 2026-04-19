//! Plan-quality validation and pure planning predicates.
//!
//! These helpers operate on the serde-JSON shape that
//! `agent_plan_subtasks` receives. They have no `AppHandle`,
//! no repository, and no filesystem dependency — they are
//! pure functions over subtask descriptors and are fully
//! unit-testable in isolation.
//!
//! Extracted from `infrastructure/agent/runtime/permissions.rs`
//! in the permissions-decomposition refactor (Phase 1).

pub(crate) mod plan_validation;
