// SPDX-License-Identifier: Apache-2.0

#[derive(Clone, Debug)]
pub(crate) struct CliError {
    msg: String,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        Self {
            msg: format!("std::io::Error: {}", e),
        }
    }
}

impl From<reqwest::Error> for CliError {
    fn from(e: reqwest::Error) -> Self {
        Self {
            msg: format!("reqwest::Error: {}", e),
        }
    }
}

impl From<std::string::FromUtf8Error> for CliError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self {
            msg: format!("std::string::FromUtf8Error: {}", e),
        }
    }
}

impl From<&str> for CliError {
    fn from(msg: &str) -> Self {
        Self {
            msg: msg.to_string(),
        }
    }
}

impl From<String> for CliError {
    fn from(msg: String) -> Self {
        Self { msg }
    }
}
