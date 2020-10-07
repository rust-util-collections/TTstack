//!
//! # TT 核心实现
//!

#![cfg(target_os = "linux")]
#![warn(missing_docs, unused_import_braces, unused_extern_crates)]

mod def;
pub use def::*;

#[cfg(not(feature = "testmock"))]
mod linux;
#[cfg(not(feature = "testmock"))]
pub use linux::*;

#[cfg(not(feature = "testmock"))]
mod common {
    use crate::Vm;
    use futures::executor::{ThreadPool, ThreadPoolBuilder};
    use lazy_static::lazy_static;
    use myutil::{err::*, *};
    use std::path::PathBuf;

    pub(crate) const CLONE_MARK: &str = "clone_";

    lazy_static! {
        pub(crate) static ref POOL: ThreadPool =
            pnk!(ThreadPoolBuilder::new().pool_size(1).create());
    }

    pub(crate) async fn asleep(sec: u64) {
        futures_timer::Delay::new(std::time::Duration::from_secs(sec)).await;
    }

    #[cfg(feature = "zfs")]
    lazy_static! {
        pub(crate) static ref ZFS_ROOT: &'static str =
            pnk!(imgroot_register(None));
    }

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

    // VM image 命名格式为:
    // - ${CLONE_MARK}_VmId
    #[inline(always)]
    pub(crate) fn vmimg_path(vm: &Vm) -> PathBuf {
        let mut vmimg_path = vm.image_path.clone();
        let vmimg_name = format!("{}{}", CLONE_MARK, vm.id);
        vmimg_path.set_file_name(vmimg_name);
        vmimg_path
    }
}

#[cfg(not(feature = "testmock"))]
pub(crate) use common::*;

mod test;

#[cfg(feature = "testmock")]
mod mocker;
#[cfg(feature = "testmock")]
pub use mocker::*;
