use std::{path::Path, fs::{File, self}, io::{ErrorKind, BufReader}};

type Field<'a> = Option<&'a str>;

pub trait SMBIOS<'a> {
    fn sys_vendor(&'a self) -> Field<'a>;
}

pub struct Dmi {
    sys_vendor: Option<String>,
    // board_name,
    // chassis_asset_tag,
    // product_name,
    // product_serial,
    // product_uuid,
}

impl<'a> SMBIOS<'a> for Dmi {
    fn sys_vendor(&'a self) -> Field<'a> {
        match &self.sys_vendor {
            Some(x) => Some(&x),
            None => None
        }
    }
}

impl<'a> Dmi {
    pub fn read(sys_class_dmi_id: &Path) -> Self{
        Self{
            sys_vendor: get_dmi_field(sys_class_dmi_id, "sys_vendor"),
        }
    }
}

fn get_dmi_field<'a>(sys_class_dmi_id: &Path, key: &str) -> Option<String> {
    let path = sys_class_dmi_id.join(key);
    if sys_class_dmi_id.is_dir() {
        if path.is_file() {
            return Some(fs::read_to_string(path).unwrap())
        }
        // if `/sys/class/dmi/id` exists, but not the object we're looking for,
        // do *not* fallback to dmidecode!
        return None;
    }

    None
}

fn dmi_decode(key: &str) {
    todo!("command");
}
