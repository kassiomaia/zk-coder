//! Application-layer tool use cases.
//!
//! This layer sits between the `Tool` adapters in
//! `infrastructure/tools/` and the core domain models. Use cases here
//! contain the orchestration logic that used to leak into the tool
//! future (e.g. rendering PDFs, mapping legacy content types onto
//! typed blocks).

pub mod pdf;
