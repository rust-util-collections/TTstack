#![cfg(feature = "testmock")]

use crate::{ImagePath, OsName};
use myutil::{err::*, *};
use std::{collections::HashMap, fs};

pub(super) fn get_os_info(
    img_path: &str,
) -> Result<HashMap<OsName, ImagePath>> {
    #[cfg(target_os = "linux")]
    const IMG_PREFIX: &str = "";
    #[cfg(target_os = "freebsd")]
    const IMG_PREFIX: &str = "";

    let res = map! {
        "CentOS7.0".to_lowercase() => format!("{}/{}CentOS7.{}", img_path, IMG_PREFIX, 0),
        "CentOS7.1".to_lowercase() => format!("{}/{}CentOS7.{}", img_path, IMG_PREFIX, 1),
        "CentOS7.2".to_lowercase() => format!("{}/{}CentOS7.{}", img_path, IMG_PREFIX, 2),
        "CentOS7.3".to_lowercase() => format!("{}/{}CentOS7.{}", img_path, IMG_PREFIX, 3),
        "CentOS7.4".to_lowercase() => format!("{}/{}CentOS7.{}", img_path, IMG_PREFIX, 4),
        "CentOS7.5".to_lowercase() => format!("{}/{}CentOS7.{}", img_path, IMG_PREFIX, 5),
        "CentOS7.6".to_lowercase() => format!("{}/{}CentOS7.{}", img_path, IMG_PREFIX, 6),
        "CentOS7.7".to_lowercase() => format!("{}/{}CentOS7.{}", img_path, IMG_PREFIX, 7),
        "CentOS7.8".to_lowercase() => format!("{}/{}CentOS7.{}", img_path, IMG_PREFIX, 8),
    };

    res.values().for_each(|i| {
        info_omit!(fs::File::create(i));
    });

    Ok(res)
}
