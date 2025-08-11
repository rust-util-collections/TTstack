//!
//! # Server Information
//!

use crate::CFG;
use ruc::*;
use parking_lot::RwLock;
use std::{collections::HashMap, sync::{Arc, LazyLock}};
use ttcore::{get_os_info, ImagePath, OsName};

pub(super) static OS_INFO: LazyLock<Arc<RwLock<HashMap<OsName, ImagePath>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

/// 定时扫描镜像信息
pub(crate) async fn refresh_os_info() -> ruc::Result<()> {
    get_os_info(&CFG.image_path)
        .map(|info| *OS_INFO.write() = info)
        .c(d!())
}
