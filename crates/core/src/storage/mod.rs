//! Image storage abstraction.
//!
//! Supports ZFS snapshots/clones, Btrfs subvolume snapshots, and
//! plain file copies as storage backends for VM images.

pub mod btrfs;
pub mod raw;
pub mod zfs;

use crate::model::Storage;
use ruc::*;

/// Trait for image storage operations.
///
/// Implementations handle the mechanics of cloning base images into
/// per-VM working copies and cleaning them up on destruction.
pub trait ImageStore: Send + Sync {
    /// Clone a base image to a new path for a VM instance.
    ///
    /// - `base`: path to the base / template image
    /// - `target`: desired path for the VM's working copy
    fn clone_image(&self, base: &str, target: &str) -> Result<()>;

    /// Remove a VM's image clone.
    fn remove_image(&self, path: &str) -> Result<()>;

    /// List available base images under the given directory.
    fn list_images(&self, base_dir: &str) -> Result<Vec<String>>;

    /// Check whether an image exists at the given path.
    fn image_exists(&self, path: &str) -> Result<bool>;

    /// Backend name for logging.
    fn name(&self) -> &'static str;
}

/// Create an [`ImageStore`] for the given backend.
pub fn create_store(kind: Storage) -> Box<dyn ImageStore> {
    match kind {
        Storage::Zfs => Box::new(zfs::ZfsStore),
        Storage::Btrfs => Box::new(btrfs::BtrfsStore),
        Storage::Raw => Box::new(raw::RawStore),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_store_names() {
        assert_eq!(create_store(Storage::Raw).name(), "raw");
        assert_eq!(create_store(Storage::Zfs).name(), "zfs");
        assert_eq!(create_store(Storage::Btrfs).name(), "btrfs");
    }
}
