// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::traits::ToolHandler;
use crate::{cmd::run_command, error::TheyaError, json_schema::JsonSchema};

pub(crate) struct ToolCargo;
impl ToolHandler<String> for ToolCargo {
    const NAME: &str = "cargo";
    const DESCRIPTION: &str = "Run any cargo command. Provide the subcommand \
                               (e.g. \"build\", \"test\", \"check\", \
                               \"clippy\") and optional extra arguments. \
                               Returns the combined stdout and stderr output \
                               along with pass/fail status.";

    fn parameters() -> JsonSchema {
        let mut props: HashMap<String, Box<JsonSchema>> = HashMap::new();
        props.insert(
            "subcommand".into(),
            Box::new(JsonSchema {
                kind: Some("string".into()),
                description: Some(
                    "The cargo subcommand to run, e.g. build, test, check, \
                     clippy, fmt, doc, run, bench"
                        .into(),
                ),
                ..Default::default()
            }),
        );
        props.insert(
            "args".into(),
            Box::new(JsonSchema {
                kind: Some("array".into()),
                items: Some(Box::new(JsonSchema {
                    kind: Some("string".into()),
                    description: Some(
                        "Additional argument or flag, e.g. \"--release\", \
                         \"--all-features\", \"my_test_name\""
                            .into(),
                    ),
                    ..Default::default()
                })),
                ..Default::default()
            }),
        );
        JsonSchema {
            kind: Some("object".into()),
            properties: Some(props),
            required: Some(vec!["subcommand".to_string()]),
            ..Default::default()
        }
    }

    fn run(arguments: serde_json::Value) -> Result<String, TheyaError> {
        let subcommand = arguments
            .as_object()
            .and_then(|o| o.get("subcommand"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        const BLOCKED: &[&str] = &["login", "logout", "publish", "install"];
        if BLOCKED.contains(&subcommand.as_str()) {
            return Ok(format!(
                "FAIL\ncargo subcommand '{subcommand}' is not allowed"
            ));
        }

        let extra: Vec<String> = arguments
            .as_object()
            .and_then(|o| o.get("args"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let mut args: Vec<&str> = vec![&subcommand];
        for a in &extra {
            args.push(a.as_str());
        }

        let (status, stdout, stderr) = run_command("cargo", args.as_slice())?;

        let combined = format!("{stdout}{stderr}").trim().to_string();
        if status.success() {
            Ok(format!("PASS\n{combined}"))
        } else {
            Ok(format!("FAIL\n{combined}"))
        }
    }
}
