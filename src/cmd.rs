// SPDX-License-Identifier: Apache-2.0

use std::process::{Command, ExitStatus, Stdio};

use super::error::{ErrorKind, TheyaError};

pub(crate) fn run_command(
    cmd: &str,
    args: &[&str],
) -> Result<(ExitStatus, String, String), TheyaError> {
    log::debug!("Invoking command: {} {}", cmd, args.join(" "));
    let output = Command::new(cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(args)
        .output()?;

    let stdout = String::from_utf8(output.stdout).unwrap_or_default();
    log::trace!("Command stdout: {stdout}");
    let stderr = String::from_utf8(output.stderr).unwrap_or_default();
    log::trace!("Command stderr: {stderr}");
    if output.status.success() {
        log::debug!("Command succeeded");
    } else {
        log::warn!(
            "Command failed with {}, {stderr}",
            output.status.code().unwrap_or_default()
        );
    }
    Ok((output.status, stdout, stderr))
}

pub(crate) fn run_command_checked(
    cmd: &str,
    args: &[&str],
) -> Result<String, TheyaError> {
    let (status, stdout, stderr) = run_command(cmd, args)?;

    if status.success() {
        Ok(stdout)
    } else {
        Err(TheyaError::new(
            ErrorKind::Bug,
            format!(
                "Command `{cmd} {}` failed with rc {} and message: {}",
                args.join(" "),
                status.code().unwrap_or_default(),
                stderr
            ),
        ))
    }
}

pub(crate) fn spawn_editor(
    editor: &str,
    file_path: &str,
) -> Result<(), TheyaError> {
    let mut child = Command::new(editor)
        .arg(file_path)
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
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
