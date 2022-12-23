use std::fmt::Display;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;
use std::{env, fs, path};

use crate::constants::{DI_DISABLED, DI_DSLIST_DEFAULT, DI_ENABLED, UNAVAILABLE};
use crate::paths::Paths;
use crate::smbios::SMBIOS;
use crate::util::{debug, error, parse_yaml_array, unquote};

pub struct Info {
    uname_info: UnameInfo,
    virt: Virt,
    pid1_prod_name: String,
    kernel_cmdline: String,
    config: Config,
    dslist: DatasourceList,
    smbios: SMBIOS,
    fs_info: FSInfo,
}

impl Info {
    pub fn collect_info(paths: &Paths) -> Self {
        let uname_info = UnameInfo::read();
        let virt = Virt::from(&uname_info);
        let is_container = virt.is_container();
        let pid1_prod_name = Self::read_pid1_product_name(&paths.proc_1_environ);
        let kernel_cmdline = Self::read_kernel_cmdline(&paths, is_container);
        let config = Config::read(&paths, &kernel_cmdline, &uname_info);
        let dslist = DatasourceList::read(&paths);
        let smbios = SMBIOS::from_kernel_name(uname_info.kernel_name.as_str(), &paths);
        let fs_info = FSInfo::read_linux(&is_container);

        Self {
            uname_info,
            virt,
            pid1_prod_name,
            kernel_cmdline,
            config,
            dslist,
            smbios,
            fs_info,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
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
        string.push_str(&format!("DSNAME={:?}\n", self.config.dsname));
        string.push_str(&format!("DSLIST={:?}\n", self.dslist));
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
            if cmdline.len() > 0 {
                return cmdline;
            }
            return format!("{UNAVAILABLE}:container");
        } else if paths.proc_cmdline.is_file() {
            return fs::read_to_string(&paths.proc_cmdline).unwrap();
        } else {
            return format!("{UNAVAILABLE}:no-cmdline");
        };
    }
}

#[derive(Debug)]
pub struct UnameInfo {
    kernel_name: String,
    node_name: String,
    kernel_release: String,
    kernel_version: String,
    machine: String,
    operating_system: String,
    cmd_out: String,
}

impl UnameInfo {
    pub fn read() -> Self {
        // run uname, and parse output.
        // uname is tricky to parse as it outputs always in a given order
        // independent of option order. kernel-version is known to have spaces.
        // 1   -s kernel-name
        // 2   -n nodename
        // 3   -r kernel-release
        // 4.. -v kernel-version(whitespace)
        // N-2 -m machine
        // N-1 -o operating-system
        static ERR_MSG: &str = "failed reading uname with 'uname -snrvmo'";

        let output = Command::new("uname")
            .arg("-snrvmo")
            .output()
            .expect(ERR_MSG);
        let out = String::from_utf8(output.stdout).expect(ERR_MSG);

        let mut out_words = out.split(' ');

        let kernel_name = out_words.next().unwrap().to_string();
        let node_name = out_words.next().unwrap().to_string();
        let kernel_release = out_words.next().unwrap().to_string();
        let operating_system = out_words.next_back().unwrap().to_string();
        let machine = out_words.next_back().unwrap().to_string();
        let kernel_version = out_words.collect::<Vec<_>>().join(" ");

        UnameInfo {
            kernel_name,
            node_name,
            kernel_release,
            kernel_version,
            machine,
            operating_system,
            cmd_out: out,
        }
    }
}

#[derive(Debug)]
pub struct Virt(String);

impl Virt {
    fn from(uname_info: &UnameInfo) -> Self {
        let mut virt = String::from(UNAVAILABLE);
        if is_systemd() {
            let output = Command::new("systemd-detect-virt").output();
            if let Ok(output) = output {
                if output.status.success() {
                    virt = String::from_utf8(output.stdout).unwrap();
                } else {
                    if output.stdout == b"none" || output.stderr == b"none" {
                        virt = String::from("none");
                    }
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
        match &self.0.to_lowercase()[..] {
            "container-other" | "lxc" | "lxc-libvirt" | "systemd-nspawn" | "docker" | "rkt"
            | "jail" => true,
            _ => false,
        }
    }
}

fn is_systemd() -> bool {
    path::Path::new("/run/systemd").is_dir()
}

#[derive(Debug)]
pub enum Mode {
    Disabled,
    Enabled,
    Search,
    Report,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cli = match self {
            Self::Disabled => DI_DISABLED,
            Self::Enabled => DI_ENABLED,
            Self::Search => "search",
            Self::Report => "report",
        };
        write!(f, "{cli}")
    }
}

#[derive(Debug, Default)]
enum Found {
    /// use the first found do no further checking
    First,
    /// enable all DS_FOUND
    #[default]
    All,
}

impl Found {
    fn cli_repr(&self) -> String {
        match self {
            Self::First => "first".to_owned(),
            Self::All => "all".to_owned(),
        }
    }
}

#[derive(Debug, Default)]
enum Maybe {
    /// enable all DS_MAYBE
    #[default]
    All,
    /// ignore any DS_MAYBE
    None,
}

impl Maybe {
    fn cli_repr(&self) -> String {
        match self {
            Self::None => "none".to_owned(),
            Self::All => "all".to_owned(),
        }
    }
}

#[derive(Debug, Default)]
enum NotFound {
    /// disable cloud-init
    #[default]
    Disabled,
    /// enable cloud-init
    Enabled,
}

impl NotFound {
    fn cli_repr(&self) -> String {
        match self {
            Self::Disabled => "disable".to_owned(),
            Self::Enabled => "enable".to_owned(),
        }
    }
}

// TODO: test fixing default modes
#[derive(Debug)]
struct Policy {
    mode: Mode,
    on_found: Found,
    on_maybe: Maybe,
    on_notfound: NotFound,
    report: bool,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            mode: Mode::Search,
            on_found: Found::default(),
            on_maybe: Maybe::default(),
            on_notfound: NotFound::default(),
            report: false,
        }
    }
}

impl Policy {
    fn default_no_dmi() -> Self {
        Self {
            mode: Mode::Search,
            on_found: Found::All,
            on_maybe: Maybe::All,
            on_notfound: NotFound::Enabled,
            ..Default::default()
        }
    }

    // XXX: impl From trait
    fn parse_from_uname(uname: &UnameInfo) -> Self {
        match &uname.machine[..] {
            // these have dmi data
            "i686" | "i386" | "x86_64" => Policy::default(),
            // aarch64 has dmi, but not currently used (LP: #1663304)
            "aarch64" | _ => Policy::default_no_dmi(),
        }
    }

    fn parse_from_str(policy_str: &str, uname: &UnameInfo) -> Self {
        let mut policy = Policy::parse_from_uname(&uname);

        let mut mode = None;
        let mut found = None;
        let mut maybe = None;
        let mut notfound = None;
        for tok in policy_str.trim().split(",") {
            match tok.split_once('=') {
                Some(("found", val)) => match val {
                    "all" => found = Some(Found::All),
                    "first" => found = Some(Found::First),
                    val => Self::parse_warn("found", val, &policy.on_found.cli_repr()),
                },
                Some(("maybe", val)) => match val {
                    "all" => maybe = Some(Maybe::All),
                    "none" => maybe = Some(Maybe::None),
                    val => Self::parse_warn("maybe", val, &policy.on_maybe.cli_repr()),
                },
                Some(("notfound", val)) => match val {
                    DI_DISABLED => notfound = Some(NotFound::Disabled),
                    DI_ENABLED => notfound = Some(NotFound::Enabled),
                    val => Self::parse_warn("notfound", val, &policy.on_notfound.cli_repr()),
                },
                Some(_) => continue, // backward compat
                None => match tok {
                    DI_ENABLED => mode = Some(Mode::Enabled),
                    DI_DISABLED => mode = Some(Mode::Disabled),
                    "search" => mode = Some(Mode::Search),
                    "report" => mode = Some(Mode::Report),
                    _ => continue, // backward compat
                },
            }
        }

        if let Some(x) = mode {
            policy.mode = x;
        };
        if let Some(x) = found {
            policy.on_found = x;
        };
        if let Some(x) = maybe {
            policy.on_maybe = x;
        };
        if let Some(x) = notfound {
            policy.on_notfound = x;
        };

        policy
    }

    fn parse_warn(key: &str, invalid: &str, valid: &str) {
        eprintln!("WARN: invalid value '{invalid}' for key '{key}'. Using {key}={valid}");
    }
}

pub struct Config {
    dsname: Option<String>,
    mode: Mode,
    on_found: Found,
    on_maybe: Maybe,
    on_notfound: NotFound,
}

impl Config {
    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    pub fn dsname(&self) -> Option<&str> {
        match &self.dsname {
            None => None,
            Some(dsname) => Some(dsname),
        }
    }

    fn from_file(path: &Path) -> (Option<String>, Option<String>) {
        // TODO: input with explicit keyname
        if !path.is_file() {
            panic!("{path:?} exists but is not a file!");
            // TODO: exit_code 1
        }
        let mut dsname = None;
        let mut policy = None;
        for line in fs::read_to_string(path).unwrap().lines() {
            let (key, val) = match line.split_once(':') {
                None => continue, // no `:` in the line.
                Some((key, val)) => {
                    let key = key.trim();
                    let val = unquote(val.trim());
                    (key, val)
                }
            };
            match key {
                "datasource" => dsname = Some(val.to_string()),
                "policy" => policy = Some(val.to_string()),
                _ => (),
            };
        }

        (dsname, policy)
    }

    pub fn read(paths: &Paths, kernel_cmdline: &str, uname: &UnameInfo) -> Self {
        let mut dsname = None;
        let mut policy = None;
        if paths.di_config.exists() {
            (dsname, policy) = Self::from_file(&paths.di_config);
        };

        for tok in kernel_cmdline.split(' ') {
            match tok.split_once('=') {
                None => continue,
                Some((key, val)) => match key {
                    "ci.ds" | "ci.datasource" => dsname = Some(val.to_string()),
                    "ci.di.policy" => policy = Some(val.to_string()),
                    _ => continue,
                },
            }
        }

        let policy = match policy {
            Some(p) => Policy::parse_from_str(&p, uname),
            None => Policy::parse_from_uname(uname),
        };

        // TODO: `debug` policy
        dbg!(&policy);

        Self {
            dsname,
            mode: policy.mode,
            on_found: policy.on_found,
            on_maybe: policy.on_maybe,
            on_notfound: policy.on_notfound,
        }
    }
}

/// somewhat hackily read through paths for `key`
///
/// currently does not respect any hierarchy in searching for key.
fn check_config<'a, P: AsRef<Path>>(key: &str, paths: &'a [P]) -> Option<(String, &'a Path)> {
    let mut value_path = None;

    for f in paths.iter().filter(|p| p.as_ref().is_file()) {
        let stream = BufReader::new(File::open(f).unwrap());
        for line in stream.lines() {
            let line = line.unwrap();

            // remove trailing comments or full line comments
            let line = match line.split_once('#') {
                Some((line, _)) => line,
                None => &line,
            }
            .trim();

            if let Some((k, v)) = line.split_once(':') {
                if key == k.trim() {
                    value_path = Some((v.trim().to_owned(), f.as_ref()));
                }
            };
        }
    }
    value_path
}

// XXX: refactor Strings -> enums
#[derive(Debug)]
struct DatasourceList(Vec<String>);

impl DatasourceList {
    fn read(paths: &Paths) -> Self {
        let mut dslist = None;

        if let Ok(dsname) = env::var("DI_DSNAME") {
            dslist = Some(dsname);
        };

        // TODO: kernel cmdline
        // LP: #1582323. cc:{'datasource_list': ['name']}
        // more generically cc:<yaml>[end_cc]

        // if DI_DSNAME is set as an envvar or DS_LIST is in the kernel cmdline,
        // then avoid parsing config.
        if let Some(dslist) = dslist {
            return Self::from(&dslist[..]);
        };

        let cfg_paths = paths.etc_ci_cfg_paths();
        if let Some((found_dslist, path)) = check_config("datasource_list", &cfg_paths[..]) {
            debug(
                1,
                format!("{:?} set datasource_list: {}", path, found_dslist),
            );
            let dslist = parse_yaml_array(&found_dslist);
            let dslist = dslist.iter().map(|x| x.to_string()).collect();
            return Self(dslist);
        };

        DatasourceList::default()
    }

    fn found() {
        todo!();
    }
}

impl Default for DatasourceList {
    fn default() -> Self {
        Self(DI_DSLIST_DEFAULT.split(' ').map(str::to_string).collect())
    }
}

impl From<&str> for DatasourceList {
    fn from(value: &str) -> Self {
        Self(value.split_whitespace().map(|s| s.to_owned()).collect())
    }
}

#[derive(Debug)]
pub struct FSInfo {
    fs_labels: String,
    iso9660_devs: String,
    fs_uuids: Option<String>,
}

impl FSInfo {
    pub fn read_linux(is_container: &bool) -> Self {
        // do not rely on links in /dev/disk which might not be present yet.
        // Note that blkid < 2.22 (centos6, trusty) do not output DEVNAME.
        // that means that DI_ISO9660_DEVS will not be set.
        if *is_container {
            let unavailable_container = format!("{}:container", UNAVAILABLE);
            // blkid will in a container, or at least currently in lxd
            // not provide useful information.
            return Self {
                fs_labels: unavailable_container.clone(),
                iso9660_devs: unavailable_container.clone(),
                fs_uuids: None,
            };
        };

        let blkid_export_out = Self::blkid_export();
        match blkid_export_out {
            None => {
                let unavailable_error = format!("{}:error", UNAVAILABLE);
                Self {
                    fs_labels: unavailable_error.clone(),
                    iso9660_devs: unavailable_error.clone(),
                    fs_uuids: Some(unavailable_error.clone()),
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
                    fs_uuids: Some(uuids),
                    iso9660_devs: isodevs,
                }
            }
        }
    }

    fn blkid_export() -> Option<String> {
        let output = Command::new("blkid")
            .args(["-c /dev/null -o export"])
            .output()
            .expect("failed to execute blkid");
        if !output.status.success() {
            let ret = output
                .status
                .code()
                .map_or("?".to_string(), |c| c.to_string());
            error(&format!(
                "failed running [{}]: blokid -c /dev/null -o export",
                ret
            ));
            None
        } else {
            Some(String::from_utf8(output.stdout).expect("valid utf8 output"))
        }
    }
}
