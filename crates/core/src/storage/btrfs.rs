//! Btrfs storage backend.
//!
//! Uses Btrfs subvolume snapshots for efficient image cloning.
//! Base images are Btrfs subvolumes; VM copies are writable snapshots.

use super::ImageStore;
use ruc::*;
use std::path::Path;
use std::process::Command;

pub struct BtrfsStore;

impl ImageStore for BtrfsStore {
    fn clone_image(&self, base: &str, target: &str) -> Result<()> {
        let output = Command::new("btrfs")
            .args(["subvolume", "snapshot", base, target])
            .output()
            .c(d!("failed to create btrfs snapshot"))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("btrfs snapshot failed: {}", err));
        }

        Ok(())
    }

    fn remove_image(&self, path: &str) -> Result<()> {
        let output = Command::new("btrfs")
            .args(["subvolume", "delete", path])
            .output()
            .c(d!("failed to delete btrfs subvolume"))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("btrfs delete failed: {}", err));
        }

        Ok(())
    }

    fn list_images(&self, base_dir: &str) -> Result<Vec<String>> {
        let output = Command::new("btrfs")
            .args(["subvolume", "list", "-o", base_dir])
            .output()
            .c(d!())?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .lines()
            .filter_map(|line| {
                // Output format: "ID ... path <relative_path>"
                line.rsplit_once("path ").map(|(_, p)| {
                    Path::new(p)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| p.to_string())
                })
            })
            .collect())
    }

    fn image_exists(&self, path: &str) -> Result<bool> {
        Ok(Path::new(path).exists())
    }

    fn name(&self) -> &'static str {
        "btrfs"
    }
}
