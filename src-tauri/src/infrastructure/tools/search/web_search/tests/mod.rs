//! Test modules for the web search tool.
//!
//! Individual test files live alongside this module, one per concern,
//! to keep any single file well inside the repository's soft limit.

mod args_tests;
mod cache_integration_tests;
mod credentials_tests;
mod error_mapping_tests;
mod locale_integration_tests;
mod policy_tests;
mod provider_google_tests;
mod provider_mock_tests;
mod tool_execute_tests;
