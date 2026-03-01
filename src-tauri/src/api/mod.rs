//! API validation and model fetching for LLM providers.
//!
//! This module provides functions to validate API keys and fetch models
//! from various LLM providers (OpenAI, Anthropic).

use reqwest::Client;
use serde::Deserialize;

use crate::nlp::config::{ApiType, FetchModelsResult, Model, ModelCapabilities, ValidationResult};

/// Default timeout for API requests (in seconds).
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Creates a configured HTTP client.
fn create_client() -> Result<Client, String> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

// =============================================================================
// OpenAI API
// =============================================================================

/// Validates an OpenAI API key.
///
/// # Errors
///
/// Returns an error string if the validation fails.
pub async fn validate_openai_key(
    api_key: &str,
    base_url: Option<&str>,
) -> Result<ValidationResult, String> {
    let client = create_client()?;
    let url = format!(
        "{}/models",
        base_url.unwrap_or("https://api.openai.com/v1")
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        Ok(ValidationResult::success(
            "API key validated successfully",
        ))
    } else {
        let status = response.status();
        Ok(ValidationResult::failure(format!(
            "Validation failed: {}",
            status
        )))
    }
}

/// Fetches available models from OpenAI.
///
/// # Errors
///
/// Returns an error string if the fetch fails.
pub async fn fetch_openai_models(
    api_key: &str,
    base_url: Option<&str>,
) -> Result<FetchModelsResult, String> {
    let client = create_client()?;
    let url = format!(
        "{}/models",
        base_url.unwrap_or("https://api.openai.com/v1")
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Ok(FetchModelsResult::failure(format!(
            "API error: {}",
            response.status()
        )));
    }

    let api_response = response
        .json::<OpenAiModelsResponse>()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let models: Vec<Model> = api_response
        .data
        .into_iter()
        .filter(|m| {
            // Filter out non-chat models
            let id = m.id.to_lowercase();
            id.contains("gpt") || id.contains("o1") || id.contains("o3")
        })
        .map(|m| {
            let caps = ModelCapabilities::infer_from_name(&m.id);
            Model::new("openai", &m.id, caps)
        })
        .collect();

    let model_names: Vec<String> = models.iter().map(|m| m.name.clone()).collect();

    Ok(FetchModelsResult::success(model_names))
}

#[derive(Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModel>,
}

#[derive(Deserialize)]
struct OpenAiModel {
    id: String,
}

// =============================================================================
// Anthropic API
// =============================================================================

/// Validates an Anthropic API key.
///
/// # Errors
///
/// Returns an error string if the validation fails.
pub async fn validate_anthropic_key(
    api_key: &str,
    base_url: Option<&str>,
) -> Result<ValidationResult, String> {
    let client = create_client()?;
    let url = format!(
        "{}/v1/messages",
        base_url.unwrap_or("https://api.anthropic.com")
    );

    // Anthropic doesn't have a validation endpoint, so we make a minimal request
    let response = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-3-haiku-20240307",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "Hi"}]
        }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    // 200 or 400 (rate limit) means key is valid
    // 401 means invalid key
    if status.as_u16() == 401 {
        Ok(ValidationResult::failure("Invalid API key"))
    } else {
        Ok(ValidationResult::success(
            "API key validated successfully",
        ))
    }
}

/// Gets known Anthropic models (Anthropic doesn't have a models endpoint).
///
/// # Errors
///
/// Returns an error string if the operation fails.
pub fn get_anthropic_models() -> FetchModelsResult {
    let models = vec![
        "claude-opus-4-20250514",
        "claude-sonnet-4-20250514",
        "claude-3-5-sonnet-20241022",
        "claude-3-5-haiku-20241022",
        "claude-3-opus-20240229",
        "claude-3-sonnet-20240229",
        "claude-3-haiku-20240307",
    ];

    FetchModelsResult::success(models.iter().map(|s| s.to_string()).collect())
}

// =============================================================================
// Unified API
// =============================================================================

/// Validates an API key with the appropriate provider.
///
/// # Errors
///
/// Returns an error string if the validation fails.
pub async fn validate_api_key(
    api_type: ApiType,
    api_key: &str,
    base_url: Option<&str>,
) -> Result<ValidationResult, String> {
    match api_type {
        ApiType::OpenAI => validate_openai_key(api_key, base_url).await,
        ApiType::Anthropic => validate_anthropic_key(api_key, base_url).await,
    }
}

/// Tests connection to a custom provider.
///
/// # Errors
///
/// Returns an error string if the connection test fails.
pub async fn test_connection(
    api_type: ApiType,
    api_key: &str,
    base_url: &str,
) -> Result<ValidationResult, String> {
    match api_type {
        ApiType::OpenAI => validate_openai_key(api_key, Some(base_url)).await,
        ApiType::Anthropic => validate_anthropic_key(api_key, Some(base_url)).await,
    }
}

/// Fetches models from the appropriate provider.
///
/// # Errors
///
/// Returns an error string if the fetch fails.
pub async fn fetch_models(
    api_type: ApiType,
    api_key: &str,
    base_url: Option<&str>,
) -> Result<FetchModelsResult, String> {
    match api_type {
        ApiType::OpenAI => fetch_openai_models(api_key, base_url).await,
        ApiType::Anthropic => Ok(get_anthropic_models()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_anthropic_models() {
        let result = get_anthropic_models();
        assert!(result.success);
        assert!(!result.models.is_empty());
        assert!(result.models.iter().any(|m| m.contains("claude")));
    }

    #[test]
    fn test_infer_capabilities() {
        let caps = ModelCapabilities::infer_from_name("gpt-4-vision-preview");
        assert!(caps.vision);

        let caps = ModelCapabilities::infer_from_name("claude-3-opus");
        assert!(caps.vision);
    }
}
