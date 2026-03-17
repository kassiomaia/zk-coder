//! NLP (Natural Language Processing) module for the Coder application.
//!
//! This module provides intent recognition, context management,
//! multi-intent detection, ambiguity resolution, and code understanding.

pub mod ambiguity;
pub mod ambiguity_resolver;
pub mod chat;
pub mod classifier;
pub mod code_analyzer;
pub mod code_understanding;
pub mod config;
pub mod context;
pub mod context_manager;
pub mod execution_planner;
pub mod intent;
pub mod llm_gateway;
pub mod multi_detector;
pub mod multi_intent;
pub mod nl_to_code;
pub mod patterns;
#[cfg(test)]
pub mod test_utils;

// Re-export commonly used types
pub use ambiguity::{
    Ambiguity, AmbiguitySeverity, AmbiguitySuggestion, AmbiguityType, ClarificationOption,
    ClarificationRequest, ResolutionType, ResolvedAmbiguity,
};
pub use ambiguity_resolver::AmbiguityResolver;
pub use classifier::{ClassifierError, IntentClassifier};
pub use code_analyzer::{AnalyzerError, CodeAnalyzer};
pub use code_understanding::{
    ClassInfo, CodeStructure, CodeUnderstanding, ComplexityMetrics, FunctionInfo,
    ProgrammingLanguage,
};
pub use context::{ContextMessage, ContextWindow, ConversationContext, MessageRole};
pub use context_manager::{ContextError, ContextManager};
pub use execution_planner::{ExecutionPlan, ExecutionPlanner};
pub use intent::{Intent, IntentClassification, IntentParams};
pub use llm_gateway::{NlpLlmError, NlpLlmGateway};
pub use multi_detector::{DetectorError, MultiIntentDetector};
pub use multi_intent::{DetectedIntent, IntentRelationship, MultiIntentResult, TextSpan};
pub use nl_to_code::{CodeConcept, NLToCodeMapper};
pub use patterns::IntentPatterns;

// Chat types (MessageRole is defined in context module, re-exported here)
pub use chat::{
    ChatMessage, Conversation, ConversationSettings, MessageMetadata, MessageStatus, Thread,
    ThreadMetadata, ThreadPriority,
};

// Config types for LLM provider management
pub use config::{
    ApiType, FetchModelsResult, Model, ModelCapabilities, OperationDefault, OperationType,
    Provider, SaveProviderRequest, ValidationResult,
};
