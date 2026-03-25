//! Application layer for chat functionality.
//!
//! This module provides the service layer that orchestrates business logic
//! between the core domain and the storage infrastructure.

mod service;

pub use service::ChatServiceImpl;
