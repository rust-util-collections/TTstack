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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_and_remove_file() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base.img");
        let clone = dir.path().join("clone.img");
        std::fs::write(&base, b"image-data").unwrap();

        let store = RawStore;
        store.clone_image(base.to_str().unwrap(), clone.to_str().unwrap()).unwrap();
        assert!(clone.exists());
        assert_eq!(std::fs::read(&clone).unwrap(), b"image-data");

        store.remove_image(clone.to_str().unwrap()).unwrap();
        assert!(!clone.exists());
    }

    #[test]
    fn clone_and_remove_directory() {
        let dir = tempfile::tempdir().unwrap();
        let base_dir = dir.path().join("base");
        let clone_dir = dir.path().join("clone");
        std::fs::create_dir(&base_dir).unwrap();
        std::fs::write(base_dir.join("disk.qcow2"), b"data").unwrap();

        let store = RawStore;
        store.clone_image(base_dir.to_str().unwrap(), clone_dir.to_str().unwrap()).unwrap();
        assert!(clone_dir.join("disk.qcow2").exists());

        store.remove_image(clone_dir.to_str().unwrap()).unwrap();
        assert!(!clone_dir.exists());
    }

    #[test]
    fn list_images_filters_clones_and_hidden() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ubuntu"), b"").unwrap();
        std::fs::write(dir.path().join("alpine"), b"").unwrap();
        std::fs::write(dir.path().join(".hidden"), b"").unwrap();
        std::fs::write(dir.path().join("clone-abc"), b"").unwrap();

        let store = RawStore;
        let images = store.list_images(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(images, vec!["alpine", "ubuntu"]);
    }

    #[test]
    fn list_images_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let store = RawStore;
        let images = store.list_images(dir.path().to_str().unwrap()).unwrap();
        assert!(images.is_empty());
    }

    #[test]
    fn list_images_nonexistent_dir() {
        let store = RawStore;
        let images = store.list_images("/no/such/path").unwrap();
        assert!(images.is_empty());
    }

    #[test]
    fn image_exists_check() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("img");
        let store = RawStore;
        assert!(!store.image_exists(path.to_str().unwrap()).unwrap());
        std::fs::write(&path, b"").unwrap();
        assert!(store.image_exists(path.to_str().unwrap()).unwrap());
    }

    #[test]
    fn remove_nonexistent_is_ok() {
        let store = RawStore;
        store.remove_image("/no/such/file").unwrap();
    }

    #[test]
    fn name_is_raw() {
        assert_eq!(RawStore.name(), "raw");
    }
}
