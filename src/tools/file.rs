// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{git::Git, traits::ToolHandler};
use crate::{
    cmd::run_command_checked,
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

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub(crate) struct FileContent {
    pub(crate) file_path: String,
    pub(crate) file_content: String,
}

pub(crate) struct ToolReadFile;

impl ToolHandler<FileContent> for ToolReadFile {
    const NAME: &str = "read_file";
    const DESCRIPTION: &str = "Read content of the specified file, return \
                               file_path, file_range, and file_content";

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

    fn run(arguments: serde_json::Value) -> Result<FileContent, TheyaError> {
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
            Ok(FileContent {
                file_path: file_path.to_string(),
                file_content: read_file_range(
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
                )?,
            })
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
            Ok(run_command_checked("rg", &["-e", pattern, path])?)
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
    const DESCRIPTION: &str = "Write content to specified file, optionally , \
                               return PASS or FAIL along with error";

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
        json_schema_props.insert(
            "line_start".into(),
            Box::new(JsonSchema {
                kind: Some("integer".into()),
                description: Some(
                    "optional argument, replace lines between(inclusive) \
                     line_start and line_end with specified file_content. The \
                     first line is line 1. To append file_content before \
                     original content, set line_start to 0. To add \
                     file_content after original content, set line_start to \
                     -1. Undefined means 1(first line)."
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
                    "optional argument, replace lines between(inclusive) \
                     line_start and line_end with specified file_content. The \
                     first line is line 1. Undefined means replace till last \
                     line"
                        .into(),
                ),
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
            let modified = read_and_replace_file_range(
                file_path,
                file_content,
                arguments
                    .as_object()
                    .and_then(|o| o.get("line_start"))
                    .and_then(|v| v.as_i64()),
                arguments
                    .as_object()
                    .and_then(|o| o.get("line_end"))
                    .and_then(|v| v.as_i64()),
            )?;
            if let Err(e) = std::fs::write(file_path, &modified) {
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

fn read_and_replace_file_range(
    file_path: &str,
    file_content: &str,
    line_start: Option<i64>,
    line_end: Option<i64>,
) -> Result<String, TheyaError> {
    let original = std::fs::read_to_string(file_path)?;
    let total_line_count = original.lines().count() as i64;

    let line_start = line_start.unwrap_or(1);
    let line_end = line_end.unwrap_or(total_line_count);

    if line_end < line_start {
        return Err(TheyaError::new(
            ErrorKind::AiInvalidReply,
            "ToolWriteFile(): Invalid argument: line_end should bigger or \
             equal than line_start"
                .to_string(),
        ));
    }

    match line_start {
        i if i < -1 => Err(TheyaError::new(
            ErrorKind::AiInvalidReply,
            "ToolWriteFile(): Invalid argument: line_start should bigger than \
             -1"
            .to_string(),
        )),
        i if (i == -1) || (i > total_line_count) => {
            log::info!("Append content after original file");
            let mut modified = original;
            if !modified.ends_with("\n") {
                modified.push('\n');
            }
            modified.push_str(file_content);
            if !modified.ends_with("\n") {
                modified.push('\n');
            }
            Ok(modified)
        }
        0 => {
            log::info!("Appending content at the beginning");
            let mut modified = file_content.to_string();
            if !modified.ends_with("\n") {
                modified.push('\n');
            }
            modified.push_str(original.as_str());
            if !modified.ends_with("\n") {
                modified.push('\n');
            }
            Ok(modified)
        }
        i if (1..=total_line_count).contains(&i) => {
            if i == 1 && line_end == total_line_count {
                log::info!("Override full content of origin file");
                let mut ret = file_content.to_string();
                if !ret.ends_with("\n") {
                    ret.push('\n');
                }
                Ok(ret)
            } else {
                log::info!("Replacing range {}:{} of origin file", i, line_end);
                let mut modified: String = original
                    .lines()
                    .take(i as usize - 1)
                    .collect::<Vec<&str>>()
                    .join("\n");
                if !modified.is_empty() {
                    modified.push('\n');
                }
                modified.push_str(file_content);
                if !modified.ends_with("\n") {
                    modified.push('\n');
                }
                for line in original.lines().skip(line_end as usize) {
                    modified.push_str(line);
                    modified.push('\n');
                }
                Ok(modified)
            }
        }
        _ => unreachable!(),
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

    #[test]
    fn write_range() {
        let test_file_path: &str = "/tmp/theya_test_write.file";
        create_test_file(test_file_path);
        assert_eq!(
            read_and_replace_file_range(test_file_path, "abc", None, None)
                .unwrap(),
            "abc\n".to_string()
        );
        assert_eq!(
            read_and_replace_file_range(test_file_path, "abc", Some(0), None)
                .unwrap(),
            "abc\n1\n2\n3\n4\n5\n".to_string()
        );
        assert_eq!(
            read_and_replace_file_range(test_file_path, "abc", Some(-1), None)
                .unwrap(),
            "1\n2\n3\n4\n5\nabc\n".to_string()
        );
        assert_eq!(
            read_and_replace_file_range(
                test_file_path,
                "a\nb\nc",
                Some(1),
                Some(2),
            )
            .unwrap(),
            "a\nb\nc\n3\n4\n5\n".to_string()
        );
        assert_eq!(
            read_and_replace_file_range(
                test_file_path,
                "a\nb\nc",
                Some(2),
                Some(5),
            )
            .unwrap(),
            "1\na\nb\nc\n".to_string()
        );
        assert_eq!(
            read_and_replace_file_range(
                test_file_path,
                "a\nb\nc",
                Some(2),
                Some(1000),
            )
            .unwrap(),
            "1\na\nb\nc\n".to_string()
        );
        assert_eq!(
            read_and_replace_file_range(
                test_file_path,
                "a\nb\nc",
                Some(2),
                None,
            )
            .unwrap(),
            "1\na\nb\nc\n".to_string()
        );

        remove_test_file(test_file_path);
    }
}
