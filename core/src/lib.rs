//!
//! # TT 核心实现
//!
//! VM will run faster on FreeBSD,
//! especially in IO-about operations.
//!

#![warn(missing_docs, unused_import_braces, unused_extern_crates)]

mod def;
pub use def::*;

#[cfg(target_os = "freebsd")]
#[cfg(not(feature = "testmock"))]
mod freebsd;
#[cfg(target_os = "freebsd")]
#[cfg(not(feature = "testmock"))]
pub use freebsd::*;

#[cfg(target_os = "linux")]
#[cfg(not(feature = "testmock"))]
mod linux;
#[cfg(target_os = "linux")]
#[cfg(not(feature = "testmock"))]
pub use linux::*;

#[cfg(not(feature = "testmock"))]
mod util {
    use futures::executor::{ThreadPool, ThreadPoolBuilder};
    use lazy_static::lazy_static;
    use myutil::{err::*, *};

    lazy_static! {
        pub(crate) static ref POOL: ThreadPool =
            pnk!(ThreadPoolBuilder::new().pool_size(1).create());
    }

    pub(crate) async fn asleep(sec: u64) {
        futures_timer::Delay::new(std::time::Duration::from_secs(sec)).await;
    }
}

#[cfg(not(feature = "testmock"))]
pub(crate) use util::*;

mod test;

#[cfg(feature = "testmock")]
mod mocker;
#[cfg(feature = "testmock")]
pub use mocker::*;
