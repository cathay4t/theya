// SPDX-License-Identifier: Apache-2.0

pub(crate) struct TheyaTools;

use super::{
    ToolHandler, ToolHandlerCmd,
    cmd::{ToolCompile, ToolFormat, ToolLintCheck, ToolUnitTest},
    file::{ToolFileList, ToolGrep, ToolReadFile, ToolWriteFile},
    git::{
        ToolGitCheckout, ToolGitCreateCommit, ToolGitDiff, ToolGitLog,
        ToolGitShowCommit,
    },
};
use crate::{
    config::TheyaProjectConfig,
    error::TheyaError,
    openai::{
        OpenAiChatMessage, OpenAiChatMessageRole, OpenAiTool,
        OpenAiToolPrototype,
    },
};

impl TheyaTools {
    pub(crate) fn code() -> Vec<OpenAiToolPrototype> {
        vec![
            ToolCompile::prototype(),
            ToolFileList::prototype(),
            ToolFormat::prototype(),
            ToolGitCreateCommit::prototype(),
            ToolGitDiff::prototype(),
            ToolGitLog::prototype(),
            ToolGitShowCommit::prototype(),
            ToolGrep::prototype(),
            ToolLintCheck::prototype(),
            ToolReadFile::prototype(),
            ToolUnitTest::prototype(),
            ToolWriteFile::prototype(),
            ToolGitCheckout::prototype(),
        ]
    }

    pub(crate) fn patch_review() -> Vec<OpenAiToolPrototype> {
        vec![
            ToolCompile::prototype(),
            ToolFileList::prototype(),
            ToolFormat::prototype(),
            ToolGitLog::prototype(),
            ToolGitShowCommit::prototype(),
            ToolGrep::prototype(),
            ToolLintCheck::prototype(),
            ToolReadFile::prototype(),
            ToolUnitTest::prototype(),
            ToolWriteFile::prototype(),
            ToolGitCheckout::prototype(),
        ]
    }

    /// Return a chat message for replying to AI, second item is boolean on
    /// where previous tool reply should be purged to save context windows.
    pub(crate) fn handle(
        tool: OpenAiTool,
        project_config: &TheyaProjectConfig,
    ) -> Result<OpenAiChatMessage, TheyaError> {
        let arguments = tool.function.arguments;

        let args_display = if tool.function.name.as_str() == ToolWriteFile::NAME
        {
            "<omitted>".to_string()
        } else {
            arguments.to_string()
        };

        log::info!(
            "Invoking tool `{}` with arguments: {args_display}",
            tool.function.name,
        );

        let content = match tool.function.name.as_str() {
            ToolFileList::NAME => ToolFileList::handle(arguments)?,
            ToolReadFile::NAME => ToolReadFile::handle(arguments)?,
            ToolWriteFile::NAME => ToolWriteFile::handle(arguments)?,
            ToolCompile::NAME => ToolCompile::handle(
                project_config.compile.as_deref(),
                arguments,
            )?,
            ToolUnitTest::NAME => ToolUnitTest::handle(
                project_config.unit_test.as_deref(),
                arguments,
            )?,
            ToolFormat::NAME => {
                ToolFormat::handle(project_config.format.as_deref(), arguments)?
            }
            ToolLintCheck::NAME => ToolLintCheck::handle(
                project_config.lint.as_deref(),
                arguments,
            )?,
            ToolGitLog::NAME => ToolGitLog::handle(arguments)?,
            ToolGitShowCommit::NAME => ToolGitShowCommit::handle(arguments)?,
            ToolGitCreateCommit::NAME => {
                ToolGitCreateCommit::handle(arguments)?
            }
            ToolGitDiff::NAME => ToolGitDiff::handle(arguments)?,
            ToolGrep::NAME => ToolGrep::handle(arguments)?,
            ToolGitCheckout::NAME => ToolGitCheckout::handle(arguments)?,
            tool_name => {
                log::warn!("Reject AI requested invalid tool: {tool_name}");
                serde_json::to_string(&format!(
                    "FAIL: Invalid tool_name {tool_name}"
                ))?
            }
        };
        let msg = OpenAiChatMessage {
            role: OpenAiChatMessageRole::Tool,
            tool_call_id: Some(tool.id),
            content: Some(content),
            ..Default::default()
        };

        Ok(msg)
    }
}
