// SPDX-License-Identifier: Apache-2.0

use std::io::Write;

use super::db::{DumpFile, VectorStore};
use crate::error::TheyaError;

pub(super) async fn handle_load(
    db: &VectorStore,
    matches: &clap::ArgMatches,
) -> Result<(), TheyaError> {
    let input_path = matches.get_one::<String>("FILE").unwrap();

    let content = std::fs::read_to_string(input_path)?;
    let dump: DumpFile = serde_json::from_str(&content).map_err(|e| {
        TheyaError::from(format!("Failed to parse dump file: {e}"))
    })?;
    let n = dump.entries.len();

    print!("Wipe existing database before loading {n} entries? [y/N] ");
    std::io::stdout().flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;

    if answer.trim().eq_ignore_ascii_case("y") {
        db.wipe().await?;
        log::info!("Existing database wiped.");
    }

    db.load_entries(dump.entries).await?;
    if !dump.embed_model.is_empty() {
        db.set_embed_model(&dump.embed_model).await?;
    }
    log::info!("Loaded {n} entries from '{input_path}'.");
    Ok(())
}
