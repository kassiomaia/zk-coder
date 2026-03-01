//! Claude API client for AI interactions.
//!
//! This module provides a client for interacting with Anthropic's Claude API.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Claude API client.
pub struct ClaudeClient {
    /// HTTP client
    client: Client,
    /// API key
    api_key: String,
    /// Model to use
    model: String,
}

impl ClaudeClient {
    /// Creates a new Claude client.
    #[must_use]
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
        }
    }

    /// Creates a client with a specific model.
    #[must_use]
    pub fn with_model(api_key: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }

    /// Completes a prompt with the Claude API.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or the response is invalid.
    pub async fn complete(&self, system: &str, user: &str) -> Result<String, ClaudeError> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: format!("{system}\n\n{user}"),
            }],
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(ClaudeError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ClaudeError::Api(format!(
                "API returned {status}: {body}"
            )));
        }

        let claude_response = response
            .json::<ClaudeResponse>()
            .await
            .map_err(ClaudeError::Http)?;

        claude_response
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| ClaudeError::Api("No content in response".to_string()))
    }

    /// Completes a prompt with system message separately.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or the response is invalid.
    pub async fn complete_with_system(
        &self,
        system: &str,
        user: &str,
    ) -> Result<String, ClaudeError> {
        let request = ClaudeRequestWithSystem {
            model: self.model.clone(),
            max_tokens: 4096,
            system: system.to_string(),
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: user.to_string(),
            }],
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(ClaudeError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ClaudeError::Api(format!(
                "API returned {status}: {body}"
            )));
        }

        let claude_response = response
            .json::<ClaudeResponse>()
            .await
            .map_err(ClaudeError::Http)?;

        claude_response
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| ClaudeError::Api("No content in response".to_string()))
    }
}

/// Request to the Claude API.
#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

/// Request with separate system message.
#[derive(Debug, Serialize)]
struct ClaudeRequestWithSystem {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ClaudeMessage>,
}

/// Message in a Claude request.
#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

/// Response from the Claude API.
#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

/// Content block in a Claude response.
#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: String,
}

/// Errors from the Claude API.
#[derive(Debug, Error)]
pub enum ClaudeError {
    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    /// API error
    #[error("API error: {0}")]
    Api(String),
    /// JSON parsing error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_client_creation() {
        let client = ClaudeClient::new("test-key");
        assert_eq!(client.model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_claude_client_with_model() {
        let client = ClaudeClient::with_model("test-key", "claude-opus-4");
        assert_eq!(client.model, "claude-opus-4");
    }
}
