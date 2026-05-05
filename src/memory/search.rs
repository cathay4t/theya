// SPDX-License-Identifier: Apache-2.0

use super::db::VectorStore;
use crate::{config::TheyaConfig, error::TheyaError, openai::OpenAiClient};

/// Number of search results to return
const SEARCH_LIMIT: usize = 5;

pub(super) async fn handle_search(
    db: &VectorStore,
    matches: &clap::ArgMatches,
    config: &TheyaConfig,
) -> Result<(), TheyaError> {
    let prompt = matches
        .get_many::<String>("PROMPT")
        .unwrap_or_default()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" ");

    let ai = OpenAiClient::new(
        &config.memory.embed_uri,
        &config.memory.embed_model,
        "",
        &config.memory.api_key,
        None,
    )
    .await?;
    let embeddings = ai
        .embed_texts(&[prompt.as_str()], config.memory.embed_dimensions)
        .await?;
    let query_vec = embeddings.into_iter().next().unwrap_or_default();

    let results = db.search(query_vec, SEARCH_LIMIT).await?;

    if results.is_empty() {
        log::info!("No relevant memories found.");
        return Ok(());
    }

    log::info!("Search results for: {prompt}\n");
    for (i, r) in results.iter().enumerate() {
        log::info!(
            "--- Result {} (distance: {:.4}) ---\nRole:    {}\nSource:  \
             {}\nTitle:   {}\nContent:\n{}\n",
            i + 1,
            r.distance,
            r.role,
            r.source,
            r.title,
            r.content,
        );
    }

    Ok(())
}
