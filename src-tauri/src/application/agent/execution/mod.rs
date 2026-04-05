//! Provides application services for observing and managing agent execution.
//!
//! This module contains the components responsible for coordinating the different
//! phases of an agent's execution, handling state transitions, and tracing the
//! runtime behavior of the agent.

pub mod bootstrap;
pub mod context_phase;
pub mod coordinator;
pub mod deps;
pub mod llm_phase;
pub mod response_phase;
pub mod state;
pub mod step_gate;
pub mod termination;
pub mod tool_phase;
pub mod tracing;
pub mod utils;
pub mod yolo;

#[cfg(test)]
#[path = "bootstrap_tests.rs"]
mod bootstrap_tests;
