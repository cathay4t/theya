// SPDX-License-Identifier: Apache-2.0

use super::db::VectorStore;
use crate::{config::TheyaConfig, error::TheyaError, openai::OpenAiClient};

pub(super) async fn handle_recalc(
    db: &VectorStore,
    config: &TheyaConfig,
) -> Result<(), TheyaError> {
    let ai = OpenAiClient::new(
        &config.memory.embed_uri,
        &config.memory.embed_model,
        "",
        &config.memory.api_key,
        None,
    )
    .await?;

    log::info!(
        "Recalculating all embedding vectors using model '{}'...",
        config.memory.embed_model
    );
    let n = db
        .recalc_vectors(
            &ai,
            config.memory.embed_dimensions,
            &config.memory.embed_model,
        )
        .await?;
    log::info!("Done. Recalculated {n} entries.");
    Ok(())
}
