//!
//! # FireCracker Engine
//!

#[cfg(all(feature = "nft", any(feature = "cow", feature = "zfs")))]
mod suitable_env;
#[cfg(any(not(feature = "nft"), not(any(feature = "cow", feature = "zfs"))))]
mod unsuitable_env;

#[cfg(all(feature = "nft", any(feature = "cow", feature = "zfs")))]
pub(super) use suitable_env::*;
#[cfg(any(
    not(feature = "nft"),
    not(any(feature = "cow", feature = "zfs"))
))]
pub(super) use unsuitable_env::*;
