// SPDX-License-Identifier: Apache-2.0

use super::db::VectorStore;
use crate::{config::TheyaConfig, error::TheyaError, openai::OpenAiClient};

const VECTOR_DB_PATH: &str = ".local/share/theya/knowledge_db";

pub(crate) struct CommandMemory;

impl CommandMemory {
    pub(crate) const CMD: &str = "memory";

    pub(crate) fn new_cmd() -> clap::Command {
        clap::Command::new(Self::CMD)
            .alias("m")
            .about("Manage long-term memory backed by a local vector database")
            .subcommand_required(true)
            .subcommand(
                clap::Command::new("update").about(
                    "Include agents chat history into the vector database",
                ),
            )
            .subcommand(
                clap::Command::new("add")
                    .about("Store a file or interactive note into memory")
                    .arg(
                        clap::Arg::new("FILE")
                            .help("Path of the file to store")
                            .required(false),
                    )
                    .arg(
                        clap::Arg::new("interactive")
                            .long("interactive")
                            .short('i')
                            .action(clap::ArgAction::SetTrue)
                            .help("Open $EDITOR to compose a note")
                            .conflicts_with("FILE"),
                    ),
            )
            .subcommand(
                clap::Command::new("search")
                    .about("Similarity-search memory with a prompt")
                    .arg(
                        clap::Arg::new("PROMPT")
                            .help("Natural-language search prompt")
                            .required(true)
                            .num_args(1..),
                    ),
            )
            .subcommand(
                clap::Command::new("dump")
                    .about("Dump the vector database to a JSON file")
                    .arg(
                        clap::Arg::new("FILE")
                            .help("Output file path")
                            .required(true),
                    ),
            )
            .subcommand(
                clap::Command::new("load")
                    .about("Restore the vector database from a JSON dump file")
                    .arg(
                        clap::Arg::new("FILE")
                            .help("Input dump file path")
                            .required(true),
                    ),
            )
            .subcommand(clap::Command::new("recalc").about(
                "Recalculate all stored embedding vectors using the current \
                 embedding model",
            ))
    }

    pub(crate) async fn handle(
        matches: &clap::ArgMatches,
        config: &TheyaConfig,
    ) -> Result<(), TheyaError> {
        let home = std::env::var("HOME").map_err(|_| {
            TheyaError::from("HOME environment variable not set")
        })?;

        let db_path = format!("{home}/{VECTOR_DB_PATH}");
        std::fs::create_dir_all(&db_path)?;

        let db = VectorStore::open(&db_path).await?;

        // Auto-recalculate vectors when the configured embedding model has
        // changed since the last run.  Skip for dump/load/recalc where the
        // stored vectors should be left untouched or are being managed
        // explicitly.
        let subcmd = matches.subcommand_name().unwrap_or("");
        if !matches!(subcmd, "dump" | "load" | "recalc")
            && let Some(stored) = db.get_embed_model().await?
            && stored != config.memory.embed_model
        {
            log::info!(
                "Embedding model changed ('{stored}' → '{}'), recalculating \
                 stored vectors...",
                config.memory.embed_model
            );
            let ai = OpenAiClient::new(
                &config.memory.embed_uri,
                &config.memory.embed_model,
                "",
                &config.memory.api_key,
                None,
            )
            .await?;
            let n = db
                .recalc_vectors(
                    &ai,
                    config.memory.embed_dimensions,
                    &config.memory.embed_model,
                )
                .await?;
            log::info!("Recalculated {n} entries.");
        }

        if matches.subcommand_matches("update").is_some() {
            super::update::handle_update(&db, &home, config).await
        } else if let Some(m) = matches.subcommand_matches("add") {
            super::add::handle_add(&db, m, config).await
        } else if let Some(m) = matches.subcommand_matches("search") {
            super::search::handle_search(&db, m, config).await
        } else if let Some(m) = matches.subcommand_matches("dump") {
            super::dump::handle_dump(&db, m).await
        } else if let Some(m) = matches.subcommand_matches("load") {
            super::load::handle_load(&db, m).await
        } else if matches.subcommand_matches("recalc").is_some() {
            super::recalc::handle_recalc(&db, config).await
        } else {
            Err(TheyaError::from("Unknown memory subcommand"))
        }
    }
}
