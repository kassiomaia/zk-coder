//! AI module for the Coder application.
//!
//! This module provides AI client implementations for interacting
//! with language models like Claude.

mod claude;

pub use claude::{ClaudeClient, ClaudeError};
