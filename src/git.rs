// SPDX-License-Identifier: Apache-2.0

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use super::error::CliError;

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct MyGitStore {
    pub(crate) dir: PathBuf,
}

impl MyGitStore {
    pub(crate) fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub(crate) fn get_cur_patch_content(&self) -> Result<String, CliError> {
        self.run_command("git show")
    }

    pub(crate) fn get_cur_changed_file_paths(
        &self,
    ) -> Result<Vec<PathBuf>, CliError> {
        let output = self.run_command("git diff HEAD~1 HEAD --name-only")?;
        Ok(output
            .lines()
            .map(|line| PathBuf::from(line.to_string()))
            .collect())
    }

    pub(crate) fn get_cur_patch_titile(&self) -> Result<String, CliError> {
        self.run_command("git log -1 --pretty=%s")
    }

    pub(crate) fn get_file_content(
        &self,
        path: &Path,
    ) -> Result<String, CliError> {
        self.run_command(&format!("git show HEAD:{}", path.display()))
    }

    fn run_command(&self, cmds: &str) -> Result<String, CliError> {
        let mut cmd_splitted: Vec<&str> = cmds.split(" ").collect();

        let cmd = cmd_splitted.remove(0);

        Ok(String::from_utf8(
            Command::new(cmd)
                .current_dir(self.dir.as_path())
                .args(cmd_splitted)
                .output()?
                .stdout,
        )?)
    }
}
