// SPDX-License-Identifier: Apache-2.0

mod git;
mod ollama;

use std::fmt::Write;

use crate::{
    git::MyGitStore,
    ollama::{
        OllamaClient, OllamaGenerate, OllamaGenerateOptions,
        OllamaGenerateResponse,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gs = MyGitStore::new(std::env::current_dir()?);
    let patch_content = gs.get_cur_patch_content()?;
    let prompt = generate_patch_review_request(gs)?;
    let uri =
        std::env::var("THEYA_URI").unwrap_or_else(|_| DEFAULT_URI.to_string());
    let model = std::env::var("THEYA_MODULE")
        .unwrap_or_else(|_| DEFAULT_MODULE.to_string());

    let client = OllamaClient::new(&uri);

    println!("Connecting to Ollama service at {uri}");
    println!("Ollama version {}", client.version().await?);
    println!("Module name {model}");
    println!("========== Patch Reviewing     =========");
    println!("{patch_content}");
    println!("========== Patch Reviewing     =========");

    let reply = ask_ai(&client, model, prompt).await?.response;
    println!("========== Patch Review Result =========\n");
    println!("{}", reply);
    Ok(())
}

fn generate_patch_review_request(
    gs: MyGitStore,
) -> Result<String, Box<dyn std::error::Error>> {
    let patch_content = gs.get_cur_patch_content()?;
    let mut ret = format!(
        "Please review this patch with improvement suggestions only (do not \
         include overview, example, highlights):\n \"\"\"\n {patch_content}\n \
         \"\"\"\n You may take these full contents of changed files as review \
         context:\n"
    );
    for changed_file in gs.get_cur_changed_file_paths()? {
        let content = gs.get_file_content(&changed_file)?;
        write!(
            ret,
            "file: {}\n\"\"\"\n{content}\n\"\"\"\n",
            changed_file.display()
        )
        .ok();
    }
    Ok(ret)
}

const DEFAULT_MODULE: &str = "qwen3-coder:30b";
const DEFAULT_URI: &str = "http://localhost:11434";

async fn ask_ai(
    client: &OllamaClient,
    model: String,
    prompt: String,
) -> Result<OllamaGenerateResponse, Box<dyn std::error::Error>> {
    // Generate response
    let request = OllamaGenerate {
        model,
        prompt,
        system: "You are a Linux software engineer reviewing patches."
            .to_string(),
        keep_alive: "0".into(),
        stream: false,
        options: Some(OllamaGenerateOptions {
            temperature: Some(1.0),
            num_ctx: Some(102400),
            num_predict: Some(-1),
            ..Default::default()
        }),
    };

    client.generate(&request).await
}
