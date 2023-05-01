//!
//! # Error Definition
//!
//! Self-descriptive error type, need NOT extra comments.
//!

#![allow(missing_docs)]

/// Used in error_chain state
#[macro_export]
macro_rules! e {
    ($err: expr) => {
        d!(format!("{}({})", stringify!($err), $err))
    };
}

/// Used in return state
#[macro_export]
macro_rules! fail {
    ($err: expr) => {
        Err(eg!(format!("{}({})", stringify!($err), $err)))
    };
}

pub const ERR_TT_UNKNOWN: i32 = -1;
pub const ERR_TT_SYS_IO: i32 = -2;

pub const ERR_TT_CREATE_VM: i32 = -50;
pub const ERR_TT_DESTROY_VM: i32 = -51;
pub const ERR_TT_START_VM: i32 = -52;
pub const ERR_TT_STOP_VM: i32 = -53;
pub const ERR_TT_UPDATE_VM: i32 = -54;

pub const ERR_TT_META_CREATE_CACHE: i32 = -100;
pub const ERR_TT_META_REMOVE_CACHE: i32 = -101;
pub const ERR_TT_META_RESTORE_CACHE: i32 = -102;

pub const ERR_TT_STORAGE_CREATE_IMAGE: i32 = -200;
pub const ERR_TT_STORAGE_DESTROY_IMAGE: i32 = -201;

pub const ERR_TT_NET_SET_NET: i32 = -300;
pub const ERR_TT_NET_UNSET_NET: i32 = -301;
pub const ERR_TT_NET_DENY_OUTGOING: i32 = -302;
pub const ERR_TT_NET_ALLOW_OUTGOING: i32 = -303;
pub const ERR_TT_NET_SET_OUTGOING_BLACKLIST: i32 = -303;

pub const ERR_TT_SNAPSHOT_CREATE: i32 = -400;
pub const ERR_TT_SNAPSHOT_DESTROY: i32 = -401;
pub const ERR_TT_SNAPSHOT_APPLY: i32 = -402;

pub const ERR_TT_CTRL_UPDATE_TEMPLATE: i32 = -500;
pub const ERR_TT_CTRL_TEMPLATE_NOT_FOUND: i32 = -501;
pub const ERR_TT_CTRL_ENGINE_NOT_FOUND: i32 = -502;
