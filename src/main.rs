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
    let prompt = generate_patch_review_request()?;
    println!("{}", ask_ai(prompt).await?.response);
    Ok(())
}

fn generate_patch_review_request() -> Result<String, Box<dyn std::error::Error>>
{
    let gs = MyGitStore::new(std::env::current_dir()?);
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

async fn ask_ai(
    prompt: String,
) -> Result<OllamaGenerateResponse, Box<dyn std::error::Error>> {
    let client = OllamaClient::new("http://localhost:11434");

    // Check whether ollama service is running
    println!("Ollama version {}", client.version().await?);

    // Generate response
    let request = OllamaGenerate {
        model: "qwen3-coder:30b".to_string(),
        prompt,
        system: "You are a Linux software engineer reviewing patches."
            .to_string(),
        context: vec![102400],
        keep_alive: "0".into(),
        stream: false,
        options: Some(OllamaGenerateOptions {
            temperature: Some(1.0),
            num_ctx: Some(102400),
            num_predict: Some(-1),
            ..Default::default()
        }),
    };

    Ok(client.generate(&request).await?)
}
