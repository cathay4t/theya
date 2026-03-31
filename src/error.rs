// SPDX-License-Identifier: Apache-2.0

#[derive(Clone, PartialEq, Eq, Copy, Debug)]
pub(crate) enum ErrorKind {
    Bug,
    EmptyInput,
    IoError,
    HttpError,
    AiInvalidReply,
    JsonError,
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct TheyaError {
    kind: ErrorKind,
    msg: String,
}

impl TheyaError {
    pub(crate) fn new(kind: ErrorKind, msg: String) -> Self {
        Self { kind, msg }
    }
}

impl std::fmt::Display for TheyaError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.msg)
    }
}

impl std::error::Error for TheyaError {}

impl From<std::io::Error> for TheyaError {
    fn from(e: std::io::Error) -> Self {
        Self {
            kind: ErrorKind::IoError,
            msg: format!("std::io::Error: {}", e),
        }
    }
}

impl From<reqwest::Error> for TheyaError {
    fn from(e: reqwest::Error) -> Self {
        Self {
            kind: ErrorKind::HttpError,
            msg: format!("reqwest::Error: {}", e),
        }
    }
}

impl From<std::string::FromUtf8Error> for TheyaError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self {
            kind: ErrorKind::Bug,
            msg: format!("std::string::FromUtf8Error: {}", e),
        }
    }
}

impl From<serde_json::Error> for TheyaError {
    fn from(e: serde_json::Error) -> Self {
        Self {
            kind: ErrorKind::JsonError,
            msg: format!("serde_json::Error: {}", e),
        }
    }
}

impl From<&str> for TheyaError {
    fn from(msg: &str) -> Self {
        Self {
            kind: ErrorKind::Bug,
            msg: msg.to_string(),
        }
    }
}

impl From<String> for TheyaError {
    fn from(msg: String) -> Self {
        Self {
            kind: ErrorKind::Bug,
            msg,
        }
    }
}
