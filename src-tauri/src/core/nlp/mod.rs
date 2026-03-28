//! Defines the core domain for Natural Language Processing (NLP) tasks.
//!
//! This module contains the fundamental entities, services, and repositories
//! related to NLP, such as intent classification, ambiguity detection, and
//! code understanding.

pub mod ambiguity;
pub mod code_understanding;
pub mod entities;
pub mod intent;
pub mod multi_intent;
pub mod patterns;
pub mod repositories;

pub use ambiguity::{
    Ambiguity, AmbiguitySeverity, AmbiguitySuggestion, AmbiguityType, ClarificationOption,
    ClarificationRequest, ResolutionType, ResolvedAmbiguity,
};
pub use code_understanding::{
    ClassInfo, CodeStructure, CodeUnderstanding, ComplexityMetrics, ComplexityRating, FunctionInfo,
    ProgrammingLanguage,
};
pub use entities::{
    ContextMessage, ContextMetadata, ContextWindow, ConversationContext, FileInfo, MessageRole,
    SymbolKind, SymbolReference,
};
pub use intent::{Intent, IntentClassification, IntentParams};
pub use multi_intent::{DetectedIntent, IntentRelationship, MultiIntentResult, TextSpan};
pub use patterns::IntentPatterns;
pub use repositories::{ContextWindowRepository, NlpDomainError};
