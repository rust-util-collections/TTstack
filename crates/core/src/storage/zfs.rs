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
    /// Derive a snapshot name from the base dataset.
    fn snap_name(base: &str) -> String {
        format!("{base}@ttsnap")
    }

    /// Ensure a snapshot exists for the base dataset, creating one if needed.
    fn ensure_snapshot(base: &str) -> Result<()> {
        let snap = Self::snap_name(base);

        // Check if snapshot already exists
        let check = Command::new("zfs")
            .args(["list", "-t", "snapshot", &snap])
            .output()
            .c(d!())?;

        if !check.status.success() {
            // Create the snapshot
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
        Self::ensure_snapshot(base)?;
        let snap = Self::snap_name(base);

        let output = Command::new("zfs")
            .args(["clone", &snap, target])
            .output()
            .c(d!())?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("zfs clone failed: {}", err));
        }

        Ok(())
    }

    fn remove_image(&self, path: &str) -> Result<()> {
        let output = Command::new("zfs")
            .args(["destroy", "-r", path])
            .output()
            .c(d!())?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("zfs destroy failed: {}", err));
        }

        Ok(())
    }

    fn list_images(&self, base_dir: &str) -> Result<Vec<String>> {
        let output = Command::new("zfs")
            .args(["list", "-H", "-o", "name", "-r", base_dir])
            .output()
            .c(d!())?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .lines()
            .filter(|l| !l.is_empty() && *l != base_dir)
            .map(|l| {
                // Return just the leaf name
                l.rsplit('/').next().unwrap_or(l).to_string()
            })
            .collect())
    }

    fn image_exists(&self, path: &str) -> Result<bool> {
        let output = Command::new("zfs")
            .args(["list", "-H", path])
            .output()
            .c(d!())?;

        Ok(output.status.success())
    }

    fn name(&self) -> &'static str {
        "zfs"
    }
}
