use serde::Deserialize;

use crate::error::ApiError;

use super::openai_compat::{self, OpenAiCompatConfig};

pub const DEFAULT_BASE_URL: &str = openai_compat::DEFAULT_OPENROUTER_BASE_URL;

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct OpenRouterPricing {
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub completion: String,
    #[serde(default)]
    pub request: String,
    #[serde(default)]
    pub image: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct OpenRouterTopProvider {
    #[serde(default)]
    pub context_length: Option<u32>,
    #[serde(default)]
    pub max_completion_tokens: Option<u32>,
    #[serde(default)]
    pub is_moderated: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OpenRouterModel {
    pub id: String,
    #[serde(default)]
    pub canonical_slug: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub pricing: OpenRouterPricing,
    #[serde(default)]
    pub context_length: u32,
    #[serde(default)]
    pub top_provider: OpenRouterTopProvider,
    #[serde(default)]
    pub supported_parameters: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OpenRouterCatalogClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl OpenRouterCatalogClient {
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: api_key.into(),
            base_url: read_base_url(),
        }
    }

    pub fn from_env() -> Result<Self, ApiError> {
        let Some(api_key) = openai_compat::read_api_key_for_env_var("OPENROUTER_API_KEY")? else {
            return Err(ApiError::missing_credentials(
                "OpenRouter",
                &["OPENROUTER_API_KEY"],
            ));
        };
        Ok(Self::new(api_key))
    }

    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub async fn list_models(&self) -> Result<Vec<OpenRouterModel>, ApiError> {
        let response = self
            .http
            .get(models_user_endpoint(&self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?;
        let response = expect_success(response).await?;
        let mut payload = response.json::<ModelsResponse>().await?.data;
        payload.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(payload)
    }
}

#[must_use]
pub fn read_base_url() -> String {
    openai_compat::read_base_url(OpenAiCompatConfig::openrouter())
}

fn models_user_endpoint(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    let root = trimmed
        .strip_suffix("/chat/completions")
        .or_else(|| trimmed.strip_suffix("/models/user"))
        .or_else(|| trimmed.strip_suffix("/models"))
        .unwrap_or(trimmed);
    format!("{root}/models/user")
}

async fn expect_success(response: reqwest::Response) -> Result<reqwest::Response, ApiError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let body = response.text().await.unwrap_or_default();
    let parsed_error = serde_json::from_str::<ErrorEnvelope>(&body).ok();
    let retryable = matches!(status.as_u16(), 408 | 409 | 429 | 500 | 502 | 503 | 504);

    Err(ApiError::Api {
        status,
        error_type: parsed_error
            .as_ref()
            .and_then(|error| error.error.error_type.clone()),
        message: parsed_error
            .as_ref()
            .and_then(|error| error.error.message.clone()),
        body,
        retryable,
    })
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<OpenRouterModel>,
}

#[derive(Debug, Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    #[serde(rename = "type")]
    error_type: Option<String>,
    message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{models_user_endpoint, ModelsResponse};

    #[test]
    fn derives_models_endpoint_from_supported_base_urls() {
        assert_eq!(
            models_user_endpoint("https://openrouter.ai/api/v1"),
            "https://openrouter.ai/api/v1/models/user"
        );
        assert_eq!(
            models_user_endpoint("https://openrouter.ai/api/v1/"),
            "https://openrouter.ai/api/v1/models/user"
        );
        assert_eq!(
            models_user_endpoint("https://openrouter.ai/api/v1/chat/completions"),
            "https://openrouter.ai/api/v1/models/user"
        );
        assert_eq!(
            models_user_endpoint("https://openrouter.ai/api/v1/models/user"),
            "https://openrouter.ai/api/v1/models/user"
        );
    }

    #[test]
    fn parses_catalog_payloads_when_optional_pricing_fields_are_missing() {
        let payload = r#"{
            "data": [
                {
                    "id": "openai/gpt-4o",
                    "canonical_slug": "openai/gpt-4o-2025-04-01",
                    "name": "OpenAI: GPT-4o",
                    "description": "General-purpose model",
                    "context_length": 128000,
                    "pricing": {
                        "prompt": "0.000005",
                        "completion": "0.000015"
                    },
                    "top_provider": {
                        "context_length": 128000,
                        "max_completion_tokens": 16384,
                        "is_moderated": true
                    },
                    "supported_parameters": ["temperature"],
                    "architecture": {
                        "modality": "text->text"
                    }
                }
            ]
        }"#;

        let parsed = serde_json::from_str::<ModelsResponse>(payload).expect("payload should parse");
        let model = parsed.data.first().expect("model should be present");

        assert_eq!(model.id, "openai/gpt-4o");
        assert_eq!(model.pricing.prompt, "0.000005");
        assert_eq!(model.pricing.completion, "0.000015");
        assert!(model.pricing.request.is_empty());
        assert!(model.pricing.image.is_empty());
    }
}
