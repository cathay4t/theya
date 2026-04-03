// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::ToolHandlerCmd;
use crate::{cmd::run_command, error::TheyaError, json_schema::JsonSchema};

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
