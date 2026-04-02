// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::{
    cmd::{run_command, run_command_checked},
    config::TheyaProjectConfig,
    error::{ErrorKind, TheyaError},
    git::Git,
    json_schema::JsonSchema,
    ollama::{
        OllamaChatMessage, OllamaChatMessageRole, OllamaFunctionPrototype,
        OllamaTool, OllamaToolPrototype,
    },
    security::is_within_current_dir,
};

pub(crate) trait ToolHandlerCmd {
    const NAME: &str;
    const DESCRIPTION: &str;

    fn parameters() -> JsonSchema {
        JsonSchema::default()
    }

    fn prototype() -> OllamaToolPrototype {
        OllamaFunctionPrototype {
            name: Self::NAME.to_string(),
            parameters: Self::parameters(),
            description: Some(Self::DESCRIPTION.to_string()),
        }
        .into()
    }

    fn run(
        cmd: &str,
        _arguments: serde_json::Value,
    ) -> Result<String, TheyaError> {
        let (status, stdout, stderr) = run_command("bash", &["-c", cmd])?;

        if status.success() {
            Ok("PASS".into())
        } else {
            Ok(format!("FAIL:{stdout}\n{stderr}"))
        }
    }

    fn handle(
        cmd: Option<&str>,
        arguments: serde_json::Value,
    ) -> Result<String, TheyaError> {
        if let Some(cmd) = cmd {
            match Self::run(cmd, arguments) {
                Ok(t) => Ok(serde_json::to_string(&t)?),
                Err(e) => {
                    log::warn!("Tool invoking failed with {e}");
                    Ok(serde_json::to_string(&format!("FAIL: {e}"))?)
                }
            }
        } else {
            Ok(serde_json::to_string(&format!(
                "tool for {} undefined by user, silent pass",
                Self::NAME,
            ))?)
        }
    }
}

pub(crate) trait ToolHandler<T>
where
    T: serde::Serialize,
{
    const NAME: &str;
    const DESCRIPTION: &str;
    fn parameters() -> JsonSchema;

    fn prototype() -> OllamaToolPrototype {
        OllamaFunctionPrototype {
            name: Self::NAME.to_string(),
            parameters: Self::parameters(),
            description: Some(Self::DESCRIPTION.to_string()),
        }
        .into()
    }

    fn run(_arguments: serde_json::Value) -> Result<T, TheyaError>;

    fn handle(arguments: serde_json::Value) -> Result<String, TheyaError> {
        match Self::run(arguments) {
            Ok(t) => Ok(serde_json::to_string(&t)?),
            Err(e) => {
                log::warn!("Tool invoking failed with {e}");
                Ok(serde_json::to_string(&format!("FAIL: {e}"))?)
            }
        }
    }
}

pub(crate) struct ToolFileList;

impl ToolHandler<Vec<String>> for ToolFileList {
    const NAME: &str = "file_list";
    const DESCRIPTION: &str =
        "Return a list of file names of specified folder in git repo";

    fn parameters() -> JsonSchema {
        let mut json_schema_props: HashMap<String, Box<JsonSchema>> =
            HashMap::new();
        json_schema_props.insert(
            "file_path".into(),
            Box::new(JsonSchema {
                kind: Some("string".into()),
                description: Some("file path".into()),
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

    fn run(arguments: serde_json::Value) -> Result<Vec<String>, TheyaError> {
        if let Some(file_path) = arguments
            .as_object()
            .and_then(|o| o.get("file_path"))
            .and_then(|v| v.as_str())
        {
            Git::file_list(Some(file_path))
        } else {
            Git::file_list(None)
        }
    }
}

pub(crate) struct ToolReadFile;

impl ToolHandler<String> for ToolReadFile {
    const NAME: &str = "read_file";
    const DESCRIPTION: &str = "Read content of the specified file";

    fn parameters() -> JsonSchema {
        let mut json_schema_props: HashMap<String, Box<JsonSchema>> =
            HashMap::new();
        json_schema_props.insert(
            "file_path".into(),
            Box::new(JsonSchema {
                kind: Some("string".into()),
                description: Some("file path".into()),
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

    fn run(arguments: serde_json::Value) -> Result<String, TheyaError> {
        if let Some(file_path) = arguments
            .as_object()
            .and_then(|o| o.get("file_path"))
            .and_then(|v| v.as_str())
        {
            if file_path.is_empty() {
                return Err(TheyaError::new(
                    ErrorKind::AiInvalidReply,
                    "Got empty file_path, please provide file_path as string
                    for file you want to read"
                        .to_string(),
                ));
            }
            if !is_within_current_dir(file_path)? {
                return Err(TheyaError::new(
                    ErrorKind::AiInvalidReply,
                    format!(
                        "Cannot access file {file_path} outside of current \
                         working directory"
                    ),
                ));
            }
            log::info!("Reading file {file_path}");
            Ok(std::fs::read_to_string(file_path)?)
        } else {
            Err(TheyaError::new(
                ErrorKind::Bug,
                format!(
                    "ToolReadFile(): Invalid argument {arguments:?}, \
                     expecting object with `file_path`"
                ),
            ))
        }
    }
}

pub(crate) struct ToolGrep;

impl ToolHandler<String> for ToolGrep {
    const NAME: &str = "grep";
    const DESCRIPTION: &str =
        "grep content of the specified file or folder using ripgrep tool";

    fn parameters() -> JsonSchema {
        let mut json_schema_props: HashMap<String, Box<JsonSchema>> =
            HashMap::new();
        json_schema_props.insert(
            "path".into(),
            Box::new(JsonSchema {
                kind: Some("string".into()),
                description: Some("path".into()),
                ..Default::default()
            }),
        );
        json_schema_props.insert(
            "pattern".into(),
            Box::new(JsonSchema {
                kind: Some("string".into()),
                description: Some("Pattern to grep".into()),
                ..Default::default()
            }),
        );
        JsonSchema {
            kind: Some("object".into()),
            properties: Some(json_schema_props),
            required: Some(vec!["path".to_string(), "pattern".to_string()]),
            ..Default::default()
        }
    }

    fn run(arguments: serde_json::Value) -> Result<String, TheyaError> {
        if let Some(para) = arguments.as_object()
            && let Some(path) = para.get("path").and_then(|v| v.as_str())
            && let Some(pattern) = para.get("pattern").and_then(|v| v.as_str())
        {
            if !is_within_current_dir(path)? {
                return Err(TheyaError::new(
                    ErrorKind::AiInvalidReply,
                    format!(
                        "Cannot grep path {path} outside of current working \
                         directory"
                    ),
                ));
            }
            log::info!("Invoking rg -e {pattern} {path} ");
            Ok(run_command_checked("rg", &["-e", pattern, path])?)
        } else {
            Err(TheyaError::new(
                ErrorKind::Bug,
                format!(
                    "ToolReadFile(): Invalid argument {arguments:?}, \
                     expecting object with `file_path`"
                ),
            ))
        }
    }
}

pub(crate) struct ToolWriteFile;

impl ToolHandler<String> for ToolWriteFile {
    const NAME: &str = "write_files";
    const DESCRIPTION: &str =
        "Write content to specified file, return PASS or FAIL along with error";

    fn parameters() -> JsonSchema {
        let mut json_schema_props: HashMap<String, Box<JsonSchema>> =
            HashMap::new();
        json_schema_props.insert(
            "file_path".into(),
            Box::new(JsonSchema {
                kind: Some("string".into()),
                description: Some("file path".into()),
                ..Default::default()
            }),
        );
        json_schema_props.insert(
            "file_content".into(),
            Box::new(JsonSchema {
                kind: Some("string".into()),
                description: Some("file content".into()),
                ..Default::default()
            }),
        );
        JsonSchema {
            kind: Some("object".into()),
            properties: Some(json_schema_props),
            required: Some(vec![
                "file_path".to_string(),
                "file_content".to_string(),
            ]),
            ..Default::default()
        }
    }

    fn run(arguments: serde_json::Value) -> Result<String, TheyaError> {
        if let Some(obj) = arguments.as_object()
            && let Some(file_path) =
                obj.get("file_path").and_then(|v| v.as_str())
            && let Some(file_content) =
                obj.get("file_content").and_then(|v| v.as_str())
        {
            if !is_within_current_dir(&file_path)? {
                return Err(TheyaError::new(
                    ErrorKind::AiInvalidReply,
                    format!(
                        "Cannot write file {file_path} outside of current \
                         working directory"
                    ),
                ));
            }
            let mut file_content = file_content.to_string();
            log::info!("Updating content of {}", file_path);
            if !file_content.ends_with('\n') {
                file_content.push('\n');
            }
            if let Err(e) = std::fs::write(file_path, &file_content) {
                Ok(format!("FAIL: {e}"))
            } else {
                Ok("PASS".into())
            }
        } else {
            log::warn!("ToolWriteFile: Invalid argument {arguments:?}");
            Err(TheyaError::new(
                ErrorKind::Bug,
                "ToolWriteFile(): argument should be dictionary with \
                 `file_path` and `file_content`"
                    .to_string(),
            ))
        }
    }
}

pub(crate) struct ToolCompile;

impl ToolHandlerCmd for ToolCompile {
    const NAME: &str = "compile";
    const DESCRIPTION: &str =
        "Compile the project return PASS or FAIL with error message";
}

pub(crate) struct ToolUnitTest;

impl ToolHandlerCmd for ToolUnitTest {
    const NAME: &str = "unit_test";
    const DESCRIPTION: &str = "Run unit test after compile passed, return \
                               PASS or FAIL with error message";

    fn parameters() -> JsonSchema {
        let mut json_schema_props: HashMap<String, Box<JsonSchema>> =
            HashMap::new();
        json_schema_props.insert(
            "test_name".into(),
            Box::new(JsonSchema {
                kind: Some("string".into()),
                description: Some(
                    "Run specified test. If undefined, run all tests".into(),
                ),
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

    fn run(
        cmd: &str,
        arguments: serde_json::Value,
    ) -> Result<String, TheyaError> {
        let (status, stdout, stderr) = if let Some(test_name) = arguments
            .as_object()
            .and_then(|o| o.get("test_name"))
            .and_then(|v| v.as_str())
        {
            let cmd = format!("{cmd} {test_name}");
            // cargo test is storing error message in STDOUT
            run_command("bash", &["-c", &cmd])?
        } else {
            run_command("bash", &["-c", cmd])?
        };

        if status.success() {
            Ok("PASS".into())
        } else {
            Ok(format!("FAIL:{stdout}\n{stderr}"))
        }
    }
}

pub(crate) struct ToolFormat;
impl ToolHandlerCmd for ToolFormat {
    const NAME: &str = "code_format";
    const DESCRIPTION: &str = "Format the code after compile pass and unit \
                               test pass, return PASS if succeeded or FAIL \
                               along with error message";
}

pub(crate) struct ToolLintCheck;
impl ToolHandlerCmd for ToolLintCheck {
    const NAME: &str = "lint_check";
    const DESCRIPTION: &str = "Run lint check after compile passed, return \
                               PASS if no error, otherwise return FAIL along \
                               with error message";
}

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
            Err(TheyaError::new(
                ErrorKind::Bug,
                format!("ToolGitLog(): Invalid argument {arguments:?}"),
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

pub(crate) fn gen_tool_prototypes_for_coding() -> Vec<OllamaToolPrototype> {
    vec![
        ToolCompile::prototype(),
        ToolFileList::prototype(),
        ToolGitCreateCommit::prototype(),
        ToolGitDiff::prototype(),
        ToolGitLog::prototype(),
        ToolGitShowCommit::prototype(),
        ToolLintCheck::prototype(),
        ToolReadFile::prototype(),
        ToolGrep::prototype(),
        ToolUnitTest::prototype(),
        ToolWriteFile::prototype(),
    ]
}

pub(crate) fn gen_tool_prototypes_for_review() -> Vec<OllamaToolPrototype> {
    vec![
        ToolGitShowCommit::prototype(),
        ToolFileList::prototype(),
        ToolReadFile::prototype(),
        ToolWriteFile::prototype(),
        ToolCompile::prototype(),
        ToolUnitTest::prototype(),
        ToolGrep::prototype(),
        ToolFormat::prototype(),
    ]
}

/// Return a chat message for replying to AI, second item is boolean on where
/// previous tool reply should be purged to save context windows.
pub(crate) fn handle_tool(
    tool: OllamaTool,
    project_config: &TheyaProjectConfig,
) -> Result<OllamaChatMessage, TheyaError> {
    let arguments = tool.function.arguments;

    let args_display = if tool.function.name.as_str() == ToolWriteFile::NAME {
        "<omitted>".to_string()
    } else {
        arguments.to_string()
    };

    log::info!(
        "Invoking tool `{}` with arguments: {args_display}",
        tool.function.name,
    );

    let content = match tool.function.name.as_str() {
        ToolFileList::NAME => ToolFileList::handle(arguments)?,
        ToolReadFile::NAME => ToolReadFile::handle(arguments)?,
        ToolWriteFile::NAME => ToolWriteFile::handle(arguments)?,
        ToolCompile::NAME => {
            ToolCompile::handle(project_config.compile.as_deref(), arguments)?
        }
        ToolUnitTest::NAME => ToolUnitTest::handle(
            project_config.unit_test.as_deref(),
            arguments,
        )?,
        ToolFormat::NAME => {
            ToolFormat::handle(project_config.format.as_deref(), arguments)?
        }
        ToolLintCheck::NAME => {
            ToolLintCheck::handle(project_config.lint.as_deref(), arguments)?
        }
        ToolGitLog::NAME => ToolGitLog::handle(arguments)?,
        ToolGitShowCommit::NAME => ToolGitShowCommit::handle(arguments)?,
        ToolGitCreateCommit::NAME => ToolGitCreateCommit::handle(arguments)?,
        ToolGitDiff::NAME => ToolGitDiff::handle(arguments)?,
        ToolGrep::NAME => ToolGrep::handle(arguments)?,
        tool_name => {
            log::warn!("Reject AI requested invalid tool: {tool_name}");
            serde_json::to_string(&format!(
                "FAIL: Invalid tool_name {tool_name}"
            ))?
        }
    };
    let msg = OllamaChatMessage {
        role: OllamaChatMessageRole::Tool,
        tool_name: Some(tool.function.name),
        content,
        ..Default::default()
    };

    Ok(msg)
}
