//! Raw file storage backend.
//!
//! Uses plain file copies for image provisioning. Simplest backend,
//! works on any filesystem, but slower and uses more disk space than
//! ZFS or Btrfs snapshot-based approaches.

use super::ImageStore;
use ruc::*;
use std::path::Path;

pub struct RawStore;

impl ImageStore for RawStore {
    fn clone_image(&self, base: &str, target: &str) -> Result<()> {
        // Use cp to copy the image; on Linux, --reflink=auto enables CoW
        // if supported. On FreeBSD, use -a (archive mode) without GNU flags.
        let mut cmd = std::process::Command::new("cp");
        #[cfg(target_os = "linux")]
        cmd.args(["--reflink=auto", "-a", base, target]);
        #[cfg(not(target_os = "linux"))]
        cmd.args(["-a", base, target]);
        let output = cmd.output().c(d!("cp image"))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("image copy failed: {}", err));
        }

        Ok(())
    }

    fn remove_image(&self, path: &str) -> Result<()> {
        let p = Path::new(path);
        if p.is_dir() {
            std::fs::remove_dir_all(p).c(d!("remove dir"))?;
        } else if p.exists() {
            std::fs::remove_file(p).c(d!("remove file"))?;
        }
        Ok(())
    }

    fn list_images(&self, base_dir: &str) -> Result<Vec<String>> {
        let dir = Path::new(base_dir);
        if !dir.is_dir() {
            return Ok(vec![]);
        }

        let mut images = Vec::new();
        for entry in std::fs::read_dir(dir).c(d!("read image dir"))? {
            let entry = entry.c(d!("read dir entry"))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with('.') && !name.starts_with("clone-") {
                images.push(name);
            }
        }
        images.sort();
        Ok(images)
    }

    fn image_exists(&self, path: &str) -> Result<bool> {
        Ok(Path::new(path).exists())
    }

    fn name(&self) -> &'static str {
        "raw"
    }
}
