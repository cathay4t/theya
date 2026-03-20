// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{
    error::{ErrorKind, TheyaError},
    json_schema::JsonSchema,
};

// Default time out to 1 hour
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60 * 60);
// Clean up memory once OllamaClient quit.
const KEEPALIVE: &str = "0";

/// Document: https://docs.ollama.com/api/generate
#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct OllamaGenerate {
    pub(crate) model: String,
    pub(crate) prompt: String,
    pub(crate) system: String,
    pub(crate) stream: bool,
    pub(crate) keep_alive: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) format: Option<JsonSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) options: Option<OllamaGenerateOptions>,
}

/// Document: https://docs.ollama.com/api/generate#body-options
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct OllamaGenerateOptions {
    /// Controls randomness in generation (higher = more random)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) temperature: Option<f64>,
    /// Limits next token selection to the K most likely
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) top_k: Option<i32>,
    /// Cumulative probability threshold for nucleus sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) top_p: Option<f64>,
    /// Minimum probability threshold for token selection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) min_p: Option<f64>,
    /// Stop sequences that will halt generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stop: Option<Vec<String>>,
    /// Maximum number of tokens to generate, if undefined, will use model
    /// setting.
    #[serde(skip_serializing_if = "Option::is_none", rename = "num_predict")]
    pub(crate) output_token_limit: Option<i32>,
    /// Context length size (number of tokens)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) num_ctx: Option<i32>,
}

/// Document: https://docs.ollama.com/api/generate#response-model
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OllamaGenerateResponse {
    #[serde(default)]
    pub(crate) error: Option<String>,
    pub(crate) model: Option<String>,
    #[serde(default)]
    pub(crate) response: String,
    pub(crate) thinking: Option<String>,
    pub(crate) done: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) done_reason: Option<String>,
    /// Number of output tokens generated in the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) eval_count: Option<u64>,
    /// Time spent generating the response in nanoseconds
    #[serde(rename = "total_duration", default)]
    pub(crate) total_duration_ns: u64,
}

pub(crate) struct OllamaClient {
    pub(crate) client: reqwest::Client,
    pub(crate) uri: String,
    pub(crate) model: String,
    pub(crate) guideline: String,
    pub(crate) num_ctx: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaVersionResponse {
    version: String,
}

impl OllamaClient {
    pub(crate) async fn new(
        uri: &str,
        model: &str,
        guideline: &str,
        num_ctx: i32,
    ) -> Result<Self, TheyaError> {
        if std::env::var("THEYA_URI").is_ok() {
            return Err("The use of THEYA_URI is deprecated, please use \
                        $HOME/.config/theya/config instead"
                .into());
        }

        if std::env::var("THEYA_MODEL").is_ok() {
            return Err("The use of THEYA_MODEL is deprecated, please use \
                        $HOME/.config/theya/config instead"
                .into());
        }

        log::info!("Ollama URI: {uri}");
        log::info!("Module name {model}");
        let ret = Self {
            client: reqwest::Client::new(),
            uri: uri.to_string(),
            model: model.to_string(),
            guideline: guideline.to_string(),
            num_ctx,
        };
        log::info!("Ollama version {}", ret.version().await?);
        Ok(ret)
    }

    async fn generate(
        &self,
        request: &OllamaGenerate,
    ) -> Result<OllamaGenerateResponse, TheyaError> {
        let url = format!("{}/api/generate", self.uri);
        let response = self
            .client
            .post(&url)
            .timeout(DEFAULT_TIMEOUT)
            .json(request)
            .send()
            .await?;
        let reply: OllamaGenerateResponse = response.json().await?;
        if let Some(err_msg) = reply.error.as_ref() {
            Err(TheyaError::new(ErrorKind::Bug, err_msg.to_string()))
        } else {
            Ok(reply)
        }
    }

    pub(crate) async fn version(&self) -> Result<String, TheyaError> {
        let url = format!("{}/api/version", self.uri);
        let response = self.client.get(&url).send().await?;
        let version: OllamaVersionResponse = response.json().await?;
        Ok(version.version)
    }

    pub(crate) async fn generate_ai_response(
        &self,
        prompt: String,
    ) -> Result<OllamaGenerateResponse, TheyaError> {
        let request = OllamaGenerate {
            model: self.model.to_string(),
            prompt,
            system: self.guideline.clone(),
            keep_alive: KEEPALIVE.into(),
            stream: false,
            options: Some(OllamaGenerateOptions {
                temperature: Some(1.0),
                num_ctx: Some(self.num_ctx),
                ..Default::default()
            }),
            ..Default::default()
        };
        self.generate(&request).await
    }

    pub(crate) async fn generate_ai_structured_response(
        &self,
        prompt: String,
        json_schema: JsonSchema,
    ) -> Result<OllamaGenerateResponse, TheyaError> {
        let request = OllamaGenerate {
            model: self.model.to_string(),
            prompt,
            system: self.guideline.clone(),
            keep_alive: KEEPALIVE.into(),
            stream: false,
            format: Some(json_schema),
            options: Some(OllamaGenerateOptions {
                temperature: Some(1.0),
                num_ctx: Some(self.num_ctx),
                ..Default::default()
            }),
        };
        self.generate(&request).await
    }
}
