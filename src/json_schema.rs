// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct JsonSchema {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) items: Option<Box<JsonSchema>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) properties: Option<HashMap<String, Box<JsonSchema>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) required: Option<Vec<String>>,
}
