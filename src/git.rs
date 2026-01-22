// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};

use super::{cmd::run_command, error::CliError};

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct MyGitStore {
    pub(crate) dir: PathBuf,
}

impl MyGitStore {
    pub(crate) fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub(crate) fn get_cur_patch_content(&self) -> Result<String, CliError> {
        self.run("git show")
    }

    pub(crate) fn get_cur_changed_file_paths(
        &self,
    ) -> Result<Vec<PathBuf>, CliError> {
        let output = self.run("git diff HEAD~1 HEAD --name-only")?;
        Ok(output
            .lines()
            .map(|line| PathBuf::from(line.to_string()))
            .collect())
    }

    pub(crate) fn get_cur_patch_titile(&self) -> Result<String, CliError> {
        self.run("git log -1 --pretty=%s")
    }

    pub(crate) fn get_file_content(
        &self,
        path: &Path,
    ) -> Result<String, CliError> {
        self.run(&format!("git show HEAD:{}", path.display()))
    }

    fn run(&self, cmds: &str) -> Result<String, CliError> {
        run_command(cmds, self.dir.as_path())
    }
}
