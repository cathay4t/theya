// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::{
    config::{TheyaPatchReviewConfig, TheyaProjectConfig},
    error::TheyaError,
    git::Git,
    ollama::OllamaClient,
    tools::{gen_tool_prototypes_for_review, handle_tool},
};
use crate::ollama::{OllamaChatMessage, OllamaChatMessageRole};

pub(crate) struct CommandPatchReview;

const CONTEXT_NUMBER: i32 = 10 * 1024 * 1024;
const MAX_ITERATION: usize = 50;

const SYSTEM_PROMPT: &str =
    "You are a Linux software developer reviewing most recent git commit. You \
     can only access single file each round in the seek of performance.";

impl CommandPatchReview {
    pub(crate) const CMD: &str = "patch-review";

    pub(crate) fn new_cmd() -> clap::Command {
        clap::Command::new(Self::CMD)
            .alias("pr")
            .about("Patch review")
    }

    pub(crate) async fn handle(
        config: &TheyaPatchReviewConfig,
        projects_config: &HashMap<String, TheyaProjectConfig>,
    ) -> Result<(), TheyaError> {
        std::env::set_current_dir(Git::get_root_dir_path()?.as_str())?;

        let uri = config.uri.as_str();
        let model = config.model.as_str();

        let project_url = Git::get_origin_remote_url()?;
        let project_config = projects_config
            .values()
            .find(|c| c.git.as_str() == project_url.as_str())
            .cloned()
            .unwrap_or_default();

        let mut client =
            OllamaClient::new(uri, model, SYSTEM_PROMPT, CONTEXT_NUMBER)
                .await?;

        let prompt = "Please review most recent git commit and fix the code \
                      files when required, a good patch(git commit) should \
                      pass the compile, unit test and lint check."
            .to_string();
        let init_chat_msg = OllamaChatMessage {
            role: OllamaChatMessageRole::User,
            content: prompt,
            ..Default::default()
        };
        client.set_user_message(init_chat_msg);
        client.reset_chat_history();
        client.set_tools(gen_tool_prototypes_for_review());

        log::info!("Reviewing patch: {}", Git::get_cur_patch_titile()?.trim());

        for i in 0..MAX_ITERATION {
            log::info!("Iteration {}/{MAX_ITERATION}", i + 1);
            log::info!("Sending out chat message to AI");
            let reply = client.chat().await?;
            let elapsed =
                std::time::Duration::from_nanos(reply.total_duration_ns);
            log::info!("Elapsed: {:.02} seconds", elapsed.as_secs_f64());

            let Some(message) = reply.message else {
                continue;
            };
            log::info!("AI: {}", message.content);

            if let Some(tool_calls) = message.tool_calls
                && !tool_calls.is_empty()
            {
                for tool_call in tool_calls {
                    match handle_tool(tool_call, &project_config) {
                        Ok(msg) => {
                            // We do not compress message here, full history
                            // is good for patch review
                            client.add_chat_message(msg);
                            log::info!("Appended tool output to queue");
                        }
                        Err(e) => {
                            log::warn!("{e}");
                        }
                    }
                }
            } else {
                break;
            }
        }

        Ok(())
    }
}
