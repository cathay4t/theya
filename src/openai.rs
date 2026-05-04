// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{
    error::{ErrorKind, TheyaError},
    json_schema::JsonSchema,
};

// Default time out to 30 minutes
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30 * 60);

// Limit the message size to 128KiB for performance concern
const MAX_MSG_SIZE: usize = 128 * 1024;

fn default_function_type() -> String {
    "function".to_string()
}

/// Document: https://platform.openai.com/docs/api-reference/chat/create
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct OpenAiToolPrototype {
    #[serde(default = "default_function_type", rename = "type")]
    pub(crate) kind: String,
    pub(crate) function: OpenAiFunctionPrototype,
}

impl From<OpenAiFunctionPrototype> for OpenAiToolPrototype {
    fn from(function: OpenAiFunctionPrototype) -> Self {
        Self {
            kind: default_function_type(),
            function,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct OpenAiFunctionPrototype {
    pub(crate) name: String,
    pub(crate) parameters: JsonSchema,
    #[serde(default)]
    pub(crate) description: Option<String>,
}

/// Role of a message in the chat conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OpenAiChatMessageRole {
    System,
    #[default]
    User,
    Assistant,
    Tool,
}

/// Tool call within an assistant message as received from the API.
/// Arguments are a JSON-encoded string per the OpenAI spec.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct OpenAiApiToolCall {
    pub(crate) id: String,
    #[serde(rename = "type", default = "default_function_type")]
    pub(crate) kind: String,
    pub(crate) function: OpenAiApiToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct OpenAiApiToolFunction {
    pub(crate) name: String,
    /// JSON-encoded string of the arguments
    pub(crate) arguments: String,
}

/// Normalized tool call for internal use with arguments already parsed.
#[derive(Debug, Clone, Default)]
pub(crate) struct OpenAiTool {
    pub(crate) id: String,
    pub(crate) function: OpenAiFunction,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct OpenAiFunction {
    pub(crate) name: String,
    pub(crate) arguments: serde_json::Value,
}

impl TryFrom<OpenAiApiToolCall> for OpenAiTool {
    type Error = TheyaError;

    fn try_from(call: OpenAiApiToolCall) -> Result<Self, Self::Error> {
        let arguments = serde_json::from_str(&call.function.arguments)
            .unwrap_or(serde_json::Value::Object(Default::default()));
        Ok(Self {
            id: call.id,
            function: OpenAiFunction {
                name: call.function.name,
                arguments,
            },
        })
    }
}

/// Document: https://platform.openai.com/docs/api-reference/chat/create#messages
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct OpenAiChatMessage {
    pub(crate) role: OpenAiChatMessageRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<String>,
    /// Present on tool response messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_call_id: Option<String>,
    /// Present on assistant messages that request tool calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_calls: Option<Vec<OpenAiApiToolCall>>,
    /// DeepSeek thinking mode: chain-of-thought content that must be echoed
    /// back to the API verbatim in subsequent requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reasoning_content: Option<String>,
}

/// Document: https://platform.openai.com/docs/api-reference/chat/create
#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct OpenAiChatRequest {
    pub(crate) model: String,
    pub(crate) messages: Vec<OpenAiChatMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) tools: Vec<OpenAiToolPrototype>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_tokens: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct OpenAiChoice {
    pub(crate) index: usize,
    pub(crate) message: OpenAiChatMessage,
    pub(crate) finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct OpenAiErrorDetail {
    pub(crate) message: String,
}

/// Document: https://platform.openai.com/docs/api-reference/chat/object
#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct OpenAiChatResponse {
    pub(crate) model: Option<String>,
    pub(crate) error: Option<OpenAiErrorDetail>,
    pub(crate) choices: Option<Vec<OpenAiChoice>>,
}

impl OpenAiChatResponse {
    pub(crate) fn message(&self) -> Option<&OpenAiChatMessage> {
        self.choices.as_ref()?.first().map(|c| &c.message)
    }

    pub(crate) fn take_message(&mut self) -> Option<OpenAiChatMessage> {
        self.choices
            .as_mut()?
            .first_mut()
            .map(|c| std::mem::take(&mut c.message))
    }
}

/// Simplified response returned by `generate_ai_response`
pub(crate) struct OpenAiGenerateResponse {
    pub(crate) response: String,
    /// Elapsed wall-clock time in nanoseconds
    pub(crate) total_duration_ns: u64,
}

#[derive(Debug, Default)]
pub(crate) struct OpenAiClient {
    pub(crate) client: reqwest::Client,
    pub(crate) uri: String,
    pub(crate) model: String,
    pub(crate) guideline: String,
    pub(crate) api_key: String,
    /// Maximum number of tokens to generate per response.
    pub(crate) max_tokens: i32,
    pub(crate) chat_user_message: OpenAiChatMessage,
    pub(crate) chat_history: Vec<OpenAiChatMessage>,
    pub(crate) tools: Vec<OpenAiToolPrototype>,
}

impl OpenAiClient {
    pub(crate) async fn new(
        uri: &str,
        model: &str,
        guideline: &str,
        api_key: &str,
        max_tokens: i32,
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

        // Fall back to the OPENAI_API_KEY environment variable when no key is
        // provided in the config file.
        let resolved_key = if api_key.is_empty() {
            std::env::var("OPENAI_API_KEY").unwrap_or_default()
        } else {
            api_key.to_string()
        };

        log::info!("OpenAI API URI: {uri}");
        log::info!("Model name: {model}");
        Ok(Self {
            client: reqwest::Client::new(),
            uri: uri.to_string(),
            model: model.to_string(),
            guideline: guideline.to_string(),
            api_key: resolved_key,
            max_tokens,
            ..Default::default()
        })
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    async fn execute_chat_completion(
        &self,
        request: &OpenAiChatRequest,
    ) -> Result<OpenAiChatResponse, TheyaError> {
        let url = format!("{}/v1/chat/completions", self.uri);
        log::debug!(
            "Sending request {}",
            serde_json::to_string_pretty(request)?
        );
        let result = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .timeout(DEFAULT_TIMEOUT)
            .json(request)
            .send()
            .await;
        let response = match result {
            Ok(r) => r,
            Err(e) => {
                if e.is_timeout() {
                    return Err(TheyaError::new(
                        ErrorKind::Timeout,
                        format!("Request of {url} timeout"),
                    ));
                } else {
                    return Err(TheyaError::from(e));
                }
            }
        };
        let reply: OpenAiChatResponse = response.json().await?;
        log::debug!("Got reply {}", serde_json::to_string_pretty(&reply)?);
        if let Some(err) = reply.error.as_ref() {
            Err(TheyaError::new(ErrorKind::Bug, err.message.clone()))
        } else {
            Ok(reply)
        }
    }

    pub(crate) async fn generate_ai_response(
        &self,
        prompt: String,
    ) -> Result<OpenAiGenerateResponse, TheyaError> {
        let start = std::time::Instant::now();
        let messages = vec![
            OpenAiChatMessage {
                role: OpenAiChatMessageRole::System,
                content: Some(self.guideline.clone()),
                ..Default::default()
            },
            OpenAiChatMessage {
                role: OpenAiChatMessageRole::User,
                content: Some(prompt),
                ..Default::default()
            },
        ];
        let request = OpenAiChatRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(1.0),
            max_tokens: Some(self.max_tokens),
            ..Default::default()
        };
        let mut reply = self.execute_chat_completion(&request).await?;
        let elapsed_ns = start.elapsed().as_nanos() as u64;
        let response = reply
            .take_message()
            .and_then(|m| m.content)
            .unwrap_or_default();
        Ok(OpenAiGenerateResponse {
            response,
            total_duration_ns: elapsed_ns,
        })
    }

    pub(crate) fn set_user_message(&mut self, message: OpenAiChatMessage) {
        self.chat_user_message = message;
    }

    // Compress the historical message by requesting AI to summarize the
    // history.
    async fn compress_chat_message(&mut self) -> Result<(), TheyaError> {
        let mut messages = vec![
            OpenAiChatMessage {
                role: OpenAiChatMessageRole::System,
                content: Some(self.guideline.clone()),
                ..Default::default()
            },
            self.chat_user_message.clone(),
        ];
        for msg in self.chat_history.iter() {
            messages.push(msg.clone());
        }
        messages.push(OpenAiChatMessage {
            role: OpenAiChatMessageRole::User,
            content: Some(
                "In order to increase performance, please make a summery the \
                 provides messages, this summery will be used to replace \
                 historical message for follow up actions"
                    .to_string(),
            ),
            ..Default::default()
        });

        let request = OpenAiChatRequest {
            model: self.model.clone(),
            messages,
            tools: self.tools.clone(),
            temperature: Some(1.0),
            max_tokens: Some(self.max_tokens),
        };

        log::info!("Request AI to make summery on historical messages");
        let mut reply = self.execute_chat_completion(&request).await?;
        if let Some(message) = reply.take_message() {
            let content = message.content.as_deref().unwrap_or("").to_string();
            log::info!("AI: {}", content);
            self.chat_history.clear();
            self.chat_history.push(message);
            Ok(())
        } else {
            Err(TheyaError::new(
                ErrorKind::Bug,
                "Got AI reply without message".to_string(),
            ))
        }
    }

    pub(crate) fn reset_chat_history(&mut self) {
        self.chat_history.clear();
    }

    pub(crate) fn add_chat_message(&mut self, message: OpenAiChatMessage) {
        self.chat_history.push(message);
    }

    pub(crate) fn set_tools(&mut self, tools: Vec<OpenAiToolPrototype>) {
        self.tools = tools;
    }

    pub(crate) fn get_messages(&self) -> Vec<OpenAiChatMessage> {
        let mut messages = vec![
            OpenAiChatMessage {
                role: OpenAiChatMessageRole::System,
                content: Some(self.guideline.clone()),
                ..Default::default()
            },
            self.chat_user_message.clone(),
        ];
        for msg in self.chat_history.iter() {
            messages.push(msg.clone());
        }
        messages
    }

    pub(crate) async fn chat(
        &mut self,
    ) -> Result<OpenAiChatResponse, TheyaError> {
        let mut messages = self.get_messages();
        // Request compressing on historical message if exceeded
        let message_json_str = serde_json::to_string(&messages)?;
        if message_json_str.len() > MAX_MSG_SIZE {
            self.compress_chat_message().await?;
            messages = self.get_messages();
            messages.push(OpenAiChatMessage {
                role: OpenAiChatMessageRole::User,
                content: Some(
                    "Please continue for next step if coding task is not \
                     finished"
                        .to_string(),
                ),
                ..Default::default()
            });
        }

        let request = OpenAiChatRequest {
            model: self.model.clone(),
            messages,
            tools: self.tools.clone(),
            temperature: Some(1.0),
            max_tokens: Some(self.max_tokens),
        };

        let reply = self.execute_chat_completion(&request).await?;

        if let Some(message) = reply.message().cloned() {
            if let Some(reasoning) = message.reasoning_content.as_ref() {
                log::info!("AI reasoning: {}", reasoning);
            }

            if let Some(msg) = message.content.as_ref() {
                log::info!("AI: {}", msg);
            }
            self.add_chat_message(message);
        }

        Ok(reply)
    }
}
