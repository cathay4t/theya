// SPDX-License-Identifier: Apache-2.0

use std::io::Write;

use super::{
    cmd::{run_command, spawn_editor},
    error::CliError,
    ollama::OllamaClient,
};

const DEFAULT_EDITOR: &str = "vim";
const COMMENT_PREFIX: &str = "<!-- Theya: ";
const COMMENT_POSTFIX: &str = " -->";

const QUICK_CHAT_CONTEXT: i32 = 1024;
const SLOW_CHAT_CONTEXT: i32 = 128 * 1024;

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

pub(crate) struct CommandQuickChat;

impl CommandQuickChat {
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
    ) -> Result<(), CliError> {
        let (context_num, system_prompt) = if matches.get_flag("SLOW") {
            (SLOW_CHAT_CONTEXT, SLOW_SYSTEM_PROMPT)
        } else {
            (QUICK_CHAT_CONTEXT, QUICK_SYSTEM_PROMPT)
        };

        let client = OllamaClient::new().await?;

        let editor = std::env::var("EDITOR")
            .unwrap_or_else(|_| DEFAULT_EDITOR.to_string());

        let tmp_file_path = std::path::PathBuf::from(format!(
            "{}.md",
            run_command("mktemp -u", &std::env::temp_dir())?.trim()
        ));

        let mut fd = std::fs::File::create(&tmp_file_path)?;
        #[rustfmt::skip]
        fd.write_all(
            format!(
                "\n\n\
                {COMMENT_PREFIX}Ollama connected to: {}{COMMENT_POSTFIX}\n\
                {COMMENT_PREFIX}Ollama version: {}{COMMENT_POSTFIX}\n\
                {COMMENT_PREFIX}Model: {}{COMMENT_POSTFIX}\n\
                {COMMENT_PREFIX}Please type your questions above, \
                save and quit{COMMENT_POSTFIX}\n",
                client.uri,
                client.version().await?,
                client.model,
            ).as_bytes(),
        )?;

        spawn_editor(&editor, &tmp_file_path)?;

        let question = std::fs::read_to_string(&tmp_file_path)?
            .lines()
            .filter(|line| !line.starts_with(COMMENT_PREFIX))
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .to_string();

        if question.is_empty() {
            println!("Got empty question, quitting");
            return Ok(());
        }

        log::debug!("Question is:```\n{question}\n```");

        log::trace!("System prompt:\n{system_prompt}");
        log::trace!("Prompt:\n{question}");

        let reply = client
            .generate_ai_response(
                system_prompt.to_string(),
                question.clone(),
                context_num,
            )
            .await?;
        log::trace!("Reply is:\n{reply:?}");

        let elapsed = std::time::Duration::from_nanos(reply.total_duration_ns);

        #[rustfmt::skip]
        let output = format!(
            "# Question\n\
            ```\n\
            {question}\n\
            ```\n\
            # Time spent: {:.02} seconds.\n\n\
            # Answer\n\n\
            {}",
            elapsed.as_secs_f64(),
            reply.response
        ).lines().map(|line| line.trim_end()).collect::<Vec<&str>>().join("\n");

        let mut fd = std::fs::File::create(&tmp_file_path)?;
        fd.write_all(output.as_bytes())?;

        drop(client);

        spawn_editor(&editor, &tmp_file_path)?;

        std::fs::remove_file(&tmp_file_path)?;

        Ok(())
    }
}
