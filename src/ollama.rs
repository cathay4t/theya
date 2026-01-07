// SPDX-License-Identifier: Apache-2.0

use reqwest;
use serde::{Deserialize, Serialize};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) num_predict: Option<i32>,
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
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "total_duration"
    )]
    pub(crate) total_duration_ns: Option<u64>,
}

pub(crate) struct OllamaClient {
    client: reqwest::Client,
    base_url: String,
}

impl OllamaClient {
    pub(crate) fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub(crate) async fn generate(
        &self,
        request: &OllamaGenerate,
    ) -> Result<OllamaGenerateResponse, Box<dyn std::error::Error>> {
        let url = format!("{}/api/generate", self.base_url);
        let response = self.client.post(&url).json(request).send().await?;
        let json: OllamaGenerateResponse = response.json().await?;
        Ok(json)
    }

    pub(crate) async fn version(
        &self,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{}/api/version", self.base_url);
        let response = self.client.get(&url).send().await?;
        Ok(response.text().await?)
    }
}
