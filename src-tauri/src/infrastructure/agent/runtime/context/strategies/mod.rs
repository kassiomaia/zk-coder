//! Concrete [`crate::core::agent::context::ContextCompactionStrategy`]
//! implementations used by the agent runtime.
//!
//! Today only [`token_threshold::TokenThresholdStrategy`] ships; new
//! strategies (fixed-window, semantic-chunk, …) land as additional
//! modules here without touching the dispatcher.

pub mod token_threshold;

pub use token_threshold::TokenThresholdStrategy;
