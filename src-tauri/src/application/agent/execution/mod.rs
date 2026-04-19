//! Provides application services for observing and managing agent execution.
//!
//! This module contains the components responsible for coordinating the different
//! phases of an agent's execution, handling state transitions, and tracing the
//! runtime behavior of the agent.

pub mod bootstrap;
pub mod context_phase;
pub mod coordinator;
pub mod deps;
/// Event types emitted by the agent harness during execution.
pub mod harness_events;
pub mod llm_phase;
pub mod memory_refresh;
pub mod message_ids;
pub mod park;
pub mod replace_drift;
pub mod response_phase;
pub mod scope_drift;
pub mod state;
pub mod step_gate;
pub mod termination;
pub mod tool_phase;
pub mod tracing;
pub mod utils;
/// YOLO mode provider integration for the agent execution.
pub mod yolo;

#[cfg(test)]
#[path = "bootstrap_tests.rs"]
mod bootstrap_tests;

#[cfg(test)]
#[path = "prompt_stability_tests.rs"]
mod prompt_stability_tests;

#[cfg(test)]
#[path = "runtime_enforcement_tests.rs"]
mod runtime_enforcement_tests;
