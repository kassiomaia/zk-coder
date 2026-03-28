//! Provides the application layer for chat functionality.
//!
//! This module contains the service layer that orchestrates the business logic
//! between the core chat domain and the underlying storage infrastructure.

mod service;

#[cfg(test)]
mod service_tests {
    include!("service_tests.rs");
}

pub use service::ChatServiceImpl;
