//! Production web search tool backed by the Google Custom Search JSON API.
//!
//! The feature contract lives in `docs/tools/web_search/spec.md`. The phased
//! implementation plan is in `docs/tools/web_search/plan.md`.
//!
//! Phase 1 ships the argument / output types and their pure validation logic.
//! No HTTP, DB, provider, or registry wiring is present at this phase.

pub mod provider;
pub mod types;

#[cfg(test)]
mod tests;
