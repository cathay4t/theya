// SPDX-License-Identifier: Apache-2.0

use std::io::Write;

use super::{
    cmd::{run_command, spawn_editor},
    config::TheyaConfig,
    error::{ErrorKind, TheyaError},
    openai::OpenAiClient,
};

const DEFAULT_EDITOR: &str = "vim";
const COMMENT_PREFIX: &str = "<!-- Theya: ";
const COMMENT_POSTFIX: &str = " -->";

#[rustfmt::skip]
const QUICK_SYSTEM_PROMPT: &str = "\
You are a Minimalism Linux Software developer providing coding assistance.\n\
Guidelines:
1. Answer with minimum content.\n\
2. Only include single recommendation.\n";

#[rustfmt::skip]
const SLOW_SYSTEM_PROMPT: &str = "\
You are a experienced Linux Software developer providing coding assistance.\n\
Guidelines:
1. Only include single recommendation.\n";

pub(crate) struct CommandChat;

impl CommandChat {
    pub(crate) const CMD: &str = "chat";

    pub(crate) fn new_cmd() -> clap::Command {
        clap::Command::new(Self::CMD)
            .alias("c")
            .about("Chat with AI")
            .arg(
                clap::Arg::new("SLOW")
                    .long("slow")
                    .action(clap::ArgAction::SetTrue)
                    .help(
                        "Take a deliberation considering with slow \
                         performance. Default false -- quick casual chat",
                    ),
            )
    }

    pub(crate) async fn handle(
        matches: &clap::ArgMatches,
        config: &TheyaConfig,
    ) -> Result<(), TheyaError> {
        let is_slow = matches.get_flag("SLOW");
        let system_prompt = if is_slow {
            SLOW_SYSTEM_PROMPT
        } else {
            QUICK_SYSTEM_PROMPT
        };
        let (uri, model, api_key) = if is_slow {
            (
                config.slow_chat.uri.as_str(),
                config.slow_chat.model.as_str(),
                config.slow_chat.api_key.as_str(),
            )
        } else {
            (
                config.quick_chat.uri.as_str(),
                config.quick_chat.model.as_str(),
                config.quick_chat.api_key.as_str(),
            )
        };

        let client = OpenAiClient::new(
            uri,
            model,
            system_prompt,
            api_key,
            if is_slow {
                config.slow_chat.max_tokens
            } else {
                Some(config.quick_chat.max_tokens)
            },
        )
        .await?;

        let editor = std::env::var("EDITOR")
            .unwrap_or_else(|_| DEFAULT_EDITOR.to_string());

        let tmp_file_path =
            format!("{}.md", run_command("mktemp", &["-u"])?.1.trim());

        let mut fd = std::fs::File::create(&tmp_file_path)?;
        #[rustfmt::skip]
        fd.write_all(
            format!(
                "\n\n\
                {COMMENT_PREFIX}OpenAI API connected to: {}{COMMENT_POSTFIX}\n\
                {COMMENT_PREFIX}Model: {}{COMMENT_POSTFIX}\n\
                {COMMENT_PREFIX}Please type your questions above, \
                save and quit{COMMENT_POSTFIX}\n",
                client.uri,
                client.model,
            ).as_bytes(),
        )?;

        spawn_editor(&editor, &tmp_file_path)?;

        let mut question = std::fs::read_to_string(&tmp_file_path)?
            .lines()
            .filter(|line| !line.starts_with(COMMENT_PREFIX))
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .to_string();

        if question.is_empty() {
            return Err(TheyaError::new(
                ErrorKind::AiInvalidReply,
                "Got empty question, quitting".into(),
            ));
        }

        if !is_slow {
            question.push_str(" (make the answer short)");
        }

        log::debug!("Question is:```\n{question}\n```");

        log::trace!("System prompt:\n{system_prompt}");
        log::trace!("Prompt:\n{question}");

        let reply = client.generate_ai_response(question.clone()).await?;
        log::trace!("Reply content:\n{}", reply.response);

        let elapsed = std::time::Duration::from_nanos(reply.total_duration_ns);

        #[rustfmt::skip]
        let mut output = format!(
            "# Question\n\
            ```\n\
            {question}\n\
            ```\n\
            # Time spent: {:.02} seconds.\n\n\
            # Answer\n\n\
            {}\n",
            elapsed.as_secs_f64(),
            reply.response
        ).lines().map(|line| line.trim_end()).collect::<Vec<&str>>().join("\n");

        output.push('\n');

        let mut fd = std::fs::File::create(&tmp_file_path)?;
        fd.write_all(output.as_bytes())?;

        drop(client);

        spawn_editor(&editor, &tmp_file_path)?;

        std::fs::remove_file(&tmp_file_path)?;

        Ok(())
    }
}
