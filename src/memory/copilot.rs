// SPDX-License-Identifier: Apache-2.0

use crate::error::TheyaError;

const COPILOT_SESSION_DIR: &str = ".copilot/session-state";

pub(super) struct AgentHistory {
    pub(super) role: String,
    pub(super) content: String,
    pub(super) timestamp: String,
}

pub(super) struct CopilotHistory {
    pub(super) session_id: String,
    pub(super) messages: Vec<AgentHistory>,
}

impl CopilotHistory {
    /// Return the latest message timestamp, or `""` if none are recorded.
    pub(super) fn max_timestamp(&self) -> &str {
        self.messages
            .iter()
            .map(|m| m.timestamp.as_str())
            .filter(|ts| !ts.is_empty())
            .max()
            .unwrap_or("")
    }

    /// Format the conversation as `"User: …\n\nAssistant: …"` text.
    pub(super) fn format_conversation(&self) -> String {
        self.messages
            .iter()
            .map(|m| {
                let label = if m.role == "user" {
                    "User"
                } else {
                    "Assistant"
                };
                format!("{label}: {}", m.content)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// Provides access to Copilot session histories stored under `home_dir`.
pub(super) struct CopilotHistoryStore {
    home_dir: String,
}

impl CopilotHistoryStore {
    pub(super) fn new(home_dir: impl Into<String>) -> Self {
        Self {
            home_dir: home_dir.into(),
        }
    }

    /// Return histories for sessions whose directory mtime is strictly after
    /// `since_ts` (Unix seconds). Sessions with no parseable messages are
    /// omitted. Directories without a readable mtime are always included.
    pub(super) fn get_sessions_since(
        &self,
        since_ts: u64,
    ) -> Result<Vec<CopilotHistory>, TheyaError> {
        let session_dir = format!("{}/{COPILOT_SESSION_DIR}", self.home_dir);
        if !std::path::Path::new(&session_dir).exists() {
            return Err(TheyaError::from(format!(
                "Copilot session directory not found: {session_dir}"
            )));
        }

        let mut histories = Vec::new();
        for dir_entry in std::fs::read_dir(&session_dir)? {
            let dir_entry = dir_entry?;
            if !dir_entry.file_type()?.is_dir() {
                continue;
            }

            let mtime = dir_entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs());

            if mtime.is_some_and(|m| m <= since_ts) {
                continue;
            }

            let session_id =
                dir_entry.file_name().to_string_lossy().to_string();
            let events_path =
                format!("{session_dir}/{session_id}/events.jsonl");
            if !std::path::Path::new(&events_path).exists() {
                continue;
            }

            let content = std::fs::read_to_string(&events_path)?;
            let messages = parse_copilot_events(&content);
            if messages.is_empty() {
                continue;
            }

            histories.push(CopilotHistory {
                session_id,
                messages,
            });
        }
        Ok(histories)
    }
}

/// Extract user and assistant text messages from a copilot `events.jsonl`
/// file. Tool-call-only messages and sub-agent messages are excluded.
fn parse_copilot_events(content: &str) -> Vec<AgentHistory> {
    let mut messages = Vec::new();

    for line in content.lines() {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };

        let kind = match event["type"].as_str() {
            Some(k) => k,
            None => continue,
        };
        let data = &event["data"];
        let timestamp = event["timestamp"].as_str().unwrap_or("");

        match kind {
            "user.message" => {
                let content = data["content"].as_str().unwrap_or("").trim();
                if content.is_empty() {
                    continue;
                }
                messages.push(AgentHistory {
                    role: "user".to_string(),
                    content: content.to_string(),
                    timestamp: timestamp.to_string(),
                });
            }
            "assistant.message" => {
                if !data["parentToolCallId"].is_null() {
                    continue;
                }
                let content = data["content"].as_str().unwrap_or("").trim();
                if content.is_empty() {
                    continue;
                }
                messages.push(AgentHistory {
                    role: "assistant".to_string(),
                    content: content.to_string(),
                    timestamp: timestamp.to_string(),
                });
            }
            _ => {}
        }
    }

    messages
}
