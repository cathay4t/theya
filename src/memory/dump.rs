// SPDX-License-Identifier: Apache-2.0

use super::db::{DumpFile, VectorStore};
use crate::error::TheyaError;

pub(super) async fn handle_dump(
    db: &VectorStore,
    matches: &clap::ArgMatches,
) -> Result<(), TheyaError> {
    let output_path = matches.get_one::<String>("FILE").unwrap();

    let embed_model = db.get_embed_model().await?.unwrap_or_default();
    let entries = db.dump().await?;
    let n = entries.len();
    let dump = DumpFile {
        version: 1,
        embed_model,
        entries,
    };

    let json = serde_json::to_string(&dump).map_err(|e| {
        TheyaError::from(format!("Failed to serialize dump: {e}"))
    })?;
    std::fs::write(output_path, json)?;

    log::info!("Dumped {n} entries to '{output_path}'.");
    Ok(())
}
