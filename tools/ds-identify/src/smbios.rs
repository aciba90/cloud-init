use std::io;
use std::{fs, path::Path, process::Command};

use crate::paths::Paths;

enum Keys {
    SysVendor,
    ProductName,
    ProductUuid,
    ProductSerial,
    ChassisAssetTag,
    BoardName,
}

impl Keys {
    fn get_dmi_field(&self) -> &str {
        match &self {
            Self::SysVendor => "system-manufacturer",
            Self::ProductName => "system-product-name",
            Self::ProductUuid => "system-uuid",
            Self::ProductSerial => "system-serial-number",
            Self::ChassisAssetTag => "chassis-asset-tag",
            Self::BoardName => panic!("asdfasdf"),
        }
    }

    fn get_dmi_file(&self) -> &str {
        match &self {
            Self::SysVendor => "sys_vendor",
            Self::ProductName => "product_name",
            Self::ProductUuid => "product_uuid",
            Self::ProductSerial => "product_serial",
            Self::ChassisAssetTag => "chassis_asset_tag",
            Self::BoardName => "board_name",
        }
    }
}

pub struct SMBIOS {
    pub sys_vendor: Option<String>,
    pub board_name: Option<String>,
    pub chassis_asset_tag: Option<String>,
    pub product_name: Option<String>,
    pub product_serial: Option<String>,
    pub product_uuid: Option<String>,
}

impl SMBIOS {
    pub fn from_kernel_name(kernel_name: &str, paths: &Paths) -> Self {
        match kernel_name {
            "FreeBSD" => todo!(),
            _ => Self::read_from_dmi(&paths.sys_class_dmi_id),
        }
    }

    pub fn read_from_dmi(sys_class_dmi_id: &Path) -> Self {
        Self {
            sys_vendor: get_dmi_field(sys_class_dmi_id, Keys::SysVendor),
            product_name: get_dmi_field(sys_class_dmi_id, Keys::ProductName),
            product_uuid: get_dmi_field(sys_class_dmi_id, Keys::ProductUuid),
            product_serial: get_dmi_field(sys_class_dmi_id, Keys::ProductSerial),
            chassis_asset_tag: get_dmi_field(sys_class_dmi_id, Keys::ChassisAssetTag),
            board_name: get_dmi_field(sys_class_dmi_id, Keys::BoardName),
        }
    }
}

fn get_dmi_field(sys_class_dmi_id: &Path, key: Keys) -> Option<String> {
    let path = sys_class_dmi_id.join(key.get_dmi_file());
    if sys_class_dmi_id.is_dir() {
        if path.is_file() {
            dbg!(&path);
            match fs::read_to_string(&path) {
                Err(e) => match e.kind() {
                    io::ErrorKind::PermissionDenied => {
                        return None;
                    }
                    _ => panic!("Error reading {}: {}", &path.display(), e),
                },
                Ok(content) => {
                    return Some(content);
                }
            }
        }
        // if `/sys/class/dmi/id` exists, but not the object we're looking for,
        // do *not* fallback to dmidecode!
        return None;
    }
    dmi_decode(&key)
}

fn dmi_decode(sys_field: &Keys) -> Option<String> {
    match &sys_field {
        Keys::BoardName => return None,
        _ => {
            let key = sys_field.get_dmi_field();
            match Command::new("dmidecode")
                .arg("--quiet")
                .arg(format!("--string={}", key))
                .output()
            {
                Err(_) => {
                    // TODO: log error
                    None
                }
                Ok(out) => {
                    // TODO: check status
                    // XXX: simplify this
                    Some(
                        std::str::from_utf8(&out.stdout)
                            .expect("valid string")
                            .to_string(),
                    )
                }
            }
        }
    }
}
