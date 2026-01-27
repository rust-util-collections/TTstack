//! ZFS storage backend.
//!
//! Uses ZFS snapshots and clones for near-instant, space-efficient
//! image provisioning. Base images are ZFS datasets; VM copies are
//! clones of an automatic snapshot.

use super::ImageStore;
use ruc::*;
use std::process::Command;

pub struct ZfsStore;

impl ZfsStore {
    /// Convert a filesystem path to a ZFS dataset name.
    ///
    /// ZFS datasets are mounted at their mountpoint, so a path like
    /// `/ttpool/images/foo` corresponds to dataset `ttpool/images/foo`.
    /// We resolve this by querying `zfs list` for the parent mountpoint.
    fn path_to_dataset(path: &str) -> Result<String> {
        // Try the path as-is first (in case it's already a dataset name)
        let check = Command::new("zfs")
            .args(["list", "-H", "-o", "name", path])
            .output();
        if let Ok(ref out) = check {
            if out.status.success() {
                return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
            }
        }

        // Strip leading '/' and try as dataset name
        let stripped = path.strip_prefix('/').unwrap_or(path);
        let check = Command::new("zfs")
            .args(["list", "-H", "-o", "name", stripped])
            .output();
        if let Ok(ref out) = check {
            if out.status.success() {
                return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
            }
        }

        // Assume it's a filesystem path: strip leading '/' to form dataset name
        Ok(stripped.to_string())
    }

    /// Derive a snapshot name from the base dataset.
    fn snap_name(base: &str) -> String {
        format!("{base}@ttsnap")
    }

    /// Ensure a snapshot exists for the base dataset, creating one if needed.
    fn ensure_snapshot(dataset: &str) -> Result<()> {
        let snap = Self::snap_name(dataset);

        let check = Command::new("zfs")
            .args(["list", "-t", "snapshot", &snap])
            .output()
            .c(d!())?;

        if !check.status.success() {
            let output = Command::new("zfs")
                .args(["snapshot", &snap])
                .output()
                .c(d!())?;

            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                return Err(eg!("zfs snapshot failed: {}", err));
            }
        }

        Ok(())
    }
}

impl ImageStore for ZfsStore {
    fn clone_image(&self, base: &str, target: &str) -> Result<()> {
        let base_ds = Self::path_to_dataset(base)?;
        let target_ds = Self::path_to_dataset(target)?;

        Self::ensure_snapshot(&base_ds)?;
        let snap = Self::snap_name(&base_ds);

        let output = Command::new("zfs")
            .args(["clone", &snap, &target_ds])
            .output()
            .c(d!())?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("zfs clone failed: {}", err));
        }

        Ok(())
    }

    fn remove_image(&self, path: &str) -> Result<()> {
        let ds = Self::path_to_dataset(path)?;
        let output = Command::new("zfs")
            .args(["destroy", "-r", &ds])
            .output()
            .c(d!())?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("zfs destroy failed: {}", err));
        }

        Ok(())
    }

    fn list_images(&self, base_dir: &str) -> Result<Vec<String>> {
        let ds = Self::path_to_dataset(base_dir)?;
        let output = Command::new("zfs")
            .args(["list", "-H", "-o", "name", "-r", &ds])
            .output()
            .c(d!())?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .lines()
            .filter(|l| !l.is_empty() && *l != ds)
            .map(|l| {
                l.rsplit('/').next().unwrap_or(l).to_string()
            })
            .collect())
    }

    fn image_exists(&self, path: &str) -> Result<bool> {
        let ds = Self::path_to_dataset(path)?;
        let output = Command::new("zfs")
            .args(["list", "-H", &ds])
            .output()
            .c(d!())?;

        Ok(output.status.success())
    }

    fn name(&self) -> &'static str {
        "zfs"
    }
}
