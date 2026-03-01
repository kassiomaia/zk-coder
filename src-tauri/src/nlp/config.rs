//! LLM Provider configuration data types.
//!
//! This module provides data structures for managing LLM providers,
//! models, and operation defaults.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// API provider type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ApiType {
    /// OpenAI-compatible API
    #[default]
    OpenAI,
    /// Anthropic API
    Anthropic,
    /// Google OpenAI-compatible API (Gemini)
    Google,
}

impl ApiType {
    /// Converts the API type to a string for database storage.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Google => "google",
        }
    }

    /// Creates an API type from a string.
    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s {
            "anthropic" => Self::Anthropic,
            "google" => Self::Google,
            _ => Self::OpenAI,
        }
    }
}

/// LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    /// Unique identifier for the provider
    pub id: String,
    /// Display name of the provider
    pub name: String,
    /// Icon identifier (e.g., "AC" for Anthropic)
    pub icon: String,
    /// Whether this is a custom (user-added) provider
    pub is_custom: bool,
    /// Type of API (OpenAI-compatible or Anthropic)
    pub api_type: ApiType,
    /// Custom base URL (for custom providers)
    pub base_url: Option<String>,
    /// Whether the provider has been configured with an API key
    pub is_configured: bool,
    /// When the provider was created
    pub created_at: DateTime<Utc>,
    /// When the provider was last updated
    pub updated_at: DateTime<Utc>,
}

impl Provider {
    /// Creates a new built-in provider.
    #[must_use]
    pub fn builtin(id: &str, name: &str, icon: &str, api_type: ApiType) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            icon: icon.to_string(),
            is_custom: false,
            api_type,
            base_url: None,
            is_configured: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new custom provider.
    #[must_use]
    pub fn custom(name: &str, icon: &str, api_type: ApiType, base_url: Option<String>) -> Self {
        let id = Self::generate_custom_id(name);
        let now = Utc::now();
        Self {
            id,
            name: name.to_string(),
            icon: icon.to_string(),
            is_custom: true,
            api_type,
            base_url,
            is_configured: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Generates a unique ID for a custom provider.
    fn generate_custom_id(name: &str) -> String {
        format!(
            "custom-{}",
            name.to_lowercase()
                .replace(' ', "-")
                .replace(|c: char| !c.is_alphanumeric() && c != '-', "")
        )
    }

    /// Marks the provider as configured.
    #[must_use]
    pub fn with_configured(mut self) -> Self {
        self.is_configured = true;
        self.updated_at = Utc::now();
        self
    }
}

/// Model capability flags.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilities {
    /// Supports analysis tasks
    #[serde(default = "default_true")]
    pub analysis: bool,
    /// Supports code generation
    #[serde(default = "default_true")]
    pub code: bool,
    /// Supports vision/image input
    #[serde(default)]
    pub vision: bool,
    /// Supports audio input/transcription
    #[serde(default)]
    pub audio: bool,
    /// Supports tips and refinements
    #[serde(default = "default_true")]
    pub tips: bool,
    /// Supports code review
    #[serde(default = "default_true")]
    pub review: bool,
}

const fn default_true() -> bool {
    true
}

impl ModelCapabilities {
    /// Creates capabilities based on model name patterns.
    #[must_use]
    pub fn infer_from_name(model_name: &str) -> Self {
        let name = model_name.to_lowercase();
        Self {
            analysis: true,
            code: true,
            vision: name.contains("vision")
                || name.contains("gpt-4o")
                || name.contains("claude-3"),
            audio: name.contains("whisper") || name.contains("tts") || name.contains("audio"),
            tips: true,
            review: true,
        }
    }
}

/// LLM model available from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    /// Unique identifier for the model
    pub id: String,
    /// ID of the provider this model belongs to
    pub provider_id: String,
    /// Display name of the model
    pub name: String,
    /// Model capabilities
    pub capabilities: ModelCapabilities,
    /// When the model was last fetched from the API
    pub last_fetched: DateTime<Utc>,
}

impl Model {
    /// Creates a new model.
    #[must_use]
    pub fn new(provider_id: &str, name: &str, capabilities: ModelCapabilities) -> Self {
        let id = format!("{}-{}", provider_id, name.to_lowercase().replace([' ', '.'], "-"));
        Self {
            id,
            provider_id: provider_id.to_string(),
            name: name.to_string(),
            capabilities,
            last_fetched: Utc::now(),
        }
    }
}

/// Operation type for model assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationType {
    /// Analysis and quick questions
    Analysis,
    /// Code generation
    Code,
    /// Image and vision tasks
    Vision,
    /// Audio transcription
    Audio,
    /// Tips and refinements
    Tips,
    /// Code review
    Review,
}

impl OperationType {
    /// Converts the operation type to a string for database storage.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Analysis => "analysis",
            Self::Code => "code",
            Self::Vision => "vision",
            Self::Audio => "audio",
            Self::Tips => "tips",
            Self::Review => "review",
        }
    }

    /// Creates an operation type from a string.
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "analysis" => Some(Self::Analysis),
            "code" => Some(Self::Code),
            "vision" => Some(Self::Vision),
            "audio" => Some(Self::Audio),
            "tips" => Some(Self::Tips),
            "review" => Some(Self::Review),
            _ => None,
        }
    }

    /// Returns all operation types.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Analysis,
            Self::Code,
            Self::Vision,
            Self::Audio,
            Self::Tips,
            Self::Review,
        ]
    }
}

/// Default model assignment for an operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationDefault {
    /// The operation type
    pub operation: OperationType,
    /// The model ID assigned as default
    pub model_id: String,
}

/// Request to save a provider.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProviderRequest {
    /// Provider ID (optional for new providers)
    pub id: Option<String>,
    /// Display name
    pub name: String,
    /// Icon identifier
    pub icon: String,
    /// API type
    pub api_type: ApiType,
    /// Custom base URL
    pub base_url: Option<String>,
}

/// Result of API key validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    /// Whether the validation succeeded
    pub valid: bool,
    /// Message describing the result
    pub message: String,
}

impl ValidationResult {
    /// Creates a successful validation result.
    #[must_use]
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            valid: true,
            message: message.into(),
        }
    }

    /// Creates a failed validation result.
    #[must_use]
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            valid: false,
            message: message.into(),
        }
    }
}

/// Result of model fetching.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchModelsResult {
    /// Whether the fetch succeeded
    pub success: bool,
    /// List of model names fetched
    pub models: Vec<String>,
    /// Optional message (for errors)
    pub message: Option<String>,
}

impl FetchModelsResult {
    /// Creates a successful fetch result.
    #[must_use]
    pub fn success(models: Vec<String>) -> Self {
        Self {
            success: true,
            models,
            message: None,
        }
    }

    /// Creates a failed fetch result.
    #[must_use]
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            models: Vec::new(),
            message: Some(message.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_builtin() {
        let provider = Provider::builtin("anthropic", "Anthropic", "AC", ApiType::Anthropic);
        assert_eq!(provider.id, "anthropic");
        assert!(!provider.is_custom);
        assert!(!provider.is_configured);
    }

    #[test]
    fn test_provider_custom() {
        let provider = Provider::custom("My Provider", "MP", ApiType::OpenAI, None);
        assert!(provider.id.starts_with("custom-"));
        assert!(provider.is_custom);
    }

    #[test]
    fn test_provider_configured() {
        let provider =
            Provider::builtin("openai", "OpenAI", "OA", ApiType::OpenAI).with_configured();
        assert!(provider.is_configured);
    }

    #[test]
    fn test_api_type_conversion() {
        assert_eq!(ApiType::from_str("openai"), ApiType::OpenAI);
        assert_eq!(ApiType::from_str("anthropic"), ApiType::Anthropic);
        assert_eq!(ApiType::from_str("unknown"), ApiType::OpenAI);
    }

    #[test]
    fn test_operation_type_conversion() {
        assert_eq!(OperationType::from_str("code"), Some(OperationType::Code));
        assert_eq!(OperationType::from_str("unknown"), None);
    }

    #[test]
    fn test_capabilities_infer() {
        let caps = ModelCapabilities::infer_from_name("gpt-4-vision-preview");
        assert!(caps.vision);
        assert!(!caps.audio);
    }
}
