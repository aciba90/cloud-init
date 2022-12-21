use std::{fs, path::Path};

type Field<'a> = Option<&'a str>;

pub trait SMBIOS<'a> {
    fn sys_vendor(&'a self) -> Field<'a>;
}

enum Keys {
    SysVendor,
    ProductName,
    ProductUuid,
    ProductSerial,
}

impl Keys {
    fn get_dmi_field(&self) -> &str {
        match &self {
            Self::SysVendor => "system-manufacturer",
            Self::ProductName => "system-product-name",
            Self::ProductUuid => "system-uuid",
            Self::ProductSerial => "system-serial-number",
        }
    }
}

pub struct Dmi {
    sys_vendor: Option<String>,
    // board_name,
    // chassis_asset_tag: Option<String>,
    product_name: Option<String>,
    product_serial: Option<String>,
    product_uuid: Option<String>,
}

impl<'a> SMBIOS<'a> for Dmi {
    fn sys_vendor(&'a self) -> Field<'a> {
        match &self.sys_vendor {
            Some(x) => Some(&x),
            None => None,
        }
    }
}

impl Dmi {
    pub fn read(sys_class_dmi_id: &Path) -> Self {
        Self {
            sys_vendor: get_dmi_field(sys_class_dmi_id, Keys::SysVendor),
            product_name: get_dmi_field(sys_class_dmi_id, Keys::ProductName),
            product_uuid: get_dmi_field(sys_class_dmi_id, Keys::ProductUuid),
            product_serial: get_dmi_field(sys_class_dmi_id, Keys::ProductSerial),
        }
    }
}

fn get_dmi_field(sys_class_dmi_id: &Path, key: Keys) -> Option<String> {
    let key = key.get_dmi_field();
    let path = sys_class_dmi_id.join(key);
    if sys_class_dmi_id.is_dir() {
        if path.is_file() {
            return Some(fs::read_to_string(path).unwrap());
        }
        // if `/sys/class/dmi/id` exists, but not the object we're looking for,
        // do *not* fallback to dmidecode!
        return None;
    }
    dmi_decode(key)
}

fn dmi_decode(sys_field: &str) -> Option<String> {
    todo!("command");
}
