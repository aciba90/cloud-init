use crate::info::Info;
use std::{fs, os::unix::fs::FileTypeExt};

#[derive(Debug, Clone)]
pub enum Datasource {
    None,
    NoCloud,
    LXD,
    Unknown(String),
}

#[derive(Debug)]
pub enum DscheckResult {
    Found(Option<String>),
    NotFound,
    Maybe(Option<String>),
}

impl Datasource {
    pub fn dscheck_fn(&self) -> fn(&Info) -> DscheckResult {
        match &self {
            Self::None => dscheck_none,
            Self::NoCloud => dscheck_no_cloud,
            Self::LXD => dscheck_lxd,
            _ => todo!(),
        }
    }
}

// TODO: old_str
impl From<&str> for Datasource {
    fn from(val: &str) -> Self {
        match &val.to_lowercase()[..] {
            "nocloud" => Self::NoCloud,
            "lxd" => Self::LXD,
            _ => Self::Unknown(val.to_string()),
        }
    }
}

impl From<&Datasource> for String {
    fn from(ds: &Datasource) -> Self {
        match ds {
            Datasource::NoCloud => "NoCloud".to_string(),
            Datasource::None => "None".to_string(),
            Datasource::LXD => "LXD".to_string(),
            Datasource::Unknown(ds) => format!("Unknown({})", ds),
        }
    }
}

fn dscheck_none(_info: &Info) -> DscheckResult {
    DscheckResult::NotFound
}

fn dscheck_no_cloud(info: &Info) -> DscheckResult {
    let fs_label = "cidata CIDATA";
    const DS_NOCLOUD: &str = "ds=nocloud";

    if info.kernel_cmdline().contains(DS_NOCLOUD) {
        return DscheckResult::Found(None);
    }

    if let Some(produc_serial) = &info.smbios().product_serial {
        if produc_serial.contains(DS_NOCLOUD) {
            return DscheckResult::Found(None);
        }
    }

    // todo!();
    DscheckResult::NotFound
}

/// LXD datasource requires active /dev/lxd/sock
/// https://linuxcontainers.org/lxd/docs/master/dev-lxd
fn dscheck_lxd(info: &Info) -> DscheckResult {
    if let Ok(meta) = fs::metadata("/dev/lxd/sock") {
        if meta.file_type().is_socket() {
            return DscheckResult::Found(None);
        }
    }

    // On LXD KVM instances, /dev/lxd/sock is not yet setup by
    // lxd-agent-loader's systemd lxd-agent.service.
    // Rely on DMI product information that is present on all LXD images.
    // Note "qemu" is returned on kvm instances launched from a host kernel
    // kernels >=5.10, due to `hv_passthrough` option.
    // systemd v. 251 should properly return "kvm" in this scenario
    // https://github.com/systemd/systemd/issues/22709
    if info.virt() == "kvm" || info.virt() == "qemu" {
        if let Some(board_name) = &info.smbios().board_name {
            if board_name == "LXD" {
                return DscheckResult::Found(None);
            }
        }
    }
    DscheckResult::NotFound
}

fn dscheck_cloud_stack(_info: &Info) -> DscheckResult {
    todo!();
}

mod util {
    use crate::paths::Paths;

    /// check the seed dir /var/lib/cloud/seed/<name> for 'required'
    /// required defaults to 'meta-data'
    pub fn check_seed_dir(paths: &Paths, name: &str, required: Option<&[&str]>) -> bool {
        let dir = paths.var_lib_cloud.join("seed").join(name);
        if !dir.is_dir() {
            return false;
        }
        let required = required.unwrap_or(&["meta-data"]);
        for f in required {
            if !dir.join(f).is_file() {
                return false;
            }
        }
        true
    }

    pub fn check_writable_seed_dir(paths: &Paths) -> bool {
        // ubuntu core bind-mounts /writable/system-data/var/lib/cloud
        // over the top of /var/lib/cloud, but the mount might not be done yet.
        const WDIR: &str = "writable/system-data";
        if !paths.root.join(WDIR).is_dir() {
            return false;
        }

        // TODO

        true
    }
}
