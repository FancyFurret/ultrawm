use crate::config::Config;
use log::{debug, error};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AiClientError {
    #[error("AI features are not enabled in config")]
    NotEnabled,
    #[error("AI API URL is not configured")]
    NoApiUrl,
    #[error("AI API key is not configured")]
    NoApiKey,
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("API returned an error: {0}")]
    ApiError(String),
    #[error("Failed to parse response: {0}")]
    ParseError(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

/// Generic AI client for making chat completion requests.
/// This is layout-agnostic and can be used for any AI functionality.
pub struct AiClient {
    client: Client,
    api_url: String,
    api_key: String,
    model: String,
    temperature: f32,
}

impl AiClient {
    pub fn from_config() -> Result<Self, AiClientError> {
        let ai_config = Config::ai();

        if !ai_config.enabled {
            return Err(AiClientError::NotEnabled);
        }

        if ai_config.api_url.is_empty() {
            return Err(AiClientError::NoApiUrl);
        }

        if ai_config.api_key.is_empty() {
            return Err(AiClientError::NoApiKey);
        }

        Ok(Self {
            client: Client::new(),
            api_url: ai_config.api_url,
            api_key: ai_config.api_key,
            model: ai_config.model,
            temperature: ai_config.temperature,
        })
    }

    /// Send a chat completion request to the AI API.
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String, AiClientError> {
        let request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            temperature: self.temperature,
        };

        debug!("Sending AI request to: {}", self.api_url);

        let response = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            error!("AI API error ({}): {}", status, error_text);
            return Err(AiClientError::ApiError(format!(
                "Status {}: {}",
                status, error_text
            )));
        }

        let completion: ChatCompletionResponse = response.json().await.map_err(|e| {
            AiClientError::ParseError(format!("Failed to parse chat completion response: {}", e))
        })?;

        completion
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| AiClientError::ParseError("No choices in response".to_string()))
    }
}

/// Strip markdown code blocks if the AI included them despite instructions
pub fn strip_markdown_code_block(text: &str) -> String {
    let trimmed = text.trim();

    // Check for ```yaml or ```yml or just ``` at the start
    if let Some(rest) = trimmed.strip_prefix("```") {
        // Skip the language tag (yaml, yml, etc.) on the first line
        let after_lang = if let Some(newline_pos) = rest.find('\n') {
            &rest[newline_pos + 1..]
        } else {
            rest
        };

        // Remove trailing ```
        if let Some(content) = after_lang.strip_suffix("```") {
            return content.trim().to_string();
        }
        return after_lang.trim().to_string();
    }

    trimmed.to_string()
}
