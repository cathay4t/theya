// SPDX-License-Identifier: Apache-2.0

use super::{
    copilot::CopilotHistoryStore,
    db::{MemoryEntry, VectorStore},
};
use crate::{config::TheyaConfig, error::TheyaError, openai::OpenAiClient};

const EXTRACTION_GUIDELINE: &str =
    "You are a knowledge extraction assistant. Analyze conversations and \
     extract key facts, technical decisions, insights, and learnings worth \
     preserving as long-term memory. Be concise and structured. If there is \
     nothing worth preserving, reply with an empty response.";

const EXTRACTION_PROMPT: &str = "Extract the important knowledge from this \
                                 conversation that should be stored as \
                                 long-term memory for future reference:";

// for any reply less than MIN_KNOWLEDGE_LEN,
const MIN_KNOWLEDGE_LEN: usize = 64;

pub(super) async fn handle_update(
    db: &VectorStore,
    home: &str,
    config: &TheyaConfig,
) -> Result<(), TheyaError> {
    if !config.memory.copilot {
        log::info!(
            "Copilot history indexing is disabled. Set `copilot = true` under \
             [memory] in your config."
        );
        return Ok(());
    }

    let since_ts = db.get_unix_epoch_timestamp().await;

    let histories =
        CopilotHistoryStore::new(home).get_sessions_since(since_ts)?;
    if histories.is_empty() {
        log::info!("Memory is up to date — no new sessions to process.");
        return Ok(());
    }

    let chat_ai = OpenAiClient::new(
        &config.memory.uri,
        &config.memory.model,
        EXTRACTION_GUIDELINE,
        &config.memory.api_key,
        None,
    )
    .await?;

    let mut new_entries: Vec<MemoryEntry> = Vec::new();

    for history in &histories {
        let prefix = &history.session_id[..8.min(history.session_id.len())];
        log::info!("Extracting knowledge from session {prefix}...");

        let prompt =
            format!("{EXTRACTION_PROMPT}\n\n{}", history.format_conversation());
        let knowledge = chat_ai.generate_ai_response(prompt).await?.response;
        let knowledge = knowledge.trim().to_string();
        if knowledge.len() < MIN_KNOWLEDGE_LEN {
            if knowledge.is_empty() {
                log::info!("No important knowledge found in session {prefix}.");
            } else {
                log::info!("Ignore short reply {knowledge}");
            }
            continue;
        }

        log::info!("Got knowledge {knowledge}");

        new_entries.push(MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            source: format!("copilot:{}", history.session_id),
            role: "knowledge".to_string(),
            title: format!("Copilot session {prefix}"),
            content: knowledge,
            created_at: history.max_timestamp().to_string(),
        });
    }

    if new_entries.is_empty() {
        log::info!("Memory is up to date — no new knowledge to index.");
    } else {
        log::info!(
            "Indexing {} session(s) of extracted knowledge...",
            new_entries.len()
        );
        let embed_ai = OpenAiClient::new(
            &config.memory.embed_uri,
            &config.memory.embed_model,
            "",
            &config.memory.api_key,
            None,
        )
        .await?;
        db.store_knowledge(
            &embed_ai,
            new_entries,
            config.memory.embed_dimensions,
            &config.memory.embed_model,
        )
        .await?;
        log::info!("Done.");
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    db.set_unix_epoch_timestamp(now).await?;

    Ok(())
}
