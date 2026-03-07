//! ZFS zvol storage backend.
//!
//! Uses ZFS volumes (zvols) as raw block devices for VMs. Each base
//! image is a zvol; VM copies are instant clones via snapshots.
//!
//! Advanced features:
//! - **Snapshots**: create, list, destroy, rollback
//! - **zfs send/recv**: full and incremental streams for backup and
//!   cross-host migration
//! - **Property queries**: volsize, used, compressratio, etc.

use super::ImageStore;
use ruc::*;
use std::io::{Read, Write};
use std::process::{Command, Stdio};

/// Fixed snapshot name used for cloning base images.
const CLONE_SNAP: &str = "ttsnap";

pub struct ZvolStore;

// ── Helper: run a zfs command and return stdout or a descriptive error ──

fn zfs_cmd(args: &[&str]) -> Result<String> {
    let output = Command::new("zfs")
        .args(args)
        .output()
        .c(d!("failed to execute zfs"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eg!("zfs {} failed: {}", args[0], stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn zfs_ok(args: &[&str]) -> bool {
    Command::new("zfs")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// ── Snapshot management ─────────────────────────────────────────────

impl ZvolStore {
    /// Create a named snapshot of a zvol.
    ///
    /// Returns the full snapshot name (`dataset@name`).
    pub fn create_snapshot(&self, dataset: &str, snap_name: &str) -> Result<String> {
        let snap = format!("{dataset}@{snap_name}");
        zfs_cmd(&["snapshot", &snap])?;
        Ok(snap)
    }

    /// List all snapshots of a dataset, returning full snapshot names.
    pub fn list_snapshots(&self, dataset: &str) -> Result<Vec<String>> {
        let out = zfs_cmd(&["list", "-H", "-o", "name", "-t", "snapshot", "-r", dataset])?;
        Ok(out
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect())
    }

    /// Destroy a snapshot (full name `dataset@snap`).
    pub fn destroy_snapshot(&self, snap: &str) -> Result<()> {
        zfs_cmd(&["destroy", snap])?;
        Ok(())
    }

    /// Rollback a zvol to the given snapshot.
    ///
    /// **Warning**: destroys all snapshots created after `snap_name`.
    pub fn rollback(&self, dataset: &str, snap_name: &str) -> Result<()> {
        let snap = format!("{dataset}@{snap_name}");
        zfs_cmd(&["rollback", "-r", &snap])?;
        Ok(())
    }

    /// Ensure the clone snapshot (`@ttsnap`) exists on `dataset`.
    fn ensure_clone_snap(dataset: &str) -> Result<()> {
        let snap = format!("{dataset}@{CLONE_SNAP}");
        if !zfs_ok(&["list", "-t", "snapshot", &snap]) {
            zfs_cmd(&["snapshot", &snap])?;
        }
        Ok(())
    }
}

// ── zfs send / recv ─────────────────────────────────────────────────

impl ZvolStore {
    /// Full send: write a complete snapshot stream to a writer.
    ///
    /// `snap` is the full snapshot name, e.g. `tank/images/alpine@backup`.
    pub fn send(&self, snap: &str, out: &mut dyn Write) -> Result<()> {
        let mut child = Command::new("zfs")
            .args(["send", snap])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .c(d!("spawn zfs send"))?;

        if let Some(mut stdout) = child.stdout.take() {
            std::io::copy(&mut stdout, out).c(d!("pipe zfs send stream"))?;
        }

        let status = child.wait().c(d!("wait zfs send"))?;
        if !status.success() {
            return Err(eg!("zfs send failed"));
        }
        Ok(())
    }

    /// Incremental send: write the delta between two snapshots.
    ///
    /// `from_snap` and `to_snap` are full snapshot names on the same dataset.
    pub fn send_incremental(
        &self,
        from_snap: &str,
        to_snap: &str,
        out: &mut dyn Write,
    ) -> Result<()> {
        let mut child = Command::new("zfs")
            .args(["send", "-i", from_snap, to_snap])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .c(d!("spawn zfs send -i"))?;

        if let Some(mut stdout) = child.stdout.take() {
            std::io::copy(&mut stdout, out).c(d!("pipe incremental stream"))?;
        }

        let status = child.wait().c(d!("wait zfs send -i"))?;
        if !status.success() {
            return Err(eg!("zfs send -i failed"));
        }
        Ok(())
    }

    /// Receive a zfs stream into a new or existing dataset.
    pub fn recv(&self, dataset: &str, input: &mut dyn Read) -> Result<()> {
        let mut child = Command::new("zfs")
            .args(["recv", dataset])
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .c(d!("spawn zfs recv"))?;

        if let Some(mut stdin) = child.stdin.take() {
            std::io::copy(input, &mut stdin).c(d!("pipe to zfs recv"))?;
        }

        let status = child.wait().c(d!("wait zfs recv"))?;
        if !status.success() {
            return Err(eg!("zfs recv failed"));
        }
        Ok(())
    }

    /// Convenience: full send to a file.
    pub fn send_to_file(&self, snap: &str, dest: &str) -> Result<()> {
        let mut f = std::fs::File::create(dest).c(d!("create send file"))?;
        self.send(snap, &mut f)
    }

    /// Convenience: incremental send to a file.
    pub fn send_incremental_to_file(
        &self,
        from_snap: &str,
        to_snap: &str,
        dest: &str,
    ) -> Result<()> {
        let mut f = std::fs::File::create(dest).c(d!("create send file"))?;
        self.send_incremental(from_snap, to_snap, &mut f)
    }

    /// Convenience: recv from a file.
    pub fn recv_from_file(&self, dataset: &str, src: &str) -> Result<()> {
        let mut f = std::fs::File::open(src).c(d!("open recv file"))?;
        self.recv(dataset, &mut f)
    }

    /// Build a `zfs send` command for piping to external programs (e.g. ssh).
    pub fn send_cmd(&self, snap: &str) -> Command {
        let mut cmd = Command::new("zfs");
        cmd.args(["send", snap]);
        cmd
    }

    /// Build an incremental `zfs send -i` command.
    pub fn send_incremental_cmd(&self, from_snap: &str, to_snap: &str) -> Command {
        let mut cmd = Command::new("zfs");
        cmd.args(["send", "-i", from_snap, to_snap]);
        cmd
    }

    /// Build a `zfs recv` command for piping from external programs.
    pub fn recv_cmd(&self, dataset: &str) -> Command {
        let mut cmd = Command::new("zfs");
        cmd.args(["recv", dataset]);
        cmd
    }
}

// ── Property queries ────────────────────────────────────────────────

impl ZvolStore {
    /// Get a ZFS property value (e.g. `"volsize"`, `"used"`, `"compressratio"`).
    pub fn get_property(&self, dataset: &str, prop: &str) -> Result<String> {
        zfs_cmd(&["get", "-H", "-o", "value", prop, dataset])
    }

    /// Set a ZFS property.
    pub fn set_property(&self, dataset: &str, prop: &str, value: &str) -> Result<()> {
        zfs_cmd(&["set", &format!("{prop}={value}"), dataset])?;
        Ok(())
    }
}

// ── ImageStore implementation ───────────────────────────────────────

impl ImageStore for ZvolStore {
    fn clone_image(&self, base: &str, target: &str) -> Result<()> {
        Self::ensure_clone_snap(base)?;
        let snap = format!("{base}@{CLONE_SNAP}");
        zfs_cmd(&["clone", &snap, target])?;
        Ok(())
    }

    fn remove_image(&self, path: &str) -> Result<()> {
        zfs_cmd(&["destroy", "-r", path])?;
        Ok(())
    }

    fn list_images(&self, base_dir: &str) -> Result<Vec<String>> {
        let out = match zfs_cmd(&["list", "-H", "-o", "name", "-r", "-t", "volume", base_dir]) {
            Ok(o) => o,
            Err(_) => return Ok(vec![]),
        };

        Ok(out
            .lines()
            .filter(|l| !l.is_empty() && *l != base_dir)
            .filter_map(|l| l.rsplit('/').next())
            .map(String::from)
            .collect())
    }

    fn image_exists(&self, path: &str) -> Result<bool> {
        Ok(zfs_ok(&["list", "-H", path]))
    }

    fn resolve_disk(&self, clone_path: &str) -> String {
        format!("/dev/zvol/{clone_path}")
    }

    fn disk_format(&self) -> &'static str {
        "raw"
    }

    fn name(&self) -> &'static str {
        "zvol"
    }
}
