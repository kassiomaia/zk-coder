//! NLP application layer.
//!
//! This module contains orchestration and workflow logic built on top of core
//! NLP entities and infrastructure-backed gateways/repositories.

pub mod ambiguity_resolver;
pub mod classifier;
pub mod code_analyzer;
pub mod context_manager;
pub mod execution_planner;
pub mod gateway;
pub mod multi_detector;
pub mod nl_to_code;
#[cfg(test)]
pub mod test_utils;

pub use ambiguity_resolver::{AmbiguityResolver, ResolverError};
pub use classifier::{ClassifierError, IntentClassifier};
pub use code_analyzer::{AnalyzerError, CodeAnalyzer};
pub use context_manager::{ContextError, ContextManager};
pub use execution_planner::{ExecutionPlan, ExecutionPlanner};
pub use gateway::{NlpLlmError, NlpLlmGateway};
pub use multi_detector::{DetectorError, MultiIntentDetector};
pub use nl_to_code::{CodeConcept, NLToCodeMapper};
