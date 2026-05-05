// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use arrow_array::{
    Array, FixedSizeListArray, Float32Array, RecordBatch, StringArray,
    types::Float32Type,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase, Select};
use serde::{Deserialize, Serialize};

use crate::error::TheyaError;

const TABLE_NAME: &str = "memories";
const METADATA_TABLE: &str = "metadata";

pub(super) struct MemoryEntry {
    pub(super) id: String,
    pub(super) source: String,
    pub(super) role: String,
    pub(super) title: String,
    pub(super) content: String,
    pub(super) created_at: String,
}

pub(super) struct SearchResult {
    pub(super) source: String,
    pub(super) role: String,
    pub(super) title: String,
    pub(super) content: String,
    pub(super) distance: f32,
}

#[derive(Serialize, Deserialize)]
pub(super) struct DumpEntry {
    pub(super) id: String,
    pub(super) source: String,
    pub(super) role: String,
    pub(super) title: String,
    pub(super) content: String,
    pub(super) created_at: String,
    pub(super) vector: Vec<f32>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct DumpFile {
    pub(super) version: u32,
    #[serde(default)]
    pub(super) embed_model: String,
    pub(super) entries: Vec<DumpEntry>,
}

pub(super) struct VectorStore {
    conn: lancedb::Connection,
}

impl VectorStore {
    pub(super) async fn open(db_path: &str) -> Result<Self, TheyaError> {
        let conn = lancedb::connect(db_path).execute().await?;
        Ok(Self { conn })
    }

    fn schema(dim: i32) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("source", DataType::Utf8, false),
            Field::new("role", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dim,
                ),
                false,
            ),
            Field::new("created_at", DataType::Utf8, false),
        ]))
    }

    async fn table_exists(&self) -> Result<bool, TheyaError> {
        let names = self.conn.table_names().execute().await?;
        Ok(names.contains(&TABLE_NAME.to_string()))
    }

    fn metadata_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("key", DataType::Utf8, false),
            Field::new("value", DataType::Utf8, false),
        ]))
    }

    async fn metadata_table_exists(&self) -> Result<bool, TheyaError> {
        let names = self.conn.table_names().execute().await?;
        Ok(names.contains(&METADATA_TABLE.to_string()))
    }

    /// Read a value from the metadata table. Returns `None` if the key does
    /// not exist or the table has not been created yet.
    async fn get_metadata(
        &self,
        key: &str,
    ) -> Result<Option<String>, TheyaError> {
        if !self.metadata_table_exists().await? {
            return Ok(None);
        }
        let table = self.conn.open_table(METADATA_TABLE).execute().await?;
        let batches = table
            .query()
            .select(Select::columns(&["key", "value"]))
            .execute()
            .await?
            .try_collect::<Vec<_>>()
            .await?;
        for batch in &batches {
            let keys = str_col(batch, "key");
            let values = str_col(batch, "value");
            for (k, v) in keys.iter().zip(values.iter()) {
                if *k == key {
                    return Ok(Some((*v).to_string()));
                }
            }
        }
        Ok(None)
    }

    /// Insert or replace a key/value pair in the metadata table.
    async fn set_metadata(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), TheyaError> {
        let schema = Self::metadata_schema();
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec![key])),
                Arc::new(StringArray::from(vec![value])),
            ],
        )?;
        if self.metadata_table_exists().await? {
            let table = self.conn.open_table(METADATA_TABLE).execute().await?;
            table.delete(&format!("key = '{key}'")).await?;
            table.add(batch).execute().await?;
        } else {
            self.conn
                .create_table(METADATA_TABLE, batch)
                .execute()
                .await?;
        }
        Ok(())
    }

    async fn add_entries(
        &self,
        entries: Vec<MemoryEntry>,
        embeddings: Vec<Vec<f32>>,
    ) -> Result<(), TheyaError> {
        if entries.is_empty() {
            return Ok(());
        }

        let dim = embeddings
            .first()
            .map(|e| e.len() as i32)
            .unwrap_or_default();
        let batch = make_record_batch(&entries, &embeddings, dim)?;

        if self.table_exists().await? {
            let table = self.conn.open_table(TABLE_NAME).execute().await?;
            table.add(batch).execute().await?;
        } else {
            self.conn.create_table(TABLE_NAME, batch).execute().await?;
        }

        Ok(())
    }

    pub(super) async fn search(
        &self,
        query_embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<SearchResult>, TheyaError> {
        if !self.table_exists().await? {
            return Ok(Vec::new());
        }

        let table = self.conn.open_table(TABLE_NAME).execute().await?;

        let batches = table
            .query()
            .nearest_to(query_embedding.as_slice())?
            .limit(limit)
            .execute()
            .await?
            .try_collect::<Vec<_>>()
            .await?;

        let mut results = Vec::new();
        for batch in &batches {
            let n = batch.num_rows();
            let sources = str_col(batch, "source");
            let roles = str_col(batch, "role");
            let titles = str_col(batch, "title");
            let contents = str_col(batch, "content");
            let distances = f32_col(batch, "_distance");

            for i in 0..n {
                results.push(SearchResult {
                    source: sources.get(i).copied().unwrap_or("").to_string(),
                    role: roles.get(i).copied().unwrap_or("").to_string(),
                    title: titles.get(i).copied().unwrap_or("").to_string(),
                    content: contents.get(i).copied().unwrap_or("").to_string(),
                    distance: distances.get(i).copied().unwrap_or(f32::MAX),
                });
            }
        }

        Ok(results)
    }

    /// Return the stored `last_update` value as Unix seconds, or `0` if no
    /// previous run has been recorded.
    pub(super) async fn get_unix_epoch_timestamp(&self) -> u64 {
        self.get_metadata("last_update")
            .await
            .ok()
            .flatten()
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    /// Persist `ts` as the `last_update` Unix-seconds timestamp.
    pub(super) async fn set_unix_epoch_timestamp(
        &self,
        ts: u64,
    ) -> Result<(), TheyaError> {
        self.set_metadata("last_update", &ts.to_string()).await
    }

    /// Return the embedding model name recorded in metadata, if any.
    pub(super) async fn get_embed_model(
        &self,
    ) -> Result<Option<String>, TheyaError> {
        self.get_metadata("embed_model").await
    }

    /// Persist the embedding model name in metadata.
    pub(super) async fn set_embed_model(
        &self,
        model: &str,
    ) -> Result<(), TheyaError> {
        self.set_metadata("embed_model", model).await
    }

    /// Embed `entries` using `ai` and persist them in the store.
    /// Also records `model` in metadata so future runs can detect model
    /// changes.
    pub(super) async fn store_knowledge(
        &self,
        ai: &crate::openai::OpenAiClient,
        entries: Vec<MemoryEntry>,
        dimensions: Option<u32>,
        model: &str,
    ) -> Result<(), TheyaError> {
        if entries.is_empty() {
            return Ok(());
        }
        let texts: Vec<&str> =
            entries.iter().map(|e| e.content.as_str()).collect();
        let embeddings = ai.embed_texts(&texts, dimensions).await?;
        self.add_entries(entries, embeddings).await?;
        self.set_embed_model(model).await
    }

    /// Re-embed every stored entry with the current `ai` client and replace
    /// the vectors in-place. Returns the number of entries recalculated.
    pub(super) async fn recalc_vectors(
        &self,
        ai: &crate::openai::OpenAiClient,
        dimensions: Option<u32>,
        model: &str,
    ) -> Result<usize, TheyaError> {
        let mut entries = self.dump().await?;
        if entries.is_empty() {
            return Ok(0);
        }
        let texts: Vec<&str> =
            entries.iter().map(|e| e.content.as_str()).collect();
        let embeddings = ai.embed_texts(&texts, dimensions).await?;
        for (entry, emb) in entries.iter_mut().zip(embeddings) {
            entry.vector = emb;
        }
        self.wipe().await?;
        let n = entries.len();
        self.load_entries(entries).await?;
        self.set_embed_model(model).await?;
        Ok(n)
    }

    /// Read every row from the memories table and return them as
    /// [`DumpEntry`] values (including their embedding vectors).
    pub(super) async fn dump(&self) -> Result<Vec<DumpEntry>, TheyaError> {
        if !self.table_exists().await? {
            return Ok(Vec::new());
        }
        let table = self.conn.open_table(TABLE_NAME).execute().await?;
        let batches = table
            .query()
            .execute()
            .await?
            .try_collect::<Vec<_>>()
            .await?;

        let mut out = Vec::new();
        for batch in &batches {
            let ids = str_col(batch, "id");
            let sources = str_col(batch, "source");
            let roles = str_col(batch, "role");
            let titles = str_col(batch, "title");
            let contents = str_col(batch, "content");
            let created_ats = str_col(batch, "created_at");
            let vectors = vector_col(batch, "vector");
            for i in 0..batch.num_rows() {
                out.push(DumpEntry {
                    id: ids.get(i).copied().unwrap_or("").to_string(),
                    source: sources.get(i).copied().unwrap_or("").to_string(),
                    role: roles.get(i).copied().unwrap_or("").to_string(),
                    title: titles.get(i).copied().unwrap_or("").to_string(),
                    content: contents.get(i).copied().unwrap_or("").to_string(),
                    created_at: created_ats
                        .get(i)
                        .copied()
                        .unwrap_or("")
                        .to_string(),
                    vector: vectors.get(i).cloned().unwrap_or_default(),
                });
            }
        }
        Ok(out)
    }

    /// Drop the memories table, effectively wiping all stored entries.
    pub(super) async fn wipe(&self) -> Result<(), TheyaError> {
        if self.table_exists().await? {
            self.conn.drop_table(TABLE_NAME, &[]).await?;
        }
        Ok(())
    }

    /// Insert pre-computed [`DumpEntry`] values (with their embedding
    /// vectors already embedded) directly into the store.
    pub(super) async fn load_entries(
        &self,
        entries: Vec<DumpEntry>,
    ) -> Result<(), TheyaError> {
        if entries.is_empty() {
            return Ok(());
        }
        let mem: Vec<MemoryEntry> = entries
            .iter()
            .map(|e| MemoryEntry {
                id: e.id.clone(),
                source: e.source.clone(),
                role: e.role.clone(),
                title: e.title.clone(),
                content: e.content.clone(),
                created_at: e.created_at.clone(),
            })
            .collect();
        let vecs: Vec<Vec<f32>> =
            entries.into_iter().map(|e| e.vector).collect();
        self.add_entries(mem, vecs).await
    }
}

fn str_col<'a>(batch: &'a RecordBatch, name: &str) -> Vec<&'a str> {
    batch
        .column_by_name(name)
        .and_then(|col| col.as_any().downcast_ref::<StringArray>())
        .map(|arr| {
            (0..arr.len())
                .map(|i| if arr.is_null(i) { "" } else { arr.value(i) })
                .collect()
        })
        .unwrap_or_default()
}

fn vector_col(batch: &RecordBatch, name: &str) -> Vec<Vec<f32>> {
    batch
        .column_by_name(name)
        .and_then(|col| col.as_any().downcast_ref::<FixedSizeListArray>())
        .map(|arr| {
            (0..arr.len())
                .map(|i| {
                    arr.value(i)
                        .as_any()
                        .downcast_ref::<Float32Array>()
                        .map(|fa| (0..fa.len()).map(|j| fa.value(j)).collect())
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn f32_col(batch: &RecordBatch, name: &str) -> Vec<f32> {
    batch
        .column_by_name(name)
        .and_then(|col| col.as_any().downcast_ref::<Float32Array>())
        .map(|arr| {
            (0..arr.len())
                .map(|i| {
                    if arr.is_null(i) {
                        f32::MAX
                    } else {
                        arr.value(i)
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn make_record_batch(
    entries: &[MemoryEntry],
    embeddings: &[Vec<f32>],
    dim: i32,
) -> Result<RecordBatch, TheyaError> {
    let schema = VectorStore::schema(dim);

    let ids = StringArray::from(
        entries.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
    );
    let sources = StringArray::from(
        entries
            .iter()
            .map(|e| e.source.as_str())
            .collect::<Vec<_>>(),
    );
    let roles = StringArray::from(
        entries.iter().map(|e| e.role.as_str()).collect::<Vec<_>>(),
    );
    let titles = StringArray::from(
        entries.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(),
    );
    let contents = StringArray::from(
        entries
            .iter()
            .map(|e| e.content.as_str())
            .collect::<Vec<_>>(),
    );
    let created_ats = StringArray::from(
        entries
            .iter()
            .map(|e| e.created_at.as_str())
            .collect::<Vec<_>>(),
    );

    let vectors = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
        embeddings
            .iter()
            .map(|emb| Some(emb.iter().copied().map(Some))),
        dim,
    );

    Ok(RecordBatch::try_new(
        schema,
        vec![
            Arc::new(ids),
            Arc::new(sources),
            Arc::new(roles),
            Arc::new(titles),
            Arc::new(contents),
            Arc::new(vectors),
            Arc::new(created_ats),
        ],
    )?)
}
