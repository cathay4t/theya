// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Write, io::Write as IoWrite};

use super::{error::CliError, git::MyGitStore, ollama::OllamaClient};

pub(crate) struct CommandPatchReview;

const CONTEXT_NUMBER: i32 = 10 * 1024 * 1024;

#[rustfmt::skip]
const SYSTEM_PROMPT: &str = "\
You are a Linux software developer reviewing provided patch.\n\
Guidelines:
1. Only include improvement suggestions without making summary \
   on what current patch is doing.\n\
2. Include code snippet for the improvement when possible.\n\
3. Check typo in function name, variable name and comments.\n\
4. Do not include suggestions for files referred as review context.\n";

impl CommandPatchReview {
    pub(crate) const CMD: &str = "patch-review";

    pub(crate) fn new_cmd() -> clap::Command {
        clap::Command::new(Self::CMD)
            .alias("pr")
            .about("Patch review")
    }

    pub(crate) async fn handle(
        _matches: &clap::ArgMatches,
    ) -> Result<(), CliError> {
        let gs = MyGitStore::new(std::env::current_dir()?);
        let patch_content = gs.get_cur_patch_content()?;
        let prompt = generate_patch_review_request(&gs)?;

        let client = OllamaClient::new().await?;

        log::debug!("========== Patch Content =========");
        log::debug!("{patch_content}");
        println!(
            "========== Reviewing: {} =============",
            gs.get_cur_patch_titile()?.trim()
        );
        std::io::stdout().flush().ok();

        log::trace!("System prompt:\n{SYSTEM_PROMPT}");
        log::trace!("Prompt:\n{prompt}");

        let reply = client
            .generate_ai_response(
                SYSTEM_PROMPT.to_string(),
                prompt,
                CONTEXT_NUMBER,
            )
            .await?
            .response;

        println!("========== Review Result =========");
        println!("{}", reply);
        Ok(())
    }
}

fn generate_patch_review_request(gs: &MyGitStore) -> Result<String, CliError> {
    let patch_content = gs.get_cur_patch_content()?;
    // TODO: Support reading guideline from git repo
    #[rustfmt::skip]
    let mut ret = format!(
        "\
        This is the patch content:\n\
        ```\n\
        {patch_content}\n\
        ```\n\
        You may also take these changed files as review context:\n"
    );
    for changed_file in gs.get_cur_changed_file_paths()? {
        let content = gs.get_file_content(&changed_file)?;
        write!(
            ret,
            "file path: {}\nfile content:```\n{content}\n```\n",
            changed_file.display()
        )
        .ok();
    }
    Ok(ret)
}
