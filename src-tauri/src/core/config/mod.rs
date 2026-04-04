//! Defines the domain entities for LLM provider and model configuration.
//!
//! This module provides the core data structures for managing LLM providers,
//! models, and their default assignments for various operations. These entities
//! form the contract for how the application's configuration is structured.

pub mod repositories;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use repositories::{
    ApiKeyRepository, ConfigDomainError, ConfigRepository, ManualModelRepository, ModelRepository,
    OperationDefaultsRepository, ProviderRepository, RepositoryResult as ConfigRepositoryResult,
};

/// Represents the type of an API provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ApiType {
    /// An OpenAI-compatible API.
    #[default]
    OpenAI,
    /// The Anthropic API.
    Anthropic,
    /// A Google AI (Gemini) API.
    Google,
}

impl ApiType {
    /// Converts the `ApiType` to a string slice for database storage.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Google => "google",
        }
    }

    /// Creates an `ApiType` from a string slice.
    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s {
            "anthropic" => Self::Anthropic,
            "google" => Self::Google,
            _ => Self::OpenAI,
        }
    }
}

/// Defines the source for a provider's list of models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModelsSource {
    /// Automatically fetch models from the provider's API.
    #[default]
    Auto,
    /// Use only a manually configured list of models.
    Manual,
    /// Combine API-fetched models with the manual list.
    Hybrid,
}

impl ModelsSource {
    /// Converts the `ModelsSource` to a string slice for database storage.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
            Self::Hybrid => "hybrid",
        }
    }

    /// Creates a `ModelsSource` from a string slice.
    #[must_use]
    pub fn from_str(s: &str) -> Self {
        match s {
            "manual" => Self::Manual,
            "hybrid" => Self::Hybrid,
            _ => Self::Auto,
        }
    }
}

/// Represents the configuration for an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    /// The unique identifier for the provider.
    pub id: String,
    /// The display name of the provider.
    pub name: String,
    /// An icon identifier for the provider (e.g., "AC" for Anthropic).
    pub icon: String,
    /// A flag indicating whether this is a custom, user-added provider.
    pub is_custom: bool,
    /// The type of the provider's API.
    pub api_type: ApiType,
    /// The custom base URL for the API, if applicable.
    pub base_url: Option<String>,
    /// A flag indicating whether the provider has been configured with an API key.
    pub is_configured: bool,
    /// The source for the provider's list of models.
    #[serde(default)]
    pub models_source: ModelsSource,
    /// The timestamp of when the provider was created.
    pub created_at: DateTime<Utc>,
    /// The timestamp of when the provider was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Provider {
    /// Creates a new built-in provider configuration.
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
            models_source: ModelsSource::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new custom provider configuration.
    #[must_use]
    pub fn custom(name: &str, icon: &str, api_type: ApiType, base_url: Option<String>) -> Self {
        let id = Self::generate_custom_id(name);
        Self::custom_with_id(&id, name, icon, api_type, base_url)
    }

    /// Creates a new custom provider configuration with a specific ID.
    #[must_use]
    pub fn custom_with_id(
        id: &str,
        name: &str,
        icon: &str,
        api_type: ApiType,
        base_url: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            icon: icon.to_string(),
            is_custom: true,
            api_type,
            base_url,
            is_configured: false,
            models_source: ModelsSource::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Generates a unique ID for a custom provider based on its name.
    fn generate_custom_id(name: &str) -> String {
        format!(
            "custom-{}",
            name.to_lowercase()
                .replace(' ', "-")
                .replace(|c: char| !c.is_alphanumeric() && c != '-', "")
        )
    }

    /// Marks the provider as configured and updates its timestamp.
    #[must_use]
    pub fn with_configured(mut self) -> Self {
        self.is_configured = true;
        self.updated_at = Utc::now();
        self
    }
}

/// A set of flags representing the capabilities of a model.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilities {
    /// A flag indicating whether the model supports analysis tasks.
    #[serde(default = "default_true")]
    pub analysis: bool,
    /// A flag indicating whether the model supports code generation.
    #[serde(default = "default_true")]
    pub code: bool,
    /// A flag indicating whether the model supports vision or image input.
    #[serde(default)]
    pub vision: bool,
    /// A flag indicating whether the model supports audio input or transcription.
    #[serde(default)]
    pub audio: bool,
    /// A flag indicating whether the model supports tips and refinements.
    #[serde(default = "default_true")]
    pub tips: bool,
    /// A flag indicating whether the model supports code review.
    #[serde(default = "default_true")]
    pub review: bool,
}

/// Returns `true` by default for JSON deserialization.
const fn default_true() -> bool {
    true
}

impl ModelCapabilities {
    /// Infers the capabilities of a model based on patterns in its name.
    #[must_use]
    pub fn infer_from_name(model_name: &str) -> Self {
        let name = model_name.to_lowercase();
        Self {
            analysis: true,
            code: true,
            vision: name.contains("vision") || name.contains("gpt-4o") || name.contains("claude-3"),
            audio: name.contains("whisper") || name.contains("tts") || name.contains("audio"),
            tips: true,
            review: true,
        }
    }
}

/// A manually configured model entry for a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualModelEntry {
    /// The name of the model (e.g., "gpt-4", "claude-3-opus-20240229").
    pub name: String,
    /// The capabilities of the model.
    pub capabilities: ModelCapabilities,
}

impl ManualModelEntry {
    /// Creates a new `ManualModelEntry` with the given name and capabilities.
    #[must_use]
    pub fn new(name: impl Into<String>, capabilities: ModelCapabilities) -> Self {
        Self {
            name: name.into(),
            capabilities,
        }
    }

    /// Creates a `ManualModelEntry` with capabilities inferred from the model's name.
    #[must_use]
    pub fn infer_from_name(name: impl Into<String>) -> Self {
        let name_str = name.into();
        let capabilities = ModelCapabilities::infer_from_name(&name_str);
        Self::new(name_str, capabilities)
    }
}

/// A request to add or update a manual model for a provider.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualModelRequest {
    /// The ID of the provider to add the model to.
    pub provider_id: String,
    /// The name of the model.
    pub name: String,
    /// The capabilities of the model (optional; will be inferred if not provided).
    pub capabilities: Option<ModelCapabilities>,
}

/// Represents an LLM model available from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    /// The unique identifier for the model.
    pub id: String,
    /// The ID of the provider this model belongs to.
    pub provider_id: String,
    /// The display name of the model.
    pub name: String,
    /// The capabilities of the model.
    pub capabilities: ModelCapabilities,
    /// The timestamp of when the model was last fetched from the API.
    pub last_fetched: DateTime<Utc>,
}

impl Model {
    /// Creates a new `Model` with the given parameters.
    #[must_use]
    pub fn new(provider_id: &str, name: &str, capabilities: ModelCapabilities) -> Self {
        let id = format!(
            "{}-{}",
            provider_id,
            name.to_lowercase().replace([' ', '.'], "-")
        );
        Self {
            id,
            provider_id: provider_id.to_string(),
            name: name.to_string(),
            capabilities,
            last_fetched: Utc::now(),
        }
    }
}

/// The type of operation for which a model can be assigned as the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationType {
    /// Analysis and quick questions.
    Analysis,
    /// Code generation.
    Code,
    /// Image and vision tasks.
    Vision,
    /// Audio transcription.
    Audio,
    /// Tips and refinements.
    Tips,
    /// Code review.
    Review,
}

impl OperationType {
    /// Converts the `OperationType` to a string slice for database storage.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Analysis => "analysis",
            Self::Code => "code",
            Self::Vision => "vision",
            Self::Audio => "audio",
            Self::Tips => "tips",
            Self::Review => "review",
        }
    }

    /// Creates an `OperationType` from a string slice.
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

    /// Returns a slice containing all `OperationType` variants.
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

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents the default model assignment for a specific operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationDefault {
    /// The type of the operation.
    pub operation: OperationType,
    /// The ID of the model assigned as the default (can be `None` if the model was deleted).
    pub model_id: Option<String>,
}

/// A request to save a provider's configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProviderRequest {
    /// The ID of the provider (optional for new providers).
    pub id: Option<String>,
    /// The display name of the provider.
    pub name: String,
    /// The icon identifier for the provider.
    pub icon: String,
    /// The type of the provider's API.
    pub api_type: ApiType,
    /// The custom base URL for the API, if applicable.
    pub base_url: Option<String>,
}

/// The result of an API key validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    /// A flag indicating whether the validation was successful.
    pub valid: bool,
    /// A message describing the result of the validation.
    pub message: String,
}

impl ValidationResult {
    /// Creates a successful `ValidationResult`.
    #[must_use]
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            valid: true,
            message: message.into(),
        }
    }

    /// Creates a failed `ValidationResult`.
    #[must_use]
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            valid: false,
            message: message.into(),
        }
    }
}

/// The result of a model fetching operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchModelsResult {
    /// A flag indicating whether the fetch was successful.
    pub success: bool,
    /// The list of model names that were fetched.
    pub models: Vec<String>,
    /// An optional message, typically for errors.
    pub message: Option<String>,
}

impl FetchModelsResult {
    /// Creates a successful `FetchModelsResult`.
    #[must_use]
    pub const fn success(models: Vec<String>) -> Self {
        Self {
            success: true,
            models,
            message: None,
        }
    }

    /// Creates a failed `FetchModelsResult`.
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
