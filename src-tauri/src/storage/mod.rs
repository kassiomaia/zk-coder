//! Storage module for the Coder application.
//!
//! This module provides data persistence using SQLite.

mod chat_store;
mod config_store;
mod context_store;

pub use chat_store::{ChatStore, ChatStoreError};
pub use config_store::{ConfigStore, ConfigStoreError};
pub use context_store::{ContextStore, StoreError};
