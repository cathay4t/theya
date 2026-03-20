// SPDX-License-Identifier: Apache-2.0

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use super::{cmd::run_command, error::TheyaError};

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct GitStore {
    pub(crate) dir: PathBuf,
}

impl GitStore {
    pub(crate) fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub(crate) fn get_cur_patch_content(&self) -> Result<String, TheyaError> {
        self.run("git show")
    }

    pub(crate) fn commit(&self, msg: &str) -> Result<(), TheyaError> {
        // msg might have space, so we do not use `run_command()` here.
        let args = vec!["commit", "-m", msg];
        Command::new("git")
            .args(args)
            .current_dir(self.dir.as_path())
            .output()?;
        Ok(())
    }

    pub(crate) fn get_cur_changed_file_paths(
        &self,
    ) -> Result<Vec<PathBuf>, TheyaError> {
        let output = self.run("git diff HEAD~1 HEAD --name-only")?;
        Ok(output
            .lines()
            .map(|line| PathBuf::from(line.to_string()))
            .collect())
    }

    pub(crate) fn get_cur_patch_titile(&self) -> Result<String, TheyaError> {
        self.run("git log -1 --pretty=%s")
    }

    pub(crate) fn get_file_content(
        &self,
        path: &Path,
    ) -> Result<String, TheyaError> {
        self.run(&format!("git show HEAD:{}", path.display()))
    }

    pub(crate) fn file_list(&self) -> Result<Vec<String>, TheyaError> {
        Ok(self
            .run("git ls-files")?
            .split("\n")
            .filter_map(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            })
            .collect())
    }

    pub(crate) fn git_short_log(
        &self,
        file_list: &[&str],
        count_limit: usize,
    ) -> Result<String, TheyaError> {
        let count_limit_str = format!("{count_limit}");
        let mut args =
            vec!["log", "--oneline", "--max-count", &count_limit_str];
        args.extend_from_slice(file_list);
        // We cannot use `run_command()` because file name might contain space
        // but `run_command()` is using space to split arguments.
        let stdout = Command::new("git")
            .args(args)
            .current_dir(self.dir.as_path())
            .output()?
            .stdout;

        Ok(String::from_utf8(stdout)?)
    }

    pub(crate) fn get_commit(&self, hash: &str) -> Result<String, TheyaError> {
        self.run(&format!("git show {hash}"))
    }

    pub(crate) fn get_commit_of_file(
        &self,
        hash: &str,
        file_path: &str,
    ) -> Result<String, TheyaError> {
        // file_path might have space, so we do not use `run_command()` here.
        let args = vec!["show", hash, file_path];
        let stdout = Command::new("git")
            .args(args)
            .current_dir(self.dir.as_path())
            .output()?
            .stdout;

        Ok(String::from_utf8(stdout)?)
    }

    pub(crate) fn get_root_dir_path(&self) -> Result<String, TheyaError> {
        self.run("git rev-parse --show-toplevel")
            .map(|p| p.trim().to_string())
    }

    pub(crate) fn get_origin_remote_url(&self) -> Result<String, TheyaError> {
        self.run("git remote get-url origin")
            .map(|p| p.trim().to_string())
    }

    fn run(&self, cmds: &str) -> Result<String, TheyaError> {
        run_command(cmds, self.dir.as_path())
    }
}
