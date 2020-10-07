//!
//! # Utils for linux.
//!

use myutil::{err::*, *};
use nix::{
    mount::{self, MsFlags},
    sys::wait,
    unistd,
};

pub(in crate::linux) fn mount_make_rprivate() -> Result<()> {
    mountx(
        None,
        "/",
        None,
        pnk!(MsFlags::from_bits(
            MsFlags::MS_REC.bits() | MsFlags::MS_PRIVATE.bits()
        )),
        None,
    )
    .c(d!())
}

#[cfg(not(feature = "testmock"))]
pub(in crate::linux) fn mount_cgroup2(path: &str) -> Result<()> {
    mountx(None, path, Some("cgroup2"), MsFlags::empty(), None).c(d!())
}

pub(in crate::linux) fn mount_dynfs_proc() -> Result<()> {
    let mut flags = MsFlags::empty();
    flags.insert(MsFlags::MS_NODEV);
    flags.insert(MsFlags::MS_NOEXEC);
    flags.insert(MsFlags::MS_NOSUID);
    flags.insert(MsFlags::MS_RELATIME);

    mountx(None, "/proc", Some("proc"), flags, None).c(d!())
}

pub(in crate::linux) fn mount_tmp_tmpfs() -> Result<()> {
    let mut flags = MsFlags::empty();
    flags.insert(MsFlags::MS_RELATIME);

    mountx(None, "/tmp", Some("tmpfs"), flags, None).c(d!())
}

#[inline(always)]
fn mountx(
    from: Option<&str>,
    to: &str,
    fstype: Option<&str>,
    flags: MsFlags,
    data: Option<&str>,
) -> Result<()> {
    mount::mount(from, to, fstype, flags, data).c(d!())
}

pub(in crate::linux) fn wait_pid() {
    while let Ok(st) = wait::waitpid(
        unistd::Pid::from_raw(-1),
        Some(wait::WaitPidFlag::WNOHANG),
    ) {
        if st == wait::WaitStatus::StillAlive {
            break;
        }
    }
}
