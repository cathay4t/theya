// SPDX-License-Identifier: Apache-2.0

use std::{collections::VecDeque, time::Duration};

use serde::{Deserialize, Serialize};

use super::{
    error::{ErrorKind, TheyaError},
    json_schema::JsonSchema,
    tools::{ToolReadFile, ToolWriteFile},
};
use crate::tools::ToolHandler;

// Default time out to 5 hours
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5 * 60 * 60);
// When to clean up memory after OllamaClient quit.
const KEEPALIVE: &str = "5m";

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
    pub(crate) options: Option<OllamaOptions>,
}

/// Role of a message in the chat conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OllamaChatMessageRole {
    System,
    #[default]
    User,
    Assistant,
    Tool,
}

fn default_function_type() -> String {
    "function".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct OllamaToolPrototype {
    #[serde(default = "default_function_type", rename = "type")]
    pub(crate) kind: String,
    pub(crate) function: OllamaFunctionPrototype,
}

impl From<OllamaFunctionPrototype> for OllamaToolPrototype {
    fn from(function: OllamaFunctionPrototype) -> Self {
        Self {
            kind: default_function_type(),
            function,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct OllamaFunctionPrototype {
    pub(crate) name: String,
    pub(crate) parameters: JsonSchema,
    #[serde(default)]
    pub(crate) description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct OllamaTool {
    pub(crate) id: String,
    pub(crate) function: OllamaFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct OllamaFunction {
    pub(crate) index: usize,
    pub(crate) name: String,
    pub(crate) arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct OllamaChatMessage {
    pub(crate) role: OllamaChatMessageRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_name: Option<String>,
    pub(crate) content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) thinking: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_calls: Option<Vec<OllamaTool>>,
}

/// Document: https://docs.ollama.com/api/chat
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct OllamaChat {
    pub(crate) model: String,
    pub(crate) messages: Vec<OllamaChatMessage>,
    pub(crate) stream: bool,
    #[serde(default)]
    pub(crate) tools: Vec<OllamaToolPrototype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) format: Option<JsonSchema>,
    pub(crate) keep_alive: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) options: Option<OllamaOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct OllamaChatResponse {
    pub(crate) model: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) message: Option<OllamaChatMessage>,
    pub(crate) done: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) done_reason: Option<String>,
    /// Time spent generating the response in nanoseconds
    #[serde(rename = "total_duration", default)]
    pub(crate) total_duration_ns: u64,
}

/// Document: https://docs.ollama.com/api/generate#body-options
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct OllamaOptions {
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

#[derive(Debug, Default)]
pub(crate) struct OllamaClient {
    pub(crate) client: reqwest::Client,
    pub(crate) uri: String,
    pub(crate) model: String,
    pub(crate) guideline: String,
    pub(crate) num_ctx: i32,
    // 0 means save all without limitation
    pub(crate) max_chat_history: usize,
    pub(crate) chat_user_message: OllamaChatMessage,
    pub(crate) chat_history: VecDeque<OllamaChatMessage>,
    pub(crate) tools: Vec<OllamaToolPrototype>,
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
            ..Default::default()
        };
        log::info!("Ollama version {}", ret.version().await?);
        Ok(ret)
    }

    pub(crate) fn reset_chat_history(&mut self) {
        self.chat_history.clear();
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
            options: Some(OllamaOptions {
                temperature: Some(1.0),
                num_ctx: Some(self.num_ctx),
                ..Default::default()
            }),
            ..Default::default()
        };
        self.generate(&request).await
    }

    pub(crate) fn set_user_message(&mut self, message: OllamaChatMessage) {
        self.chat_user_message = message;
    }

    pub(crate) fn set_max_chat_history(&mut self, v: usize) {
        self.max_chat_history = v;
    }

    // Set all file read and write content to `<omitted>`
    pub(crate) fn compress_chat_message(&mut self) {
        for msg in self.chat_history.iter_mut() {
            let Some(tool_name) = msg.tool_name.as_ref() else {
                continue;
            };
            if tool_name == ToolReadFile::NAME
                || tool_name == ToolWriteFile::NAME
            {
                msg.content = "<omitted>".to_string();
            }
        }
    }

    pub(crate) fn add_chat_message(&mut self, message: OllamaChatMessage) {
        self.chat_history.push_back(message)
    }

    pub(crate) fn set_tools(&mut self, tools: Vec<OllamaToolPrototype>) {
        self.tools = tools;
    }

    pub(crate) async fn chat(
        &mut self,
    ) -> Result<OllamaChatResponse, TheyaError> {
        let mut messages = vec![
            OllamaChatMessage {
                role: OllamaChatMessageRole::System,
                content: self.guideline.to_string(),
                ..Default::default()
            },
            self.chat_user_message.clone(),
        ];
        for msg in self.chat_history.iter() {
            messages.push(msg.clone());
        }

        let request = OllamaChat {
            model: self.model.to_string(),
            messages,
            format: None,
            keep_alive: KEEPALIVE.into(),
            tools: self.tools.clone(),
            options: Some(OllamaOptions {
                temperature: Some(1.0),
                num_ctx: Some(self.num_ctx),
                ..Default::default()
            }),
            ..Default::default()
        };

        log::debug!(
            "Sending request {}",
            serde_json::to_string_pretty(&request)?
        );

        let url = format!("{}/api/chat", self.uri);
        let response = self
            .client
            .post(&url)
            .timeout(DEFAULT_TIMEOUT)
            .json(&request)
            .send()
            .await?;
        let reply: OllamaChatResponse = response.json().await?;
        log::debug!("Got reply {}", serde_json::to_string_pretty(&reply)?);
        if let Some(err_msg) = reply.error.as_ref() {
            Err(TheyaError::new(ErrorKind::Bug, err_msg.to_string()))
        } else {
            if let Some(message) = reply.message.as_ref() {
                if self.max_chat_history > 0
                    && self.chat_history.len() > self.max_chat_history
                {
                    self.chat_history.pop_front();
                }
                self.chat_history.push_back(message.clone());
            }

            Ok(reply)
        }
    }
}
