//!
//! # TT Core Implementation
//!

#![warn(missing_docs, unused_import_braces, unused_extern_crates)]

mod def;
pub use def::*;

#[cfg(all(target_os = "linux", not(feature = "testmock")))]
mod linux;
#[cfg(all(target_os = "linux", not(feature = "testmock")))]
pub use linux::*;

#[cfg(all(target_os = "linux", not(feature = "testmock")))]
mod common {
    use crate::Vm;
    use futures::executor::{ThreadPool, ThreadPoolBuilder};
    use ruc::*;
    use std::{path::PathBuf, sync::LazyLock};

    pub(crate) const CLONE_MARK: &str = "clone_";

    pub(crate) static POOL: LazyLock<ThreadPool> =
        LazyLock::new(|| pnk!(ThreadPoolBuilder::new().pool_size(1).create()));

    pub(crate) async fn asleep(sec: u64) {
        futures_timer::Delay::new(std::time::Duration::from_secs(sec)).await;
    }

    #[cfg(feature = "zfs")]
    pub(crate) static ZFS_ROOT: LazyLock<&'static str> =
        LazyLock::new(|| pnk!(imgroot_register(None)));

    #[cfg(feature = "zfs")]
    pub(crate) fn imgroot_register(
        imgpath: Option<&str>,
    ) -> Option<&'static str> {
        static mut ROOT: Option<String> = None;
        if let Some(path) = imgpath {
            unsafe {
                ROOT.replace(
                    path.trim_start_matches("/dev/zvol/")
                        .trim_end_matches('/')
                        .to_owned(),
                );
            }
        }

        unsafe { ROOT.as_deref() }
    }

    // VM image naming format:
    // - ${CLONE_MARK}_VmId
    #[inline(always)]
    pub(crate) fn vmimg_path(vm: &Vm) -> PathBuf {
        let mut vmimg_path = vm.image_path.clone();
        let vmimg_name = format!("{}{}", CLONE_MARK, vm.id);
        vmimg_path.set_file_name(vmimg_name);
        vmimg_path
    }
}

#[cfg(all(target_os = "linux", not(feature = "testmock")))]
pub(crate) use common::*;

mod test;

// Use mocker for testmock feature OR non-Linux platforms
#[cfg(any(feature = "testmock", not(target_os = "linux")))]
mod mocker;
#[cfg(any(feature = "testmock", not(target_os = "linux")))]
pub use mocker::*;

// Provide common constants for non-Linux platforms
#[cfg(not(target_os = "linux"))]
mod non_linux_common {
    use crate::Vm;
    use std::path::PathBuf;
    
    pub(crate) const CLONE_MARK: &str = "clone_";
    
    // VM image naming format:
    // - ${CLONE_MARK}_VmId
    #[inline(always)]
    pub(crate) fn vmimg_path(vm: &Vm) -> PathBuf {
        let mut vmimg_path = vm.image_path.clone();
        let vmimg_name = format!("{}{}", CLONE_MARK, vm.id());
        vmimg_path.set_file_name(vmimg_name);
        vmimg_path
    }
}


