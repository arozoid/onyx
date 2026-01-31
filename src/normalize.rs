use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use walkdir::WalkDir;

use crate::helpers::ONYX_DIR;

/// Normalizes permissions for everything in ONYX_DIR
/// - base rootfs: read-only (dirs 755, files 644, executables 755)
/// - delta layers: user-writable (dirs 700, files 600, executables 700)
/// - preserves symlinks, devices, sockets
/// - skips overlay `work/` directories
pub fn normalize_onyx_dir() -> std::io::Result<()> {
    let onyx_dir = ONYX_DIR.as_path();

    for entry in fs::read_dir(onyx_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // special case: tmp directory
            if path.file_name().map(|n| n == "tmp").unwrap_or(false) {
                fs::set_permissions(&path, Permissions::from_mode(0o1777))?;
                continue;
            }

            let is_delta = path.file_name()
                .map(|name| name == "delta")
                .unwrap_or(false);

            if is_delta {
                // normalize all user delta trees
                for delta_uid_entry in fs::read_dir(&path)? {
                    let delta_uid = delta_uid_entry?;
                    let delta_uid_path = delta_uid.path();

                    // delta/sys/<box>
                    if delta_uid_path.is_dir() {
                        for box_entry in fs::read_dir(&delta_uid_path)? {
                            let box_path = box_entry?.path();
                            normalize_path(&box_path, true)?;
                        }
                    }
                }
            } else {
                // everything else: treat as base rootfs / system files
                normalize_path(&path, false)?;
            }
        }
    }

    Ok(())
}

/// Recursive normalization for a given path
fn normalize_path(root_path: &Path, is_delta: bool) -> std::io::Result<()> {
    let (dir_mode, file_mode, exe_mode) = if is_delta {
        (0o700, 0o600, 0o700)
    } else {
        (0o755, 0o644, 0o755)
    };

    for entry in WalkDir::new(root_path).follow_links(false) {
        let entry = entry?;
        let path = entry.path();

        // skip overlay work directories
        if path.file_name().map(|s| s == "work").unwrap_or(false) {
            continue;
        }

        let metadata = fs::symlink_metadata(path)?;

        if metadata.file_type().is_dir() {
            fs::set_permissions(path, Permissions::from_mode(dir_mode))?;
        } else if metadata.file_type().is_file() {
            // detect executables
            let is_executable = metadata.permissions().mode() & 0o111 != 0
                || path.extension()
                    .map(|ext| matches!(ext.to_str().unwrap(), "sh" | "py" | "pl" | "rb"))
                    .unwrap_or(false);

            let mode = if is_executable { exe_mode } else { file_mode };
            fs::set_permissions(path, Permissions::from_mode(mode))?;
        } else if metadata.file_type().is_symlink() {
            // leave symlinks untouched
            continue;
        } else {
            // device, fifo, socket, etc. → preserve as-is
            continue;
        }
    }

    Ok(())
}