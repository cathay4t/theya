// SPDX-License-Identifier: Apache-2.0

use super::db::VectorStore;
use crate::error::TheyaError;

pub(super) async fn handle_list(db: &VectorStore) -> Result<(), TheyaError> {
    let entries = db.dump().await?;

    if entries.is_empty() {
        log::info!("No entries in knowledge database.");
        return Ok(());
    }

    println!("{:<37} {:<25} {:<50}", "ID", "Created At", "Title");
    println!("{}", "-".repeat(130));

    for entry in entries {
        println!(
            "{:<37} {:<25} {:<50}",
            entry.id,
            entry.created_at,
            if entry.title.len() > 40 {
                format!("{}…", &entry.title[..37])
            } else {
                entry.title.clone()
            }
        );
    }

    Ok(())
}
