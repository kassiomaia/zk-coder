//! Application layer for agent services.
//!
//! This module provides the concrete implementation of the AgentService trait
//! defined in the core layer. It coordinates between the domain entities and
//! the infrastructure layer (session store, approval queue, runtime, registry).

pub mod conversions;
pub mod execution;
pub mod planner;
pub mod read_models;
pub mod service;

pub use service::AgentServiceImpl;
