//! QEMU/KVM engine implementation.
//!
//! Launches VMs via `qemu-system-x86_64` with KVM acceleration.
//! Each VM gets its own tap device connected to the host bridge.

use super::VmEngine;
use crate::model::{RUN_DIR, Vm, VmState};
use ruc::*;
use std::path::Path;
use std::process::Command;

pub struct QemuEngine;

impl Default for QemuEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl QemuEngine {
    pub fn new() -> Self {
        Self
    }

    /// Resolve the disk image file path.
    ///
    /// If `image_path` is a regular file, use it directly.
    /// If it's a directory (e.g. a ZFS dataset or Btrfs subvolume),
    /// look for a qcow2 file inside it.
    fn resolve_disk(image_path: &str) -> String {
        use std::path::Path;
        let p = Path::new(image_path);
        if p.is_dir() {
            // Look for a .qcow2 file first, then any single file
            if let Ok(entries) = std::fs::read_dir(p) {
                let files: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_file())
                    .collect();
                if let Some(qcow2) = files
                    .iter()
                    .find(|f| f.path().extension().is_some_and(|ext| ext == "qcow2"))
                {
                    return qcow2.path().to_string_lossy().into_owned();
                }
                if files.len() == 1 {
                    return files[0].path().to_string_lossy().into_owned();
                }
            }
            // Fallback: assume disk.qcow2
            format!("{image_path}/disk.qcow2")
        } else {
            image_path.to_string()
        }
    }

    fn build_cmd(&self, vm: &Vm, image_path: &str) -> Command {
        let tap = crate::net::tap_name(&vm.id);
        let disk = Self::resolve_disk(image_path);
        let mut cmd = Command::new("qemu-system-x86_64");
        cmd.args(["-enable-kvm", "-daemonize"])
            .args(["-name", &vm.id])
            .args(["-m", &format!("{}M", vm.mem)])
            .args(["-smp", &vm.cpu.to_string()])
            .args(["-drive", &format!("file={disk},format=qcow2,if=virtio")])
            .args([
                "-netdev",
                &format!("tap,id=net0,ifname={tap},script=no,downscript=no"),
            ])
            .args(["-device", "virtio-net-pci,netdev=net0"])
            .args(["-pidfile", &self.pid_path(vm)])
            .args([
                "-monitor",
                &format!("unix:{},server,nowait", self.monitor_path(vm)),
            ])
            .args(["-vnc", "none"]);

        // Attach cloud-init seed ISO if it exists (for cloud images)
        let seed = self.seed_path(vm);
        if Path::new(&seed).exists() {
            cmd.args([
                "-drive",
                &format!("file={seed},format=raw,if=virtio,readonly=on"),
            ]);
        }

        cmd
    }

    /// Generate a cloud-init NoCloud seed ISO for the VM.
    ///
    /// This allows cloud images (Alpine, Debian, Ubuntu) to auto-configure
    /// on first boot: set root password, enable SSH, configure networking.
    fn generate_seed_iso(&self, vm: &Vm) -> Result<()> {
        let seed_dir = format!("{RUN_DIR}/seed-{}", vm.id);
        std::fs::create_dir_all(&seed_dir).c(d!("create seed dir"))?;

        // meta-data
        let meta_data = format!("instance-id: {}\nlocal-hostname: {}\n", vm.id, vm.id);
        std::fs::write(format!("{seed_dir}/meta-data"), meta_data).c(d!("write meta-data"))?;

        // network-config (v2) — static IP on the virtio NIC
        let network_config = format!(
            r#"version: 2
ethernets:
  id0:
    match:
      driver: virtio_net
    addresses:
      - {ip}/16
    routes:
      - to: 0.0.0.0/0
        via: 10.10.0.1
    nameservers:
      addresses:
        - 8.8.8.8
        - 1.1.1.1
"#,
            ip = vm.ip,
        );
        std::fs::write(format!("{seed_dir}/network-config"), network_config)
            .c(d!("write network-config"))?;

        // user-data — set root password and enable SSH
        let user_data = r#"#cloud-config
password: ttstack
chpasswd:
  expire: false
ssh_pwauth: true
disable_root: false
runcmd:
  - sed -i 's/^#*PermitRootLogin.*/PermitRootLogin yes/' /etc/ssh/sshd_config
  - systemctl restart sshd 2>/dev/null || service sshd restart 2>/dev/null || rc-service sshd restart 2>/dev/null || true
"#;
        std::fs::write(format!("{seed_dir}/user-data"), user_data).c(d!("write user-data"))?;

        // Generate ISO using genisoimage or mkisofs
        let seed_iso = self.seed_path(vm);
        let meta = format!("{seed_dir}/meta-data");
        let user = format!("{seed_dir}/user-data");
        let netcfg = format!("{seed_dir}/network-config");

        let output = if Path::new("/usr/bin/genisoimage").exists() {
            Command::new("genisoimage")
                .args([
                    "-output", &seed_iso, "-volid", "cidata", "-joliet", "-rock", "-quiet",
                ])
                .args([&meta, &user, &netcfg])
                .output()
                .c(d!("generate seed ISO"))?
        } else {
            Command::new("mkisofs")
                .args(["-o", &seed_iso, "-V", "cidata", "-J", "-R", "-quiet"])
                .args([&meta, &user, &netcfg])
                .output()
                .c(d!("generate seed ISO"))?
        };

        // Clean up temp dir
        let _ = std::fs::remove_dir_all(&seed_dir);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("seed ISO creation failed: {}", stderr));
        }

        Ok(())
    }

    fn pid_path(&self, vm: &Vm) -> String {
        format!("{RUN_DIR}/qemu-{}.pid", vm.id)
    }

    fn monitor_path(&self, vm: &Vm) -> String {
        format!("{RUN_DIR}/qemu-{}.sock", vm.id)
    }

    fn seed_path(&self, vm: &Vm) -> String {
        format!("{RUN_DIR}/seed-{}.iso", vm.id)
    }

    fn read_pid(&self, vm: &Vm) -> Result<u32> {
        let path = self.pid_path(vm);
        let content = std::fs::read_to_string(&path).c(d!("read pid file"))?;
        content.trim().parse::<u32>().c(d!("invalid pid"))
    }

    fn process_alive(pid: u32) -> bool {
        Path::new(&format!("/proc/{pid}")).exists()
    }
}

impl VmEngine for QemuEngine {
    fn create(&self, vm: &Vm, image_path: &str) -> Result<()> {
        std::fs::create_dir_all(RUN_DIR).c(d!("create runtime dir"))?;

        // Generate cloud-init seed ISO (best-effort; non-cloud images ignore it)
        if let Err(e) = self.generate_seed_iso(vm) {
            eprintln!(
                "[qemu] WARN: could not create seed ISO for {}: {e} (cloud-init may not work)",
                vm.id
            );
        }

        let output = self
            .build_cmd(vm, image_path)
            .output()
            .c(d!("spawn qemu"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eg!("qemu launch failed: {}", stderr));
        }

        Ok(())
    }

    fn start(&self, vm: &Vm) -> Result<()> {
        let sock = self.monitor_path(vm);
        if !Path::new(&sock).exists() {
            // Monitor socket gone means QEMU exited; re-create the VM.
            return Err(eg!(
                "VM {} has no monitor socket; it must be re-created",
                vm.id
            ));
        }

        // Send "cont" command to the QEMU monitor to resume execution
        let output = Command::new("sh")
            .args([
                "-c",
                &format!(r#"echo "cont" | socat - UNIX-CONNECT:{sock}"#),
            ])
            .output()
            .c(d!("qemu monitor cont"))?;

        if !output.status.success() {
            return Err(eg!("failed to resume VM via QEMU monitor"));
        }
        Ok(())
    }

    fn stop(&self, vm: &Vm) -> Result<()> {
        // Pause the VM via QEMU monitor "stop" command.
        // This freezes the vCPU without killing the QEMU process,
        // allowing later resume via "cont".
        let sock = self.monitor_path(vm);
        if Path::new(&sock).exists() {
            let _ = Command::new("sh")
                .args([
                    "-c",
                    &format!(r#"echo "stop" | socat - UNIX-CONNECT:{sock}"#),
                ])
                .output();
        }
        Ok(())
    }

    fn destroy(&self, vm: &Vm) -> Result<()> {
        if let Ok(pid) = self.read_pid(vm)
            && Self::process_alive(pid)
        {
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::Signal::SIGKILL,
            );
        }

        let _ = std::fs::remove_file(self.pid_path(vm));
        let _ = std::fs::remove_file(self.monitor_path(vm));
        let _ = std::fs::remove_file(self.seed_path(vm));

        Ok(())
    }

    fn state(&self, vm: &Vm) -> Result<VmState> {
        match self.read_pid(vm) {
            Ok(pid) if Self::process_alive(pid) => {
                // Query QEMU monitor to distinguish Running vs Paused
                let sock = self.monitor_path(vm);
                if Path::new(&sock).exists()
                    && let Ok(output) = Command::new("sh")
                        .args([
                            "-c",
                            &format!(r#"echo "info status" | socat - UNIX-CONNECT:{sock}"#),
                        ])
                        .output()
                {
                    let body = String::from_utf8_lossy(&output.stdout);
                    if body.contains("paused") {
                        return Ok(VmState::Paused);
                    }
                }
                Ok(VmState::Running)
            }
            _ => Ok(VmState::Stopped),
        }
    }

    fn name(&self) -> &'static str {
        "qemu"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_disk_plain_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("image.qcow2");
        std::fs::write(&file, b"fake").unwrap();
        let resolved = QemuEngine::resolve_disk(file.to_str().unwrap());
        assert_eq!(resolved, file.to_str().unwrap());
    }

    #[test]
    fn resolve_disk_dir_with_qcow2() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("disk.qcow2"), b"fake").unwrap();
        std::fs::write(dir.path().join("other.txt"), b"other").unwrap();
        let resolved = QemuEngine::resolve_disk(dir.path().to_str().unwrap());
        assert!(resolved.ends_with("disk.qcow2"));
    }

    #[test]
    fn resolve_disk_dir_single_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("myimage"), b"fake").unwrap();
        let resolved = QemuEngine::resolve_disk(dir.path().to_str().unwrap());
        assert!(resolved.ends_with("myimage"));
    }

    #[test]
    fn resolve_disk_dir_fallback() {
        let dir = tempfile::tempdir().unwrap();
        // Multiple files, none .qcow2
        std::fs::write(dir.path().join("a"), b"fake").unwrap();
        std::fs::write(dir.path().join("b"), b"fake").unwrap();
        let resolved = QemuEngine::resolve_disk(dir.path().to_str().unwrap());
        assert!(resolved.ends_with("disk.qcow2")); // fallback
    }

    #[test]
    fn resolve_disk_empty_dir_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = QemuEngine::resolve_disk(dir.path().to_str().unwrap());
        assert!(resolved.ends_with("disk.qcow2"));
    }
}
