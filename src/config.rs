// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::code::default_code_guideline;
use crate::TheyaError;

const DEFAULT_MODEL: &str = "qwen3-coder:30b";
const CONFIG_PATH: &str = ".config/theya/config";
const DEFAULT_URI: &str = "http://localhost:11434";
const DEFAULT_CODE_CONTEXT_COUNT: i32 = 10 * 1024 * 1024;

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
        if self.slow_chat.uri.is_empty() {
            self.slow_chat.uri = self.main.uri.clone();
        }
        if self.patch_review.uri.is_empty() {
            self.patch_review.uri = self.main.uri.clone();
        }
        if self.code.uri.is_empty() {
            self.code.uri = self.main.uri.clone();
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
}

impl Default for TheyaMainConfig {
    fn default() -> Self {
        Self { uri: default_uri() }
    }
}

fn default_model() -> String {
    DEFAULT_MODEL.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TheyaQuickChatConfig {
    #[serde(default = "default_model")]
    pub(crate) model: String,
    #[serde(default)]
    pub(crate) uri: String,
}

impl Default for TheyaQuickChatConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            uri: default_uri(),
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
}

impl Default for TheyaSlowChatConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            uri: default_uri(),
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
}

impl Default for TheyaPatchReviewConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            uri: default_uri(),
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
    #[serde(default = "default_code_guideline")]
    pub(crate) guideline: String,
    #[serde(default = "default_code_context_count")]
    pub(crate) context_count: i32,
}

fn default_code_context_count() -> i32 {
    DEFAULT_CODE_CONTEXT_COUNT
}

impl Default for TheyaCodeConfig {
    fn default() -> Self {
        Self {
            uri: default_uri(),
            model: default_model(),
            guideline: default_code_guideline(),
            context_count: DEFAULT_CODE_CONTEXT_COUNT,
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
uri = "http://ollama.example.org:11434"

[quick-chat]
model = "qwen3-coder:30b-a3b-q4_K_M"

[slow-chat]
model = "qwen3-coder:30b-a3b-q8_0"

[patch-review]
model = "qwen3.5:35b"

[code]
model = "qwen3-coder:30b-a3b-q8_0"
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

        assert_eq!(config.main.uri, "http://ollama.example.org:11434");

        assert_eq!(config.quick_chat.model, "qwen3-coder:30b-a3b-q4_K_M");

        assert_eq!(config.slow_chat.model, "qwen3-coder:30b-a3b-q8_0");

        assert_eq!(config.patch_review.model, "qwen3.5:35b");
        assert_eq!(config.code.model, "qwen3-coder:30b-a3b-q8_0");
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
