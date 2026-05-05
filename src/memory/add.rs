// SPDX-License-Identifier: Apache-2.0

use std::io::Write;

use super::db::{MemoryEntry, VectorStore};
use crate::{
    cmd::{run_command, spawn_editor},
    config::TheyaConfig,
    error::{ErrorKind, TheyaError},
    openai::OpenAiClient,
};

const DEFAULT_EDITOR: &str = "vim";
/// Maximum characters per chunk when splitting large documents
const CHUNK_SIZE: usize = 1500;
/// Characters of overlap between consecutive chunks
const CHUNK_OVERLAP: usize = 200;

pub(super) async fn handle_add(
    db: &VectorStore,
    matches: &clap::ArgMatches,
    config: &TheyaConfig,
) -> Result<(), TheyaError> {
    let (content, source, title) = if matches.get_flag("interactive") {
        let editor = std::env::var("EDITOR")
            .unwrap_or_else(|_| DEFAULT_EDITOR.to_string());

        let tmp_path =
            format!("{}.txt", run_command("mktemp", &["-u"])?.1.trim());
        let mut fd = std::fs::File::create(&tmp_path)?;
        fd.write_all(
            b"# Enter the content you want to memorize.\n\
              # Lines beginning with '#' are ignored.\n\n",
        )?;
        drop(fd);

        spawn_editor(&editor, &tmp_path)?;

        let raw = std::fs::read_to_string(&tmp_path)?;
        std::fs::remove_file(&tmp_path)?;

        let content: String = raw
            .lines()
            .filter(|l| !l.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        if content.is_empty() {
            return Err(TheyaError::new(
                ErrorKind::EmptyInput,
                "No content provided".to_string(),
            ));
        }

        let id = uuid::Uuid::new_v4().to_string();
        (
            content,
            format!("interactive:{id}"),
            "interactive note".to_string(),
        )
    } else if let Some(file_path) = matches.get_one::<String>("FILE") {
        let abs_path = std::fs::canonicalize(file_path)
            .map_err(|e| {
                TheyaError::from(format!(
                    "Cannot resolve path '{file_path}': {e}"
                ))
            })?
            .to_string_lossy()
            .to_string();

        let content = std::fs::read_to_string(&abs_path)?;
        let title = std::path::Path::new(&abs_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| abs_path.clone());

        (content, format!("file:{abs_path}"), title)
    } else {
        return Err(TheyaError::from(
            "Provide a file path or pass --interactive",
        ));
    };

    let chunks = chunk_text(&content, CHUNK_SIZE, CHUNK_OVERLAP);
    let n_chunks = chunks.len();
    let created_at = unix_timestamp();

    let entries: Vec<MemoryEntry> = chunks
        .into_iter()
        .enumerate()
        .map(|(i, chunk)| {
            let chunk_source = if n_chunks == 1 {
                source.clone()
            } else {
                format!("{source}:chunk-{i}")
            };
            MemoryEntry {
                id: uuid::Uuid::new_v4().to_string(),
                source: chunk_source,
                role: "note".to_string(),
                title: title.clone(),
                content: chunk,
                created_at: created_at.clone(),
            }
        })
        .collect();

    log::info!("Storing {} chunk(s) from '{title}'...", entries.len());
    let ai = OpenAiClient::new(
        &config.memory.embed_uri,
        &config.memory.embed_model,
        "",
        &config.memory.api_key,
        None,
    )
    .await?;
    db.store_knowledge(
        &ai,
        entries,
        config.memory.embed_dimensions,
        &config.memory.embed_model,
    )
    .await?;
    log::info!("Done.");

    Ok(())
}

/// Split `text` into overlapping chunks of at most `max_chars` characters.
fn chunk_text(text: &str, max_chars: usize, overlap: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        return vec![text.to_string()];
    }
    let step = max_chars.saturating_sub(overlap).max(1);
    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        chunks.push(chars[start..end].iter().collect::<String>());
        if end == chars.len() {
            break;
        }
        start += step;
    }
    chunks
}

fn unix_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
