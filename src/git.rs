// SPDX-License-Identifier: Apache-2.0

use super::{cmd::run_command_checked, error::TheyaError};

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct Git {}

impl Git {
    pub(crate) fn get_cur_patch_titile() -> Result<String, TheyaError> {
        Self::run(&["log", "-1", "--pretty=%s"])
    }

    pub(crate) fn file_list(
        path: Option<&str>,
    ) -> Result<Vec<String>, TheyaError> {
        let path = path.unwrap_or("./");
        let args = vec!["ls-files", path];
        Self::run(&args).map(|output| {
            output
                .split("\n")
                .filter_map(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.to_string())
                    }
                })
                .collect()
        })
    }

    pub(crate) fn get_root_dir_path() -> Result<String, TheyaError> {
        Self::run(&["rev-parse", "--show-toplevel"])
            .map(|p| p.trim().to_string())
    }

    pub(crate) fn get_origin_remote_url() -> Result<String, TheyaError> {
        Self::run(&["remote", "get-url", "origin"])
            .map(|p| p.trim().to_string())
    }

    fn run(args: &[&str]) -> Result<String, TheyaError> {
        run_command_checked("git", args)
    }
}
