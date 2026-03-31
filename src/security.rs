// SPDX-License-Identifier: Apache-2.0

use super::error::TheyaError;

/// Determines if the provided path is within the current working directory.
pub(crate) fn is_within_current_dir(path: &str) -> Result<bool, TheyaError> {
    let cwd = std::env::current_dir()?;

    let canonical_cwd = std::fs::canonicalize(&cwd)?;
    let canonical_target = std::fs::canonicalize(path)?;

    Ok(canonical_target.starts_with(&canonical_cwd))
}
