//! Application services for execution observability.

pub mod bootstrap;
pub mod coordinator;
pub mod context_phase;
pub mod deps;
pub mod llm_phase;
pub mod response_phase;
pub mod state;
pub mod step_gate;
pub mod termination;
pub mod tool_phase;
pub mod tracing;
pub mod utils;

#[cfg(test)]
#[path = "bootstrap_tests.rs"]
mod bootstrap_tests;
