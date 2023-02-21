use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;
use std::{fs, path};

use crate::constants::UNAVAILABLE;
use crate::paths::Paths;
use crate::sources::DatasourceList;
use crate::util::Logger;
use smbios::SmBios;
use uname::UnameInfo;

use self::config::Config;
pub use self::config::{Found, Maybe, Mode, NotFound};

mod config;
mod smbios;
mod uname;

pub struct Info<'a> {
    paths: Paths,
    uname_info: UnameInfo,
    virt: Virt,
    pid1_prod_name: String,
    kernel_cmdline: String,
    config: Config,
    dslist: DatasourceList,
    smbios: SmBios,
    fs_info: FSInfo,
    logger: &'a Logger,
}

impl<'a> Info<'a> {
    pub fn collect_info(logger: &'a Logger, paths: &Paths) -> Self {
        let uname_info = UnameInfo::read();
        let virt = Virt::from(&uname_info);
        let is_container = virt.is_container();
        let pid1_prod_name = Self::read_pid1_product_name(&paths.proc_1_environ);
        let kernel_cmdline = Self::read_kernel_cmdline(paths, is_container);
        let config = Config::read(paths, &kernel_cmdline, &uname_info);
        let dslist = DatasourceList::read(logger, paths);
        let smbios = SmBios::from_kernel_name(uname_info.kernel_name.as_str(), paths);
        let fs_info = FSInfo::read_linux(logger, &is_container);

        Self {
            paths: paths.clone(),
            uname_info,
            virt,
            pid1_prod_name,
            kernel_cmdline,
            config,
            dslist,
            smbios,
            fs_info,
            logger,
        }
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn virt(&self) -> &str {
        &self.virt.0
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn dslist(&self) -> &DatasourceList {
        &self.dslist
    }

    pub fn kernel_cmdline(&self) -> &str {
        &self.kernel_cmdline
    }

    pub fn smbios(&self) -> &SmBios {
        &self.smbios
    }

    pub fn fs_info(&self) -> &FSInfo {
        &self.fs_info
    }

    pub fn logger(&self) -> &Logger {
        self.logger
    }

    pub fn to_old_str(&self) -> String {
        let mut string = String::new();
        string.push_str(&format!(
            "DMI_PRODUCT_NAME={:?}\n",
            self.smbios.product_name
        ));
        string.push_str(&format!("DMI_SYS_VENDOR={:?}\n", self.smbios.sys_vendor));
        string.push_str(&format!(
            "DMI_PRODUCT_SERIAL={:?}\n",
            self.smbios.product_serial
        ));
        string.push_str(&format!(
            "DMI_PRODUCT_UUID={:?}\n",
            self.smbios.product_uuid
        ));
        string.push_str(&format!("PID_1_PRODUCT_NAME={}\n", self.pid1_prod_name));
        string.push_str(&format!(
            "DMI_CHASSIS_ASSET_TAG={:?}\n",
            self.smbios.chassis_asset_tag
        ));
        string.push_str(&format!("DMI_BOARD_NAME={:?}\n", self.smbios.board_name));
        string.push_str(&format!("FS_LABELS={:?}\n", self.fs_info.fs_labels));
        string.push_str(&format!("ISO9660_DEVS={:?}\n", self.fs_info.iso9660_devs));
        string.push_str(&format!("KERNEL_CMD_LINE={}\n", self.kernel_cmdline));
        string.push_str(&format!("VIRT={:?}\n", self.virt));
        string.push_str(&format!(
            "UNAME_KERNEL_NAME={}",
            self.uname_info.kernel_name
        ));
        string.push_str(&format!(
            "UNAME_KERNEL_RELEASE={}\n",
            self.uname_info.kernel_release
        ));
        string.push_str(&format!(
            "UNAME_KERNEL_VERSION={}\n",
            self.uname_info.kernel_version
        ));
        string.push_str(&format!("UNAME_MACHINE={}\n", self.uname_info.machine));
        string.push_str(&format!("UNAME_NODENAME={}\n", self.uname_info.node_name));
        string.push_str(&format!(
            "UNAME_OPERATING_SYSTEM={}\n",
            self.uname_info.operating_system
        ));
        string.push_str(&format!("DSNAME={:?}\n", self.config.dsname()));
        string.push_str(&format!("DSLIST={}\n", self.dslist.to_old_str()));
        string.push_str(&format!("MODE={}\n", self.config.mode));
        string.push_str(&format!("ON_FOUND={:?}\n", self.config.on_found));
        string.push_str(&format!("ON_MAYBE={:?}\n", self.config.on_maybe));
        string.push_str(&format!("ON_NOTFOUND={:?}\n", self.config.on_notfound));

        // TODO: pid, ppid, is_container

        string
    }

    fn read_pid1_product_name<T: AsRef<Path>>(proc_pid_1_environ: T) -> String {
        let product_name = String::from(UNAVAILABLE);
        let environ = fs::File::open(proc_pid_1_environ.as_ref()).unwrap();
        let buffered = BufReader::new(environ);

        // /proc/x/environ contain lines null terminated
        for line in buffered
            .split(b'\0')
            .map(|w| String::from_utf8(w.unwrap()).unwrap())
        {
            let (key, value) = line.split_once('=').unwrap();
            if key.to_lowercase() == "product_name" {
                return value.to_string();
            }
        }

        product_name
    }

    fn read_kernel_cmdline(paths: &Paths, is_container: bool) -> String {
        if is_container {
            let cmdline = fs::read_to_string(&paths.proc_1_cmdline).unwrap();
            let cmdline = cmdline.replace('\0', " ");
            if cmdline.is_empty() {
                return cmdline;
            }
            format!("{UNAVAILABLE}:container")
        } else if paths.proc_cmdline.is_file() {
            fs::read_to_string(&paths.proc_cmdline).unwrap()
        } else {
            format!("{UNAVAILABLE}:no-cmdline")
        }
    }
}

#[derive(Debug)]
pub struct Virt(String);

impl Virt {
    fn from(uname_info: &UnameInfo) -> Self {
        dbg!(&uname_info);
        let mut virt = String::from(UNAVAILABLE);
        if is_systemd() {
            let output = Command::new("systemd-detect-virt").output();
            if let Ok(output) = output {
                if output.status.success() {
                    virt = String::from_utf8(output.stdout).unwrap();
                    let n_to_remove = virt.trim_end().len();
                    virt.truncate(n_to_remove);
                } else if output.stdout == b"none" || output.stderr == b"none" {
                    virt = String::from("none");
                }
            }
        } else if uname_info.kernel_name == "FreeBSD" {
            // Map FreeBSD's vm_guest names to those systemd-detect-virt that
            // don't match up. See
            // https://github.com/freebsd/freebsd/blob/master/sys/kern/subr_param.c#L144-L160
            // https://www.freedesktop.org/software/systemd/man/systemd-detect-virt.html
            //
            //  systemd    | kern.vm_guest
            // ---------------------+---------------
            //  none       | none
            //  kvm        | kvm
            //  vmware     | vmware
            //  microsoft  | hv
            //  oracle     | vbox
            //  xen        | xen
            //  parallels  | parallels
            //  bhyve      | bhyve
            //  vm-other   | generic
            if let Ok(output) = Command::new("sysctl")
                .arg("-qn")
                .arg("kern.vm_guest")
                .output()
            {
                if let Ok(out) = String::from_utf8(output.stdout) {
                    match &out[..] {
                        "hv" => virt = String::from("microsoft"),
                        "vbox" => virt = String::from("oracle"),
                        "generic" => virt = String::from("vm-other"),
                        _ => virt = out,
                    }
                }
            }
            if let Ok(output) = Command::new("sysctl")
                .arg("-qn")
                .arg("security.jail.jailed")
                .output()
            {
                if let Ok(out) = String::from_utf8(output.stdout) {
                    if &out[..] == "1" {
                        virt = String::from("jail");
                    }
                }
            }
        }
        Self(virt)
    }

    fn is_container(&self) -> bool {
        matches!(
            &self.0.to_lowercase()[..],
            "container-other"
                | "lxc"
                | "lxc-libvirt"
                | "systemd-nspawn"
                | "docker"
                | "rkt"
                | "jail"
        )
    }
}

fn is_systemd() -> bool {
    path::Path::new("/run/systemd").is_dir()
}

#[derive(Debug)]
pub struct FSInfo {
    fs_labels: String,
    iso9660_devs: String,
    _fs_uuids: Option<String>,
}

impl FSInfo {
    pub fn read_linux(logger: &Logger, is_container: &bool) -> Self {
        // do not rely on links in /dev/disk which might not be present yet.
        // Note that blkid < 2.22 (centos6, trusty) do not output DEVNAME.
        // that means that DI_ISO9660_DEVS will not be set.
        if *is_container {
            let unavailable_container = format!("{}:container", UNAVAILABLE);
            // blkid will in a container, or at least currently in lxd
            // not provide useful information.
            return Self {
                fs_labels: unavailable_container.clone(),
                iso9660_devs: unavailable_container,
                _fs_uuids: None,
            };
        };

        let blkid_export_out = Self::blkid_export(logger);
        match blkid_export_out {
            None => {
                let unavailable_error = format!("{}:error", UNAVAILABLE);
                Self {
                    fs_labels: unavailable_error.clone(),
                    iso9660_devs: unavailable_error.clone(),
                    _fs_uuids: Some(unavailable_error),
                }
            }
            Some(blkid_export_out) => {
                let delim = ',';

                let mut labels = String::new();
                let mut uuids = String::new();
                let mut isodevs = String::new();
                let mut ftype = None;
                let mut dev = None;
                let mut label = None;
                for line in blkid_export_out.lines() {
                    dbg!(&line);
                    if let Some((_, value)) = line.split_once("DEVNAME=") {
                        if let Some(dev_prev) = dev {
                            if matches!(ftype, Some("iso9660")) {
                                isodevs.push_str(&format!("{}={}", dev_prev, label.unwrap_or("")));
                                isodevs.push(delim);
                            }
                            ftype = None;
                            label = None;
                            dev = Some(value);
                        }
                    } else if line.starts_with("LABEL=") || line.starts_with("LABEL_FATBOOT=") {
                        let value = match line.split_once("LABEL=") {
                            Some((_, value)) => value,
                            None => match line.split_once("LABEL_FATBOOT=") {
                                None => panic!("One should match!"),
                                Some((_, value)) => value,
                            },
                        };
                        labels.push_str(value);
                        labels.push(delim);
                    } else if let Some((_, value)) = line.split_once("TYPE=") {
                        ftype = Some(value);
                    } else if let Some((_, value)) = line.split_once("UUID=") {
                        uuids.push_str(value);
                        uuids.push(delim);
                    }
                }

                if let Some(dev_prev) = dev {
                    if matches!(ftype, Some("iso9660")) {
                        isodevs.push_str(&format!("{}={}", dev_prev, label.unwrap_or("")));
                        isodevs.push(delim);
                    }
                }

                Self {
                    fs_labels: labels,
                    _fs_uuids: Some(uuids),
                    iso9660_devs: isodevs,
                }
            }
        }
    }

    fn blkid_export(logger: &Logger) -> Option<String> {
        let output = Command::new("blkid")
            .args(["-c /dev/null -o export"])
            .output()
            .expect("failed to execute blkid");
        if !output.status.success() {
            let ret = output
                .status
                .code()
                .map_or("?".to_string(), |c| c.to_string());
            logger.error(&format!(
                "failed running [{}]: blkid -c /dev/null -o export",
                ret
            ));
            None
        } else {
            Some(String::from_utf8(output.stdout).expect("valid utf8 output"))
        }
    }

    /// Return true if there is a filesystem that matches any of the labels.
    pub fn has_fs_with_label(&self, labels: &[&str]) -> bool {
        for label in labels {
            if self.fs_labels.contains(&format!(",{},", label)) {
                return true;
            }
        }
        false
    }
}
