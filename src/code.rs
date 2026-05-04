// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, io::Write};

use super::{
    cmd::spawn_editor,
    config::{TheyaCodeConfig, TheyaProjectConfig},
    error::{ErrorKind, TheyaError},
    openai::{
        OpenAiChatMessage, OpenAiChatMessageRole, OpenAiClient, OpenAiTool,
    },
    tools::{Git, TheyaTools},
};

const DEFAULT_EDITOR: &str = "vim";
const COMMENT_PREFIX: &str = "<!-- Theya: ";
const COMMENT_POSTFIX: &str = " -->";
const MAX_ITERATION: usize = 1000;
const CODE_TASK_FILE: &str = "/tmp/theya_code_task.md";

pub(crate) fn default_code_guideline() -> String {
    "You are a Linux Software developer who is working in a git repo and \
     providing code assistant. The common workflow(in provided order):\n* \
     Check historical git commit of related files\n* Modify non-test code\n* \
     Compile and fix error\n* Run unit test and fix error\n* Add unit test if \
     fit\n* Run code format and lint check"
        .to_string()
}

pub(crate) struct CommandCode;

impl CommandCode {
    pub(crate) const CMD: &str = "code";

    pub(crate) fn new_cmd() -> clap::Command {
        clap::Command::new(Self::CMD).about("Request AI to code in a git repo")
    }

    pub(crate) async fn handle(
        config: &TheyaCodeConfig,
        projects_config: &HashMap<String, TheyaProjectConfig>,
    ) -> Result<(), TheyaError> {
        let now = std::time::SystemTime::now();

        std::env::set_current_dir(Git::get_root_dir_path()?.as_str())?;

        let project_url = Git::get_origin_remote_url()?;
        let project_config = projects_config
            .values()
            .find(|c| c.git.as_str() == project_url.as_str())
            .cloned()
            .unwrap_or_default();

        let mut client = OpenAiClient::new(
            config.uri.as_str(),
            config.model.as_str(),
            config.guideline.as_str(),
            config.api_key.as_str(),
            config.max_tokens,
        )
        .await?;

        log::trace!("System prompt:\n{}", config.guideline.as_str());

        let editor = std::env::var("EDITOR")
            .unwrap_or_else(|_| DEFAULT_EDITOR.to_string());

        if !std::path::Path::new(CODE_TASK_FILE).exists() {
            let mut fd = std::fs::File::create(CODE_TASK_FILE)?;
            fd.write_all(
                format!(
                    "\n\n{COMMENT_PREFIX}Please type your request above, save \
                     and quit{COMMENT_POSTFIX}\n",
                )
                .as_bytes(),
            )?;
        };

        spawn_editor(&editor, CODE_TASK_FILE)?;

        let coding_task = std::fs::read_to_string(CODE_TASK_FILE)?
            .lines()
            .filter(|line| !line.starts_with(COMMENT_PREFIX))
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .to_string();

        if coding_task.is_empty() {
            return Err(TheyaError::new(
                ErrorKind::EmptyInput,
                "Got empty input, quitting".into(),
            ));
        }

        log::info!("Coding task: {coding_task}");

        let prompt = format!(
            "\
            You been requested to modify files in current git repo for coding \
             task:\n```\n{coding_task}\n```\n\nRecommendations:\n1. A good \
             patch should pass compiling, unit test and link check.\n2. Only \
             create git commit after you consider current changes is a good \
             patch for specified coding task."
        );
        let init_chat_msg = OpenAiChatMessage {
            role: OpenAiChatMessageRole::User,
            content: Some(prompt),
            ..Default::default()
        };
        client.set_user_message(init_chat_msg);
        client.reset_chat_history();
        client.set_tools(TheyaTools::code(&project_config));

        for i in 0..MAX_ITERATION {
            log::info!("Iteration {}/{MAX_ITERATION}", i + 1);
            log::info!("Sending out chat message to AI");
            let mut reply = match client.chat().await {
                Ok(r) => r,
                Err(e) => {
                    if e.can_retry() {
                        // start again
                        client.reset_chat_history();
                        continue;
                    } else {
                        break;
                    }
                }
            };

            let Some(message) = reply.take_message() else {
                continue;
            };

            if let Some(tool_calls) = message.tool_calls
                && !tool_calls.is_empty()
            {
                for api_tool_call in tool_calls {
                    let tool_call_id = api_tool_call.id.clone();
                    let tool_call = match OpenAiTool::try_from(api_tool_call) {
                        Ok(t) => t,
                        Err(e) => {
                            log::warn!("Failed to parse tool arguments: {e}");
                            continue;
                        }
                    };
                    match TheyaTools::handle(tool_call, &project_config) {
                        Ok(msg) => {
                            client.add_chat_message(msg);
                            log::info!("Appended tool output to queue");
                        }
                        Err(e) => {
                            log::warn!("{e}");
                            client.add_chat_message(OpenAiChatMessage {
                                role: OpenAiChatMessageRole::Tool,
                                tool_call_id: Some(tool_call_id),
                                content: Some(format!("FAILED: {e}")),
                                ..Default::default()
                            });
                        }
                    }
                }
            } else {
                let prompt = "Do not modify any code, just check whether \
                              original coding task is finished or not, reply \
                              YES if done, otherwise provide action plan"
                    .to_string();

                let check_msg = OpenAiChatMessage {
                    role: OpenAiChatMessageRole::User,
                    content: Some(prompt),
                    ..Default::default()
                };
                client.add_chat_message(check_msg);

                match client.chat().await {
                    Ok(mut reply) => {
                        if let Some(msg) = reply.take_message() {
                            let content = msg.content.as_deref().unwrap_or("");
                            if content.contains("YES")
                                || content.contains("yes")
                            {
                                break;
                            } else {
                                client.add_chat_message(msg);
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        if e.can_retry() {
                            // start again
                            client.reset_chat_history();
                            continue;
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        if let Ok(elapsed) = now.elapsed() {
            log::info!("Elapsed: {} seconds", elapsed.as_secs());
        }
        std::fs::remove_file(CODE_TASK_FILE)?;

        Ok(())
    }
}
