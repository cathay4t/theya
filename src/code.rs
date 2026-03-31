// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, io::Write};

use super::{
    cmd::{run_command, spawn_editor},
    config::{TheyaCodeConfig, TheyaProjectConfig},
    error::{ErrorKind, TheyaError},
    git::Git,
    ollama::{OllamaChatMessage, OllamaChatMessageRole, OllamaClient},
    tools::{gen_tool_prototypes_for_coding, handle_tool},
};

const DEFAULT_EDITOR: &str = "vim";
const COMMENT_PREFIX: &str = "<!-- Theya: ";
const COMMENT_POSTFIX: &str = " -->";
const MAX_ITERATION: usize = 100;
const MAX_CHAT_HISTORY: usize = 10;

pub(crate) fn default_code_guideline() -> String {
    "You are a Linux Software developer who is working in a git repo and \
     providing code assistant."
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
        std::env::set_current_dir(Git::get_root_dir_path()?.as_str())?;

        let project_url = Git::get_origin_remote_url()?;
        let project_config = projects_config
            .values()
            .find(|c| c.git.as_str() == project_url.as_str())
            .cloned()
            .unwrap_or_default();

        let mut client = OllamaClient::new(
            config.uri.as_str(),
            config.model.as_str(),
            config.guideline.as_str(),
            config.context_count,
        )
        .await?;
        client.set_max_chat_history(MAX_CHAT_HISTORY);

        log::trace!("System prompt:\n{}", config.guideline.as_str());

        let editor = std::env::var("EDITOR")
            .unwrap_or_else(|_| DEFAULT_EDITOR.to_string());

        let tmp_file_path = std::path::PathBuf::from(format!(
            "{}.md",
            run_command("mktemp", &["-u"])?.1.trim()
        ));

        let mut fd = std::fs::File::create(&tmp_file_path)?;
        #[rustfmt::skip]
        fd.write_all(
            format!(
                "\n\n\
                {COMMENT_PREFIX}Ollama connected to: {}{COMMENT_POSTFIX}\n\
                {COMMENT_PREFIX}Ollama version: {}{COMMENT_POSTFIX}\n\
                {COMMENT_PREFIX}Model: {}{COMMENT_POSTFIX}\n\
                {COMMENT_PREFIX}Please type your request above, \
                save and quit{COMMENT_POSTFIX}\n",
                client.uri,
                client.version().await?,
                client.model,
            ).as_bytes(),
        )?;

        spawn_editor(&editor, &tmp_file_path)?;

        let coding_task = std::fs::read_to_string(&tmp_file_path)?
            .lines()
            .filter(|line| !line.starts_with(COMMENT_PREFIX))
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .to_string();

        std::fs::remove_file(&tmp_file_path)?;

        if coding_task.is_empty() {
            return Err(TheyaError::new(
                ErrorKind::EmptyInput,
                "Got empty input, quitting".into(),
            ));
        }

        log::info!("Coding task: {coding_task}");

        #[rustfmt::skip]
        let prompt = format!("
            You been requested to modify files in current git repo for coding \
            task:\n\
            ```\n\
            {coding_task}\n\
            ```\n\n\
            Suggested coding workflow is:\n\
             1. Find out which file by listing all files.\n\
             2. Find out historical git commit of interested file.\n\
             3. Modify files to fulfill specified coding task.\n\
             4. Compile the code and modify code if any got compile failure.\n\
             5. Run unit test and modify unit test code if got failure.\n\
             6. Run lint check and fix all lints.\n\
             6. Create git commit with summery of what changed.\n\n\
            In the seek of performance, historical message will not contains \
            read or write file content, hence make a summery on what you read \
            or about to write after got tool reply."
        );
        let init_chat_msg = OllamaChatMessage {
            role: OllamaChatMessageRole::User,
            content: prompt,
            ..Default::default()
        };
        client.set_user_message(init_chat_msg);
        client.reset_chat_history();
        client.set_tools(gen_tool_prototypes_for_coding());

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
                            // TODO: If historical message exceeded the
                            // maximum context length, ask AI to summarize
                            // and use summery to replace historical messages.
                            client.compress_chat_message();
                            client.add_chat_message(msg);
                            log::info!("Appended tool output to queue");
                        }
                        Err(e) => {
                            log::warn!("{e}");
                        }
                    }
                }
            } else {
                // TODO: Ask AI to check whether task been finished, otherwise
                // retry
                break;
            }
        }

        Ok(())
    }
}
