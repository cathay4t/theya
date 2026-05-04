// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;

use crate::{
    cmd::run_command,
    error::TheyaError,
    json_schema::JsonSchema,
    openai::{OpenAiFunctionPrototype, OpenAiToolPrototype},
};

pub(crate) trait ToolHandlerCmd {
    const NAME: &str;
    const DESCRIPTION: &str;

    fn parameters() -> JsonSchema {
        JsonSchema::default()
    }

    fn prototype() -> OpenAiToolPrototype {
        OpenAiFunctionPrototype {
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
    T: Serialize,
{
    const NAME: &str;
    const DESCRIPTION: &str;
    fn parameters() -> JsonSchema;

    fn prototype() -> OpenAiToolPrototype {
        OpenAiFunctionPrototype {
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
