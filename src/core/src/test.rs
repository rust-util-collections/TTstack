#![cfg(feature = "testmock")]

use crate::{ImagePath, OsName};
use myutil::{err::*, *};
use std::{collections::HashMap, fs};

pub(super) fn get_os_info(
    img_path: &str,
) -> Result<HashMap<OsName, ImagePath>> {
    let res = map! {
        "CentOS7.0".to_lowercase() => format!("{}/CentOS7.{}", img_path, 0),
        "CentOS7.1".to_lowercase() => format!("{}/CentOS7.{}", img_path, 1),
        "CentOS7.2".to_lowercase() => format!("{}/CentOS7.{}", img_path, 2),
        "CentOS7.3".to_lowercase() => format!("{}/CentOS7.{}", img_path, 3),
        "CentOS7.4".to_lowercase() => format!("{}/CentOS7.{}", img_path, 4),
        "CentOS7.5".to_lowercase() => format!("{}/CentOS7.{}", img_path, 5),
        "CentOS7.6".to_lowercase() => format!("{}/CentOS7.{}", img_path, 6),
        "CentOS7.7".to_lowercase() => format!("{}/CentOS7.{}", img_path, 7),
        "CentOS7.8".to_lowercase() => format!("{}/CentOS7.{}", img_path, 8),
    };

    res.values().for_each(|i| {
        info_omit!(fs::File::create(i));
    });

    Ok(res)
}
