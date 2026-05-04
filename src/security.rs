// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use super::error::TheyaError;

/// Determines if the provided path is within the current working directory.
/// For paths that do not yet exist, the nearest existing ancestor is checked.
pub(crate) fn is_within_current_dir(path: &str) -> Result<bool, TheyaError> {
    let cwd = std::env::current_dir()?;
    let canonical_cwd = std::fs::canonicalize(&cwd)?;

    // Make the path absolute so that ancestor-walking works correctly for
    // relative paths (e.g. "foo.txt" whose parent would otherwise be "").
    let abs = if Path::new(path).is_absolute() {
        Path::new(path).to_path_buf()
    } else {
        cwd.join(path)
    };

    // Walk up to the first existing ancestor so that new files can be checked.
    let mut check = abs;
    loop {
        match std::fs::canonicalize(&check) {
            Ok(canonical_target) => {
                return Ok(canonical_target.starts_with(&canonical_cwd));
            }
            Err(_) => {
                if let Some(parent) = check.parent() {
                    check = parent.to_path_buf();
                } else {
                    return Ok(false);
                }
            }
        }
    }
}
