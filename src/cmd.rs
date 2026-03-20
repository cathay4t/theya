// SPDX-License-Identifier: Apache-2.0

use std::process::Command;

use super::error::TheyaError;

pub(crate) fn run_command(
    cmds: &str,
    cwd: &std::path::Path,
) -> Result<String, TheyaError> {
    let mut cmd_splitted: Vec<&str> = cmds.split(" ").collect();

    let stdout = Command::new(cmd_splitted.remove(0))
        .args(cmd_splitted)
        .current_dir(cwd)
        .output()?
        .stdout;

    Ok(String::from_utf8(stdout)?)
}

pub(crate) fn spawn_editor(
    editor: &str,
    file_path: &std::path::Path,
) -> Result<(), TheyaError> {
    let mut child = Command::new(editor)
        .arg(file_path)
        .stdin(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .spawn()?;
    if !child
        .wait()
        .map_err(|e| {
            TheyaError::from(format!("Editor '{editor}' failed with {e}"))
        })?
        .success()
    {
        return Err(TheyaError::from(format!("Editor '{editor}' failed")));
    }
    Ok(())
}
