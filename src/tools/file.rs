// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::{git::Git, traits::ToolHandler};
use crate::{
    cmd::run_command,
    error::{ErrorKind, TheyaError},
    json_schema::JsonSchema,
    security::is_within_current_dir,
};

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
    const DESCRIPTION: &str = "Read content of the specified file, optionally \
                               read specified line range";

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
            "line_start".into(),
            Box::new(JsonSchema {
                kind: Some("integer".into()),
                description: Some(
                    "optional argument, only read content after specified \
                     line, including the specified line, first line is line \
                     1. Undefined means start from first line."
                        .into(),
                ),
                ..Default::default()
            }),
        );
        json_schema_props.insert(
            "line_end".into(),
            Box::new(JsonSchema {
                kind: Some("integer".into()),
                description: Some(
                    "optional argument, only read content till specified \
                     line, including the specified line, first line is line \
                     1. Undefined means till the end of file."
                        .into(),
                ),
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
            Ok(read_file_range(
                file_path,
                arguments
                    .as_object()
                    .and_then(|o| o.get("line_start"))
                    .and_then(|v| v.as_u64())
                    .map(|i| i as usize),
                arguments
                    .as_object()
                    .and_then(|o| o.get("line_end"))
                    .and_then(|v| v.as_u64())
                    .map(|i| i as usize),
            )?)
        } else {
            Err(TheyaError::new(
                ErrorKind::Bug,
                "ToolReadFile(): Invalid argument: expecting object with \
                 `file_path`"
                    .to_string(),
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
            let (status, stdout, stderr) =
                run_command("rg", &["-e", pattern, path])?;

            if status.success() {
                Ok(stdout)
            } else if status.code() == Some(1) {
                Ok("error: no match".to_string())
            } else {
                Err(TheyaError::new(
                    ErrorKind::AiInvalidReply,
                    format!(
                        "grep failed with error {}, stdout: '{stdout}', \
                         stderr: '{stderr}'",
                        status.code().unwrap_or_default()
                    ),
                ))
            }
        } else {
            Err(TheyaError::new(
                ErrorKind::Bug,
                "ToolGrep(): Invalid argument: expecting object with `path` \
                 and `pattern`"
                    .to_string(),
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
            if !is_within_current_dir(file_path)? {
                return Err(TheyaError::new(
                    ErrorKind::AiInvalidReply,
                    format!(
                        "Cannot write file {file_path} outside of current \
                         working directory"
                    ),
                ));
            }
            log::info!("Modifying {file_path}");
            if let Err(e) = std::fs::write(file_path, file_content) {
                Ok(format!("FAIL: {e}"))
            } else {
                Ok("PASS".into())
            }
        } else {
            Err(TheyaError::new(
                ErrorKind::Bug,
                "ToolWriteFile(): argument should be dictionary with \
                 `file_path` and `file_content`"
                    .to_string(),
            ))
        }
    }
}

fn read_file_range(
    file_path: &str,
    line_start: Option<usize>,
    line_end: Option<usize>,
) -> Result<String, TheyaError> {
    let full_content = std::fs::read_to_string(file_path)?;
    let total_line_count = full_content.lines().count();
    let line_start = line_start
        .map(|i| i.clamp(1, total_line_count))
        .unwrap_or(1)
        - 1;

    let line_end = line_end
        .map(|i| i.clamp(1, total_line_count))
        .unwrap_or(total_line_count);

    if line_start == 0 && line_end == total_line_count {
        log::info!("reading file {file_path}");
    } else {
        log::info!(
            "reading file {file_path}, range {}:{line_end}",
            line_start + 1
        );
    }

    if line_end >= line_start {
        let mut content = full_content
            .lines()
            .skip(line_start)
            .take(line_end - line_start)
            .collect::<Vec<&str>>()
            .join("\n");
        content.push('\n');
        Ok(content)
    } else {
        Err(TheyaError::new(
            ErrorKind::AiInvalidReply,
            "ToolReadFile(): Invalid argument line_start and line_end: \
             line_end should equal or bigger than line_start"
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn create_test_file(file_path: &str) {
        let mut content = String::new();
        for i in 1..=5 {
            content.push_str(&format!("{i}"));
            content.push('\n');
        }
        std::fs::write(file_path, &content).unwrap();
    }

    fn remove_test_file(file_path: &str) {
        std::fs::remove_file(file_path).ok();
    }

    #[test]
    fn read_range() {
        let test_file_path: &str = "/tmp/theya_test_read.file";

        create_test_file(test_file_path);
        assert_eq!(
            read_file_range(test_file_path, None, None).unwrap(),
            "1\n2\n3\n4\n5\n".to_string()
        );

        assert_eq!(
            read_file_range(test_file_path, Some(1), None).unwrap(),
            "1\n2\n3\n4\n5\n".to_string()
        );

        assert_eq!(
            read_file_range(test_file_path, Some(0), None).unwrap(),
            "1\n2\n3\n4\n5\n".to_string()
        );

        assert_eq!(
            read_file_range(test_file_path, Some(2), None).unwrap(),
            "2\n3\n4\n5\n".to_string()
        );

        assert_eq!(
            read_file_range(test_file_path, Some(2), Some(3)).unwrap(),
            "2\n3\n".to_string()
        );

        assert_eq!(
            read_file_range(test_file_path, Some(2), Some(2)).unwrap(),
            "2\n".to_string()
        );

        assert_eq!(
            read_file_range(test_file_path, Some(2), Some(9)).unwrap(),
            "2\n3\n4\n5\n".to_string()
        );

        remove_test_file(test_file_path);
    }
}
