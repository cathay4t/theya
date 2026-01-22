// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Write, io::Write as IoWrite};

use super::{error::CliError, git::MyGitStore, ollama::OllamaClient};

const DEFAULT_MODULE: &str = "qwen3-coder:30b";
const DEFAULT_URI: &str = "http://localhost:11434";

pub(crate) struct CommandPatchReview;

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
        let uri = std::env::var("THEYA_URI")
            .unwrap_or_else(|_| DEFAULT_URI.to_string());
        let model = std::env::var("THEYA_MODULE")
            .unwrap_or_else(|_| DEFAULT_MODULE.to_string());

        let client = OllamaClient::new(&uri);

        log::info!("Ollama URI: {uri}");
        log::info!("Ollama version {}", client.version().await?);
        log::info!("Module name {model}");
        log::debug!("========== Patch Content =========");
        log::debug!("{patch_content}");
        print!(
            "========== Reviewing: {} =============",
            gs.get_cur_patch_titile()?.trim()
        );
        std::io::stdout().flush().ok();

        log::trace!("Prompt:\n{prompt}");

        let reply = client
            .generate_ai_response(model, prompt, 10240, -1)
            .await?
            .response;

        print!("\r");
        print!("========== Review Result =========\n");
        print!("{}", reply);
        Ok(())
    }
}

fn generate_patch_review_request(gs: &MyGitStore) -> Result<String, CliError> {
    let patch_content = gs.get_cur_patch_content()?;
    let mut ret = format!(
        "You are a Linux software engineer reviewing provided patch. Please \
         only include improvement suggestions without making summary on what \
         current patch is doing. Please include code snippet for the \
         improvement when possible. Please check typo in function name, \
         variable name and commit message. This is the patch content:\n \
         \"\"\"\n {patch_content}\n \"\"\"\n You may also take these changed \
         files as review context:\n"
    );
    for changed_file in gs.get_cur_changed_file_paths()? {
        let content = gs.get_file_content(&changed_file)?;
        write!(
            ret,
            "file path: {}\nfile content:\"\"\"\n{content}\n\"\"\"\n",
            changed_file.display()
        )
        .ok();
    }
    Ok(ret)
}
