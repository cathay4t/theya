// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, io::Write};

use serde::{Deserialize, Serialize};

use super::{
    cmd::{run_command, spawn_editor},
    config::{TheyaCodeConfig, TheyaProjectConfig},
    error::{ErrorKind, TheyaError},
    git::GitStore,
    json_schema::JsonSchema,
    ollama::OllamaClient,
};

const DEFAULT_EDITOR: &str = "vim";
const COMMENT_PREFIX: &str = "<!-- Theya: ";
const COMMENT_POSTFIX: &str = " -->";
const MAX_FILE_COUNT: usize = 10;
const MAX_GIT_LOG_COUNT: usize = 10;
const MAX_REF_COMMIT_COUNT: usize = 1;
const GIT_COMMIT_HASH_COMPACT_LEN: usize = 7;
const MAX_RETRY: usize = 10;
const RETRY_INTERVAL_MS: u64 = 1000;
const PROMPT_HEADER: &str = "You are working in a git repository assisting \
                             user on the coding task specified below.\nThis \
                             task has been derived into small steps.";
const DEFAULT_RUST_BUILD_CMD: &str = "cargo build";
const DEFAULT_RUST_LINT_CMD: &str =
    "cargo fmt --all && cargo clippy --all-targets --fix";

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum CodeType {
    Unknown,
    Rust,
}

macro_rules! retry {
    ($f:expr) => {{
        let mut retry_count = 0;
        loop {
            let result = $f;
            if result.is_ok() {
                break result;
            } else if retry_count > MAX_RETRY {
                break result;
            } else if let Err(e) = &result {
                if !e.retryable() {
                    break result;
                } else {
                    retry_count += 1;
                    log::info!(
                        "Retrying {retry_count}/{MAX_RETRY} on failure {e}"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(
                        RETRY_INTERVAL_MS,
                    ))
                    .await;
                }
            }
        }
    }};
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct ChangedFile {
    #[serde(rename = "file_path")]
    path: String,
    #[serde(rename = "file_content")]
    content: String,
}

pub(crate) struct CommandCode;

async fn get_file_list_to_read(
    client: &OllamaClient,
    coding_task: &str,
    gs: &GitStore,
) -> Result<Vec<String>, TheyaError> {
    let file_list = gs.file_list()?;
    let mut file_list_str = String::new();
    for file in &file_list {
        file_list_str.push_str(&format!(" * {}\n", file));
    }

    let json_schema = JsonSchema {
        kind: "array".into(),
        items: Some(Box::new(JsonSchema {
            kind: "string".into(),
            ..Default::default()
        })),
        ..Default::default()
    };

    #[rustfmt::skip]
    let mut prompt = format!(
        "{PROMPT_HEADER}\n\
         Currently we need to generate a file list which content is \
         required as context for fulfill coding task describe below.\n\
         Order the file list from most related to lest related.\n\
         Do not include file which is not related to coding task.\n\
         # Coding Task\n\
         ```\n\
         {coding_task}\n\
         ```\n\
         # Git repo file lists\n\
         {file_list_str}\n"
    );

    if is_task_require_current_git_commit(client, coding_task).await? {
        let patch_content = gs.get_cur_patch_content()?;
        prompt.push_str(&format!(
            "# Current git commit\n```\n{patch_content}\n```\n"
        ));
    }

    log::debug!("Requesting\n{prompt}");
    log::info!("Request AI to suggest on which files to read");
    let reply = client
        .generate_ai_structured_response(prompt, json_schema)
        .await?;

    let elapsed = std::time::Duration::from_nanos(reply.total_duration_ns);
    log::info!("Elapsed: {:.02} seconds", elapsed.as_secs_f64());

    log::trace!("Reply is:\n{}", reply.response);

    let mut ret: Vec<String> = serde_json::from_str(reply.response.as_str())
        .map_err(|_| {
            TheyaError::new(
                ErrorKind::AiInvalidReply,
                format!("AI reply is not valid JSON: {}", reply.response),
            )
        })?;

    // AI might have delusion by inventing file path, we need to check
    // returned file_path is valid.
    ret.retain(|file_path| file_list.contains(file_path));
    ret.dedup();

    if ret.is_empty() {
        return Err(TheyaError::new(
            ErrorKind::AiInvalidReply,
            "AI reply with empty file list".into(),
        ));
    }

    ret.truncate(MAX_FILE_COUNT);

    log::info!("AI suggest to read these files:",);
    for file_path in ret.as_slice() {
        log::info!("{file_path}");
    }

    Ok(ret)
}

async fn get_related_git_commits(
    client: &OllamaClient,
    coding_task: &str,
    gs: &GitStore,
    file_list: &[String],
) -> Result<Vec<String>, TheyaError> {
    let mut file_list_str = String::new();
    for file in file_list {
        file_list_str.push_str(&format!(" * {}\n", file));
    }
    let file_list_ref: Vec<&str> =
        file_list.iter().map(|s| s.as_str()).collect();
    let short_log =
        gs.git_short_log(file_list_ref.as_slice(), MAX_GIT_LOG_COUNT)?;
    let commit_hashes: Vec<&str> = short_log
        .split('\n')
        .filter_map(|l| l.split(' ').next())
        .filter(|l| l.len() == GIT_COMMIT_HASH_COMPACT_LEN)
        .collect();

    let json_schema = JsonSchema {
        kind: "array".into(),
        items: Some(Box::new(JsonSchema {
            kind: "string".into(),
            ..Default::default()
        })),
        ..Default::default()
    };

    #[rustfmt::skip]
    let prompt = format!(
        "{PROMPT_HEADER}\n\
         Currently, you need to generate a list of related git commit hash \
         which could be used to as reference of coding task describe below.\n\
         Order the commit hash from most related to lest related.\n\
         Do not include commit hash which is not related to coding task.\n\
         Git commit hash example `54b47820`.\n\
         # Coding Task\n\
         ```\n\
         {coding_task}\n\
         ```\n\
         # Related files\n\
         {file_list_str}\n\
         # Git commits of related files\n\
         ```\n\
         {short_log}\n\
         ```\n"
    );
    log::debug!("Requesting\n{prompt}");
    log::info!("Requesting related git commit");

    let reply = client
        .generate_ai_structured_response(prompt, json_schema)
        .await?;

    let elapsed = std::time::Duration::from_nanos(reply.total_duration_ns);
    log::info!("Elapsed: {:.02} seconds", elapsed.as_secs_f64());

    log::trace!("Reply is:\n{}", reply.response);

    // AI might reply with extra content other than commit hash ,
    // we have to use known commit hash list and filter the response and still
    // preserve the commit hash order of AI response
    let mut ret: Vec<String> = serde_json::from_str(reply.response.as_str())
        .map_err(|_| {
            TheyaError::new(
                ErrorKind::AiInvalidReply,
                format!("AI reply is not valid JSON: {}", reply.response),
            )
        })?;
    // AI might have delusion by inventing file path, we need to check
    // returned file_path is valid.
    ret.retain(|hash| commit_hashes.contains(&hash.as_str()));

    ret.dedup();
    if ret.is_empty() {
        return Err(TheyaError::new(
            ErrorKind::AiInvalidReply,
            "Got empty AI reply for git commit hashes".into(),
        ));
    }

    ret.truncate(MAX_REF_COMMIT_COUNT);

    log::info!("AI suggest to use these commits as reference:");
    for hash in ret.as_slice().iter() {
        if let Some(line) = short_log.lines().find(|l| l.contains(hash)) {
            log::info!("{line}");
        }
    }

    Ok(ret)
}

async fn get_file_list_from_commit_reference(
    client: &OllamaClient,
    coding_task: &str,
    gs: &GitStore,
    commit_hashes: &[String],
) -> Result<Vec<String>, TheyaError> {
    log::info!(
        "Request AI to suggest file list base on historical git commits"
    );

    let file_list = gs.file_list()?;
    let mut file_list_str = String::new();
    for file in &file_list {
        file_list_str.push_str(&format!(" * {}\n", file));
    }

    let mut commit_contents = String::new();
    for commit_hash in commit_hashes {
        commit_contents
            .push_str(&format!("{}\n", gs.get_commit(commit_hash.as_str())?));
    }

    #[rustfmt::skip]
    let prompt = format!(
        "{PROMPT_HEADER}\n\
         Currently we need to generate list of files which must be \
         modified or included as reference to fulfill this coding task.\n\
         The content of related git commits are provided as reference.\n\
         # Coding Task\n\
         ```\n\
         {coding_task}\n\
         ```\n\n\
         # File paths\n\
         {file_list_str}\n\
         # Related git commits\n\
         ```\n\
         {commit_contents}
         ```\n"
    );
    log::debug!("Requesting\n{prompt}");

    let json_schema = JsonSchema {
        kind: "array".into(),
        items: Some(Box::new(JsonSchema {
            kind: "string".into(),
            ..Default::default()
        })),
        ..Default::default()
    };
    let reply = client
        .generate_ai_structured_response(prompt, json_schema)
        .await?;
    let elapsed = std::time::Duration::from_nanos(reply.total_duration_ns);
    log::info!("Elapsed: {:.02} seconds", elapsed.as_secs_f64());

    let ret: Vec<String> = serde_json::from_str(reply.response.as_str())
        .map_err(|_| {
            TheyaError::new(
                ErrorKind::AiInvalidReply,
                format!("AI reply is not valid JSON: {}", reply.response),
            )
        })?;
    log::info!("AI suggest to use these files:",);
    for file_path in ret.as_slice() {
        log::info!("{file_path}");
    }

    Ok(ret)
}

async fn code_it(
    client: &OllamaClient,
    coding_task: &str,
    gs: &GitStore,
    file_list: Vec<String>,
) -> Result<Vec<ChangedFile>, TheyaError> {
    log::info!("Requesting AI to code on these files");
    for file in &file_list {
        log::info!("{file}");
    }

    let mut file_contents = String::new();
    for file in &file_list {
        let content = gs.get_file_content(&std::path::Path::new(file))?;
        file_contents.push_str(&format!("## {file}\n```\n{content}\n```\n"));
    }

    let mut json_schema_props: HashMap<String, Box<JsonSchema>> =
        HashMap::new();
    json_schema_props.insert(
        "file_path".into(),
        Box::new(JsonSchema {
            kind: "string".into(),
            ..Default::default()
        }),
    );
    json_schema_props.insert(
        "file_content".into(),
        Box::new(JsonSchema {
            kind: "string".into(),
            ..Default::default()
        }),
    );

    let json_schema = JsonSchema {
        kind: "array".into(),
        items: Some(Box::new(JsonSchema {
            kind: "object".into(),
            properties: Some(json_schema_props),
            required: Some(vec![
                "file_path".to_string(),
                "file_content".to_string(),
            ]),
            ..Default::default()
        })),
        ..Default::default()
    };

    #[rustfmt::skip]
    let prompt = format!(
        "{PROMPT_HEADER}\n\
         Currently you need to modify provides files for the coding task.\n\
         Reply with file_path and file_content.\n\
         Do not include any file which is not changed\n\
         # Coding Task\n\
         ```\n\
         {coding_task}\n\
         ```\n\n\
         # File contents\n\
         {file_contents}\n"
    );
    log::debug!("Requesting\n{prompt}");

    let reply = client
        .generate_ai_structured_response(prompt, json_schema)
        .await?;

    let elapsed = std::time::Duration::from_nanos(reply.total_duration_ns);
    log::info!("Elapsed: {:.02} seconds", elapsed.as_secs_f64());

    log::trace!("Reply is:\n{}", reply.response);
    let ret: Vec<ChangedFile> = serde_json::from_str(reply.response.as_str())
        .map_err(|_| {
        TheyaError::new(
            ErrorKind::AiInvalidReply,
            format!("AI reply is not valid JSON: {}", reply.response),
        )
    })?;

    Ok(ret)
}

async fn is_task_require_current_git_commit(
    client: &OllamaClient,
    coding_task: &str,
) -> Result<bool, TheyaError> {
    log::info!(
        "Request AI to suggest whether current git commit content is required \
         for coding task"
    );

    #[rustfmt::skip]
    let prompt = format!(
        "You are working in side of git repo.\n\
         Replay boolean on whether current git commit content \
         should be provided for specified coding task\n\
         # Coding Task\n\
         ```\n\
         {coding_task}\n\
         ```\n"
    );
    log::debug!("Requesting\n{prompt}");

    let json_schema = JsonSchema {
        kind: "boolean".into(),
        ..Default::default()
    };
    let reply = client
        .generate_ai_structured_response(prompt, json_schema)
        .await?;
    let elapsed = std::time::Duration::from_nanos(reply.total_duration_ns);
    log::info!("Elapsed: {:.02} seconds", elapsed.as_secs_f64());
    log::trace!("Reply is:\n{}", reply.response);

    let ret: bool =
        serde_json::from_str(reply.response.as_str()).map_err(|_| {
            TheyaError::new(
                ErrorKind::AiInvalidReply,
                format!("AI reply is not valid JSON: {}", reply.response),
            )
        })?;

    log::info!("Need current git commit content: {ret}");

    Ok(ret)
}

/// Compile and fix the code if compile failed.
async fn run_cmd_and_amend(
    client: &OllamaClient,
    coding_task: &str,
    gs: &GitStore,
    cmd: &str,
    purpose: &str,
) -> Result<(), TheyaError> {
    todo!()
}

/// Lint format and fix the code if failed.
async fn check_lint(
    client: &OllamaClient,
    coding_task: &str,
    gs: &GitStore,
    cmd: &str,
) -> Result<(), TheyaError> {
    todo!()
}

/// Run unit test and fix the code if failed.
async fn check_unit_test(
    client: &OllamaClient,
    coding_task: &str,
    gs: &GitStore,
    cmd: &str,
) -> Result<(), TheyaError> {
    todo!()
}

impl CommandCode {
    pub(crate) const CMD: &str = "code";

    pub(crate) fn new_cmd() -> clap::Command {
        clap::Command::new(Self::CMD).about("Request AI to code in a git repo")
    }

    pub(crate) async fn handle(
        config: &TheyaCodeConfig,
        projects_config: &HashMap<String, TheyaProjectConfig>,
    ) -> Result<(), TheyaError> {
        let gs = GitStore::new(std::env::current_dir()?);

        std::env::set_current_dir(gs.get_root_dir_path()?.as_str())?;

        let project_url = gs.get_origin_remote_url()?;
        let mut project_config = projects_config
            .values()
            .find(|c| c.git.as_str() == project_url.as_str())
            .cloned()
            .unwrap_or_default();

        println!("HAHA94 {:?}", project_config);

        let client = OllamaClient::new(
            config.uri.as_str(),
            config.model.as_str(),
            config.guideline.as_str(),
            config.context_count,
        )
        .await?;

        log::trace!("System prompt:\n{}", config.guideline.as_str());

        let editor = std::env::var("EDITOR")
            .unwrap_or_else(|_| DEFAULT_EDITOR.to_string());

        let tmp_file_path = std::path::PathBuf::from(format!(
            "{}.md",
            run_command("mktemp -u", &std::env::temp_dir())?.trim()
        ));

        let mut fd = std::fs::File::create(&tmp_file_path)?;
        #[rustfmt::skip]
        fd.write_all(
            format!(
                "\n\n\
                {COMMENT_PREFIX}Ollama connected to: {}{COMMENT_POSTFIX}\n\
                {COMMENT_PREFIX}Ollama version: {}{COMMENT_POSTFIX}\n\
                {COMMENT_PREFIX}Model: {}{COMMENT_POSTFIX}\n\
                {COMMENT_PREFIX}Please type your request above, \
                save and quit{COMMENT_POSTFIX}\n",
                client.uri,
                client.version().await?,
                client.model,
            ).as_bytes(),
        )?;

        spawn_editor(&editor, &tmp_file_path)?;

        let coding_task = std::fs::read_to_string(&tmp_file_path)?
            .lines()
            .filter(|line| !line.starts_with(COMMENT_PREFIX))
            .collect::<Vec<&str>>()
            .join("\n")
            .trim()
            .to_string();

        std::fs::remove_file(&tmp_file_path)?;

        if coding_task.is_empty() {
            return Err(TheyaError::new(
                ErrorKind::EmptyInput,
                "Got empty input, quitting".into(),
            ));
        }

        log::info!("Coding task: {coding_task}");

        let file_list = retry!(
            async {
                get_file_list_to_read(&client, coding_task.as_str(), &gs).await
            }
            .await
        )?;

        let commit_hashes = retry!(
            async {
                get_related_git_commits(
                    &client,
                    coding_task.as_str(),
                    &gs,
                    &file_list,
                )
                .await
            }
            .await
        )?;

        let file_list = retry!(
            async {
                get_file_list_from_commit_reference(
                    &client,
                    coding_task.as_str(),
                    &gs,
                    &commit_hashes,
                )
                .await
            }
            .await
        )?;

        let changed_files =
            code_it(&client, coding_task.as_str(), &gs, file_list).await?;
        let mut code_type = CodeType::Unknown;
        for changed_file in changed_files {
            if std::path::Path::new(changed_file.path.as_str()).exists() {
                if changed_file.path.ends_with(".rs") {
                    code_type = CodeType::Rust
                }
                log::info!("Updating {}", changed_file.path.as_str());
                if !changed_file.content.is_empty() {
                    std::fs::write(
                        changed_file.path.as_str(),
                        changed_file.content.as_str(),
                    )?;
                }
            }
        }

        // TODO: Ask AI to generate a good commit message
        gs.commit(&coding_task)?;

        if code_type == CodeType::Rust {
            if project_config.compile.is_none() {
                project_config.compile = Some(DEFAULT_RUST_BUILD_CMD.into());
            }
            if project_config.lint.is_none() {
                project_config.lint = Some(DEFAULT_RUST_LINT_CMD.into());
            }
            if project_config.unit_test.is_none() {
                project_config.unit_test = Some("cargo test".into());
            }
        }

        if let Some(cmd) = project_config.compile.as_ref() {
            run_cmd_and_amend(
                &client,
                coding_task.as_str(),
                &gs,
                cmd,
                "compile",
            )
            .await?;
        }

        if let Some(cmd) = project_config.lint.as_ref() {
            run_cmd_and_amend(&client, coding_task.as_str(), &gs, cmd, "lint")
                .await?;
        }

        if let Some(cmd) = project_config.unit_test.as_ref() {
            run_cmd_and_amend(
                &client,
                coding_task.as_str(),
                &gs,
                cmd,
                "unit test",
            )
            .await?;
        }

        Ok(())
    }
}
