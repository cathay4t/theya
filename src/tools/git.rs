// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::traits::ToolHandler;
use crate::{
    cmd::run_command_checked,
    error::{ErrorKind, TheyaError},
    json_schema::JsonSchema,
};

pub(crate) struct ToolGitLog;
impl ToolHandler<Vec<String>> for ToolGitLog {
    const NAME: &str = "git_log";
    const DESCRIPTION: &str = "Get recent 10 commits of specified files with \
                               commit hash and subject of commit";

    fn parameters() -> JsonSchema {
        let mut json_schema_props: HashMap<String, Box<JsonSchema>> =
            HashMap::new();
        json_schema_props.insert(
            "file_paths".into(),
            Box::new(JsonSchema {
                kind: Some("array".into()),
                items: Some(Box::new(JsonSchema {
                    kind: Some("string".into()),
                    description: Some("file path".into()),
                    ..Default::default()
                })),
                ..Default::default()
            }),
        );
        JsonSchema {
            kind: Some("object".into()),
            properties: Some(json_schema_props),
            required: Some(vec!["file_path".to_string()]),
            ..Default::default()
        }
    }

    fn run(arguments: serde_json::Value) -> Result<Vec<String>, TheyaError> {
        if let Some(file_paths) = arguments
            .as_object()
            .and_then(|o| o.get("file_paths"))
            .and_then(|v| v.as_array())
        {
            let mut args =
                vec!["log", "--pretty=format:'%h %s'", "--max-count=10", "--"];
            for file_path in file_paths {
                if let Some(p) = file_path.as_str() {
                    args.push(p);
                }
            }

            Ok(run_command_checked("git", &args)?
                .split('\n')
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect())
        } else {
            let args =
                vec!["log", "--pretty=format:'%h %s'", "--max-count=10", "--"];
            Ok(run_command_checked("git", &args)?
                .split('\n')
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect())
        }
    }
}

pub(crate) struct ToolGit;
impl ToolHandler<String> for ToolGit {
    const NAME: &str = "git";
    const DESCRIPTION: &str =
        "Run arbitrary git command with specified arguments";

    fn parameters() -> JsonSchema {
        JsonSchema {
            kind: Some("array".into()),
            items: Some(Box::new(JsonSchema {
                kind: Some("string".into()),
                ..Default::default()
            })),
            description: Some("arguments".into()),
            ..Default::default()
        }
    }

    fn run(arguments: serde_json::Value) -> Result<String, TheyaError> {
        if let Some(args) = arguments.as_array() {
            let args: Vec<&str> =
                args.iter().filter_map(|v| v.as_str()).collect();
            Ok(run_command_checked("git", args.as_slice())?)
        } else {
            Err(TheyaError::new(
                ErrorKind::AiInvalidReply,
                "git: need array of string as arguments".to_string(),
            ))
        }
    }
}

pub(crate) struct ToolGitCheckout;
impl ToolHandler<String> for ToolGitCheckout {
    const NAME: &str = "git_checkout";
    const DESCRIPTION: &str = "Restore working files using git checkout";

    fn parameters() -> JsonSchema {
        JsonSchema {
            kind: Some("array".into()),
            items: Some(Box::new(JsonSchema {
                kind: Some("string".into()),
                ..Default::default()
            })),
            description: Some("arguments".into()),
            ..Default::default()
        }
    }

    fn run(arguments: serde_json::Value) -> Result<String, TheyaError> {
        if let Some(args) = arguments.as_array() {
            let mut args: Vec<&str> =
                args.iter().filter_map(|v| v.as_str()).collect();
            args.insert(0, "checkout");
            Ok(run_command_checked("git", args.as_slice())?)
        } else {
            Err(TheyaError::new(
                ErrorKind::AiInvalidReply,
                "git: need array of string as arguments".to_string(),
            ))
        }
    }
}

pub(crate) struct ToolGitDiff;
impl ToolHandler<String> for ToolGitDiff {
    const NAME: &str = "git_diff";
    const DESCRIPTION: &str =
        "Get uncommitted changes since last git commit, no parameter required";

    fn parameters() -> JsonSchema {
        JsonSchema::default()
    }

    fn run(_arguments: serde_json::Value) -> Result<String, TheyaError> {
        run_command_checked("git", &["diff"])
    }
}

pub(crate) struct ToolGitShowCommit;
impl ToolHandler<String> for ToolGitShowCommit {
    const NAME: &str = "git_show_commit";
    const DESCRIPTION: &str = "Get content of specified commit hash or most \
                               recent commit when no argument provided";

    fn parameters() -> JsonSchema {
        JsonSchema {
            kind: Some("string".into()),
            description: Some(
                "commit hash to query, if omit, show recent commit".into(),
            ),
            ..Default::default()
        }
    }

    fn run(arguments: serde_json::Value) -> Result<String, TheyaError> {
        if let serde_json::Value::String(hash) = &arguments {
            Ok(run_command_checked("git", &["show", hash])?)
        } else {
            Ok(run_command_checked("git", &["show"])?)
        }
    }
}

pub(crate) struct ToolGitCreateCommit;
impl ToolHandler<()> for ToolGitCreateCommit {
    const NAME: &str = "git_commit_changes";
    const DESCRIPTION: &str = "Commit all the changes to git repository";

    fn parameters() -> JsonSchema {
        let mut json_schema_props: HashMap<String, Box<JsonSchema>> =
            HashMap::new();
        json_schema_props.insert(
            "commit_message".into(),
            Box::new(JsonSchema {
                kind: Some("string".into()),
                description: Some("commit message".into()),
                ..Default::default()
            }),
        );
        JsonSchema {
            kind: Some("object".into()),
            properties: Some(json_schema_props),
            required: Some(Vec::new()),
            ..Default::default()
        }
    }

    fn run(arguments: serde_json::Value) -> Result<(), TheyaError> {
        if let Some(msg) = arguments
            .as_object()
            .and_then(|o| o.get("commit_message"))
            .and_then(|v| v.as_str())
        {
            run_command_checked("git", &["commit", "-a", "-m", msg])?;
            Ok(())
        } else {
            Err(TheyaError::new(
                ErrorKind::Bug,
                "ToolGitCreateCommit(): need `commit_message` argument"
                    .to_string(),
            ))
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct Git {}

impl Git {
    pub(crate) fn get_cur_patch_titile() -> Result<String, TheyaError> {
        Self::run(&["log", "-1", "--pretty=%s"])
    }

    pub(crate) fn file_list(
        path: Option<&str>,
    ) -> Result<Vec<String>, TheyaError> {
        let mut path = path.unwrap_or("./");
        if path.is_empty() {
            path = "./";
        }
        let args = vec!["ls-files", path];
        Self::run(&args).map(|output| {
            output
                .split("\n")
                .filter_map(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.to_string())
                    }
                })
                .collect()
        })
    }

    pub(crate) fn get_root_dir_path() -> Result<String, TheyaError> {
        Self::run(&["rev-parse", "--show-toplevel"])
            .map(|p| p.trim().to_string())
    }

    pub(crate) fn get_origin_remote_url() -> Result<String, TheyaError> {
        Self::run(&["remote", "get-url", "origin"])
            .map(|p| p.trim().to_string())
    }

    fn run(args: &[&str]) -> Result<String, TheyaError> {
        run_command_checked("git", args)
    }
}
