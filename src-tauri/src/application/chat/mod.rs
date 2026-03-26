//! Application layer for chat functionality.
//!
//! This module provides the service layer that orchestrates business logic
//! between the core domain and the storage infrastructure.

mod service;

#[cfg(test)]
mod service_tests {
    include!("service_tests.rs");
}

pub use service::ChatServiceImpl;
