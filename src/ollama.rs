// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use super::error::CliError;

/// Document: https://docs.ollama.com/api/generate
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OllamaGenerate {
    pub(crate) model: String,
    pub(crate) prompt: String,
    pub(crate) system: String,
    pub(crate) stream: bool,
    pub(crate) keep_alive: String,
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
    /// Maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none", rename = "num_predict")]
    pub(crate) output_token_limit: Option<i32>,
    /// Context length size (number of tokens)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) num_ctx: Option<i32>,
}

/// Document: https://docs.ollama.com/api/generate#response-model
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OllamaGenerateResponse {
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
    #[serde(rename = "total_duration")]
    pub(crate) total_duration_ns: u64,
}

pub(crate) struct OllamaClient {
    pub(crate) client: reqwest::Client,
    pub(crate) uri: String,
    pub(crate) model: String,
}

const DEFAULT_MODEL: &str = "qwen3-coder:30b";
const DEFAULT_URI: &str = "http://localhost:11434";

impl OllamaClient {
    pub(crate) async fn new() -> Result<Self, CliError> {
        let uri = std::env::var("THEYA_URI")
            .unwrap_or_else(|_| DEFAULT_URI.to_string());
        let model = std::env::var("THEYA_MODEL")
            .unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        log::info!("Ollama URI: {uri}");
        log::info!("Module name {model}");
        let ret = Self {
            client: reqwest::Client::new(),
            uri: uri.to_string(),
            model: model.to_string(),
        };
        log::info!("Ollama version {}", ret.version().await?);
        Ok(ret)
    }

    async fn generate(
        &self,
        request: &OllamaGenerate,
    ) -> Result<OllamaGenerateResponse, CliError> {
        let url = format!("{}/api/generate", self.uri);
        let response = self.client.post(&url).json(request).send().await?;
        let json: OllamaGenerateResponse = response.json().await?;
        Ok(json)
    }

    pub(crate) async fn version(&self) -> Result<String, CliError> {
        let url = format!("{}/api/version", self.uri);
        let response = self.client.get(&url).send().await?;
        Ok(response.text().await?)
    }

    pub(crate) async fn generate_ai_response(
        &self,
        system: String,
        prompt: String,
        num_ctx: i32,
    ) -> Result<OllamaGenerateResponse, CliError> {
        let request = OllamaGenerate {
            model: self.model.to_string(),
            prompt,
            system,
            keep_alive: "0".into(),
            stream: false,
            options: Some(OllamaGenerateOptions {
                temperature: Some(1.0),
                num_ctx: Some(num_ctx),
                output_token_limit: Some(-1),
                ..Default::default()
            }),
        };
        self.generate(&request).await
    }
}
