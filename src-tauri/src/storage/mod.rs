//! Storage module for the Coder application.
//!
//! This module provides data persistence using SQLite.

mod context_store;

pub use context_store::{ContextStore, StoreError};
