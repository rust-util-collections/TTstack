//!
//! # Server Information
//!

use crate::CFG;
use lazy_static::lazy_static;
use myutil::{err::*, *};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};
use ttcore::{get_os_info, ImagePath, OsName};

lazy_static! {
    pub(super) static ref OS_INFO: Arc<RwLock<HashMap<OsName, ImagePath>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

/// 定时扫描镜像信息
pub(crate) async fn refresh_os_info() -> Result<()> {
    get_os_info(&CFG.image_path)
        .map(|info| *OS_INFO.write() = info)
        .c(d!())
}
