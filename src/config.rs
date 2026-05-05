// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::code::default_code_guideline;
use crate::TheyaError;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TheyaMemoryConfig {
    /// Whether to index GitHub Copilot chat history
    #[serde(default)]
    pub(crate) copilot: bool,
    /// URI for the knowledge-extraction chat API (falls back to [main] uri)
    #[serde(default)]
    pub(crate) uri: String,
    #[serde(default)]
    pub(crate) api_key: String,
    /// Model for knowledge extraction (falls back to quick-chat model)
    #[serde(default)]
    pub(crate) model: String,
    /// URI for the embeddings API (falls back to `uri`)
    #[serde(default)]
    pub(crate) embed_uri: String,
    /// Model for generating embeddings (falls back to `model`)
    #[serde(default)]
    pub(crate) embed_model: String,
    /// Number of dimensions for the embedding vectors (optional, uses model
    /// default when unset)
    #[serde(default)]
    pub(crate) embed_dimensions: Option<u32>,
}

const DEFAULT_MODEL: &str = "gpt-4o";
const CONFIG_PATH: &str = ".config/theya/config";
const DEFAULT_URI: &str = "https://api.openai.com";
const DEFAULT_QUICK_CHAT_MAX_TOKENS: i32 = 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct TheyaConfig {
    #[serde(default)]
    pub(crate) main: TheyaMainConfig,
    #[serde(default)]
    pub(crate) quick_chat: TheyaQuickChatConfig,
    #[serde(default)]
    pub(crate) slow_chat: TheyaSlowChatConfig,
    #[serde(default)]
    pub(crate) patch_review: TheyaPatchReviewConfig,
    #[serde(default)]
    pub(crate) code: TheyaCodeConfig,
    #[serde(default)]
    pub(crate) memory: TheyaMemoryConfig,
    #[serde(default)]
    pub(crate) projects: HashMap<String, TheyaProjectConfig>,
}

impl TheyaConfig {
    pub(crate) fn load() -> Result<Self, TheyaError> {
        let Ok(home_dir) = std::env::var("HOME") else {
            log::warn!("HOME system variable undefined, using default config");
            return Ok(Self::default());
        };

        let config_path =
            std::path::PathBuf::from(format!("{home_dir}/{CONFIG_PATH}"));

        let mut ret = if config_path.exists() {
            let toml_content =
                std::fs::read_to_string(format!("{home_dir}/{CONFIG_PATH}"))?;
            Self::parse(&toml_content)?
        } else {
            Self::default()
        };
        ret.apply_main_config();

        Ok(ret)
    }

    pub(crate) fn parse(toml_content: &str) -> Result<Self, TheyaError> {
        toml::from_str(toml_content)
            .map_err(|e| TheyaError::from(e.to_string()))
    }

    pub(crate) fn apply_main_config(&mut self) {
        if self.quick_chat.uri.is_empty() {
            self.quick_chat.uri = self.main.uri.clone();
        }
        if self.quick_chat.api_key.is_empty() {
            self.quick_chat.api_key = self.main.api_key.clone();
        }
        if self.slow_chat.uri.is_empty() {
            self.slow_chat.uri = self.main.uri.clone();
        }
        if self.slow_chat.api_key.is_empty() {
            self.slow_chat.api_key = self.main.api_key.clone();
        }
        if self.patch_review.uri.is_empty() {
            self.patch_review.uri = self.main.uri.clone();
        }
        if self.patch_review.api_key.is_empty() {
            self.patch_review.api_key = self.main.api_key.clone();
        }
        if self.code.uri.is_empty() {
            self.code.uri = self.main.uri.clone();
        }
        if self.code.api_key.is_empty() {
            self.code.api_key = self.main.api_key.clone();
        }
        if self.memory.uri.is_empty() {
            self.memory.uri = self.main.uri.clone();
        }
        if self.memory.api_key.is_empty() {
            self.memory.api_key = self.main.api_key.clone();
        }
        if self.memory.model.is_empty() {
            self.memory.model = self.quick_chat.model.clone();
        }
        if self.memory.embed_uri.is_empty() {
            self.memory.embed_uri = self.memory.uri.clone();
        }
        if self.memory.embed_model.is_empty() {
            self.memory.embed_model = self.memory.model.clone();
        }
    }
}

fn default_uri() -> String {
    DEFAULT_URI.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TheyaMainConfig {
    #[serde(default = "default_uri")]
    pub(crate) uri: String,
    #[serde(default)]
    pub(crate) api_key: String,
}

impl Default for TheyaMainConfig {
    fn default() -> Self {
        Self {
            uri: default_uri(),
            api_key: String::new(),
        }
    }
}

fn default_model() -> String {
    DEFAULT_MODEL.to_string()
}

fn default_quick_chat_max_tokens() -> i32 {
    DEFAULT_QUICK_CHAT_MAX_TOKENS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TheyaQuickChatConfig {
    #[serde(default = "default_model")]
    pub(crate) model: String,
    #[serde(default)]
    pub(crate) uri: String,
    #[serde(default)]
    pub(crate) api_key: String,
    #[serde(default = "default_quick_chat_max_tokens")]
    pub(crate) max_tokens: i32,
}

impl Default for TheyaQuickChatConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            uri: default_uri(),
            api_key: String::new(),
            max_tokens: DEFAULT_QUICK_CHAT_MAX_TOKENS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TheyaSlowChatConfig {
    #[serde(default = "default_model")]
    pub(crate) model: String,
    #[serde(default)]
    pub(crate) uri: String,
    #[serde(default)]
    pub(crate) api_key: String,
    #[serde(default)]
    pub(crate) max_tokens: Option<i32>,
}

impl Default for TheyaSlowChatConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            uri: default_uri(),
            api_key: String::new(),
            max_tokens: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TheyaPatchReviewConfig {
    #[serde(default = "default_model")]
    pub(crate) model: String,
    #[serde(default)]
    pub(crate) uri: String,
    #[serde(default)]
    pub(crate) api_key: String,
    #[serde(default)]
    pub(crate) max_tokens: Option<i32>,
}

impl Default for TheyaPatchReviewConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            uri: default_uri(),
            api_key: String::new(),
            max_tokens: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TheyaProjectConfig {
    pub(crate) git: String,
    /// Command to compile
    #[serde(default)]
    pub(crate) compile: Option<String>,
    #[serde(default)]
    pub(crate) format: Option<String>,
    #[serde(default)]
    pub(crate) lint: Option<String>,
    /// Command to run unit test
    #[serde(default)]
    pub(crate) unit_test: Option<String>,
    /// Command to run integration test
    #[serde(default)]
    pub(crate) integ_test: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TheyaCodeConfig {
    #[serde(default)]
    pub(crate) uri: String,
    #[serde(default = "default_model")]
    pub(crate) model: String,
    #[serde(default)]
    pub(crate) api_key: String,
    #[serde(default = "default_code_guideline")]
    pub(crate) guideline: String,
    #[serde(default)]
    pub(crate) max_tokens: Option<i32>,
}

impl Default for TheyaCodeConfig {
    fn default() -> Self {
        Self {
            uri: default_uri(),
            model: default_model(),
            api_key: String::new(),
            guideline: default_code_guideline(),
            max_tokens: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let toml_content = r#"
[main]
uri = "https://api.openai.com"
api_key = "sk-proj-test"

[quick-chat]
model = "gpt-4o-mini"
max_tokens = 512

[slow-chat]
model = "gpt-4o"

[patch-review]
model = "gpt-4o"
max_tokens = 8192

[code]
model = "gpt-4o"
guideline = """
Test multiple lines
guideline
"""

[projects.nipart]
git = "https://github.com/cathay4t/nipart.git"
compile = "cargo build"
format = "cargo fmt"
unit_test = "cargo test"
integ_test = "sudo tests/run-tests.sh"
"#;

        let config = TheyaConfig::parse(toml_content).unwrap();

        assert_eq!(config.main.uri, "https://api.openai.com");
        assert_eq!(config.main.api_key, "sk-proj-test");

        assert_eq!(config.quick_chat.model, "gpt-4o-mini");
        assert_eq!(config.quick_chat.max_tokens, 512);

        assert_eq!(config.slow_chat.model, "gpt-4o");
        assert_eq!(config.slow_chat.max_tokens, None);

        assert_eq!(config.patch_review.model, "gpt-4o");
        assert_eq!(config.patch_review.max_tokens, Some(8192));

        assert_eq!(config.code.model, "gpt-4o");
        assert_eq!(config.code.max_tokens, None);
        assert_eq!(config.code.guideline, "Test multiple lines\nguideline\n");

        assert_eq!(config.projects.len(), 1);
        let project = &config.projects["nipart"];
        assert_eq!(project.git, "https://github.com/cathay4t/nipart.git");
        assert_eq!(project.compile.as_deref(), Some("cargo build"));
        assert_eq!(project.format.as_deref(), Some("cargo fmt"));
        assert_eq!(project.unit_test.as_deref(), Some("cargo test"));
        assert_eq!(
            project.integ_test.as_deref(),
            Some("sudo tests/run-tests.sh")
        );
    }
}
