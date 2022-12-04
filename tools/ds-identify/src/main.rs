#![allow(non_snake_case)]

use std::io::{BufRead, BufReader};
use std::path::{self, PathBuf};
use std::process::{self, Command};
use std::{
    env,
    ffi::OsStr,
    fs,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

const UNAVAILABLE: &str = "unavailable";

const PATH_RUN: &str = "run";
const PATH_SYS_CLASS_DMI_ID: &str = "sys/class/dmi/id";
const PATH_SYS_HYPERVISOR: &str = "sys/hypervisor";
const PATH_SYS_CLASS_BLOCK: &str = "sys/class/block";
const PATH_DEV_DISK: &str = "dev/disk";
const PATH_VAR_LIB_CLOUD: &str = "var/lib/cloud";
const PATH_DI_CONFIG: &str = "etc/cloud/ds-identify.cfg";
const PATH_PROC_CMDLINE: &str = "proc/cmdline";
const PATH_PROC_1_CMDLINE: &str = "proc/1/cmdline";
const PATH_PROC_1_ENVIRON: &str = "proc/1/environ";
const PATH_PROC_UPTIME: &str = "proc/uptime";
// let PATH_ETC_CLOUD=  get_env_var("PATH_ETC_CLOUD:-${PATH_ROOT}/etc/cloud}";
// let PATH_ETC_CI_CFG=  get_env_var("PATH_ETC_CI_CFG:-${PATH_ETC_CLOUD}/cloud.cfg}";
// let PATH_ETC_CI_CFG_D=  get_env_var("PATH_ETC_CI_CFG_D:-${PATH_ETC_CI_CFG}.d}";
const PATH_RUN_CI: &str = "cloud-init";
// let PATH_RUN_CI_CFG=${PATH_RUN_CI_CFG:-${PATH_RUN_CI}/cloud.cfg};
// let PATH_RUN_DI_RESULT=${PATH_RUN_DI_RESULT:-${PATH_RUN_CI}/.ds-identify.result};

const DI_DSLIST_DEFAULT: &str = "MAAS ConfigDrive NoCloud AltCloud Azure Bigstep \
CloudSigma CloudStack DigitalOcean Vultr AliYun Ec2 GCE OpenNebula OpenStack \
OVF SmartOS Scaleway Hetzner IBMCloud Oracle Exoscale RbxCloud UpCloud VMware \
LXD NWCS";

struct Paths {
    root: PathBuf,

    pub run: PathBuf,
    pub proc_1_environ: PathBuf,
    pub run_ci: PathBuf,
}

impl Paths {
    fn with_root(root: &Path) -> Self {
        Self {
            root: PathBuf::from(root),
            run: Self::compose_paths(root, PATH_RUN),
            proc_1_environ: Self::compose_paths(root, PATH_PROC_1_ENVIRON),
            run_ci: Self::compose_paths(root, PATH_RUN_CI),
        }
    }

    fn compose_paths<P: AsRef<Path>>(root: P, default: &str) -> PathBuf {
        root.as_ref().join(default)
    }

    fn path_from_env<P: AsRef<Path>>(root: P, name: &str, default: &str) -> PathBuf {
        env::var(name).map_or_else(|_| Self::compose_paths(&root, &default), PathBuf::from)
    }
    pub fn from_env() -> Self {
        let root = env::var("PATH_ROOT").unwrap_or_else(|_| String::from("/"));
        let root = Path::new(&root);
        let run = Self::path_from_env(root, "PATH_RUN", &PATH_RUN);
        let proc_1_environ = Self::path_from_env(root, "PATH_PROC_1_ENVIRON", &PATH_PROC_1_ENVIRON);
        let run_ci = Self::path_from_env(root, "PATH_RUN_CI", &PATH_RUN_CI);

        Paths {
            root: PathBuf::from(root),
            run,
            proc_1_environ,
            run_ci,
        }
    }
}

fn error<S: AsRef<str>>(msg: S) {
    let msg = format!("Error: {}", msg.as_ref());
    debug(0, &msg);
    eprintln!("{}", &msg);
}

fn debug<S: AsRef<str>>(level: i32, msg: S) {
    // TODO: Find a way to not recompute this value in every call
    let DI_DEBUG_LEVEL: i32 = get_env_var("DEBUG_LEVEL", String::from("-1"))
        .parse()
        .unwrap();
    if level >= DI_DEBUG_LEVEL {
        // XXX: enable
        // return;
    }
    static _DI_LOGGED: AtomicBool = AtomicBool::new(false);
    if !_DI_LOGGED.load(Ordering::Relaxed) {
        // first time here, open file descriptor for append
        // TODO: log to file
        _DI_LOGGED.store(true, Ordering::Release);
    }
    eprintln!("{}", msg.as_ref());
}

fn get_env_var<K: AsRef<OsStr>>(key: K, default: String) -> String {
    env::var(key).unwrap_or_else(|_| default)
}

struct Info {
    uname_info: UnameInfo,
    virt: String,
    pid1_prod_name: String,
    kernel_cmdline: (),
    config: Config,
}

impl Info {
    fn collect_info(paths: &Paths) -> Self {
        let uname_info = UnameInfo::read();
        let virt = read_virt(&uname_info);
        let pid1_prod_name = read_pid1_product_name(&paths.proc_1_environ);
        let kernel_cmdline = read_kernel_cmdline();
        let config = Config::read();
        // read_datasource_list
        // read_dmi_sys_vendor
        // read_dmi_board_name
        // read_dmi_chassis_asset_tag
        // read_dmi_product_name
        // read_dmi_product_serial
        // read_dmi_product_uuid
        // read_fs_info

        Self {
            uname_info,
            virt,
            pid1_prod_name,
            kernel_cmdline,
            config,
        }
    }

    fn to_old_str(&self) -> String {
        let mut string = String::new();
        // TODO: DMI_PRODUCT_NAME
        // TODO: DMI_SYS_VENDOR
        // TODO: DMI_PRODUCT_SERIAL
        // TODO: DMI_PRODUCT_UUID
        // TODO: PID_1_PRODUCT_NAME
        // TODO: DMI_CHASSIS_ASSET_TAG
        // TODO: DMI_BOARD_NAME
        // TODO: FS_LABELS
        // TODO: ISO9660_DEVS
        // TODO: KERNEL_CMDLINE VIRT
        // TODO: UNAME_KERNEL_NAME
        string.push_str(&format!(
            "UNAME_KERNEL_NAME={}",
            self.uname_info.kernel_name
        ));
        // TODO: UNAME_KERNEL_RELEASE
        // TODO: UNAME_KERNEL_VERSION
        // TODO: UNAME_MACHINE UNAME_NODENAME
        // TODO: UNAME_OPERATING_SYSTEM
        // TODO: DSNAME
        // TODO: DSLIST
        // TODO: MODE
        // TODO: ON_FOUND
        // TODO: ON_MAYBE
        // TODO: ON_NOTFOUND

        string
    }
}

fn read_kernel_cmdline() {
    todo!();
}

struct UnameInfo {
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

fn is_systemd() -> bool {
    path::Path::new("/run/systemd").is_dir()
}

fn detect_virt(uname_info: &UnameInfo) -> String {
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
    virt
}

fn read_virt(uname_info: &UnameInfo) -> String {
    detect_virt(&uname_info)
}
fn read_cmdline() {}

enum Mode {
    Report,
    Search,
}

struct Config {
    mode: Mode,
    on_found: (),
    on_maybe: (),
    on_notfound: (),
    dsname: (),
}

impl Config {
    pub fn read() -> Self {
        todo!();
    }
}
fn read_uptime<P: AsRef<Path>>(path: P) -> String {
    let res = String::from(UNAVAILABLE);
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(..) => return res,
    };
    let mut buffered = BufReader::new(file);

    let mut buf = String::new();
    match buffered.read_line(&mut buf) {
        Ok(_) => (),
        Err(_) => return res,
    };
    buf.split(' ').take(1).collect()
}

fn _main() {
    // TODO: ensure_sane_path

    let args: Vec<String> = env::args().collect();
    let args_str: &str = &args[1..].join(" ");

    let paths = Paths::from_env();

    let DI_LOG = paths.run_ci.join("ds-identify.log");

    debug(
        1,
        format!(
            "[up {}s] ds-identify {args_str}",
            read_uptime(PATH_PROC_UPTIME)
        ),
    );

    let info = Info::collect_info(&paths);

    if DI_LOG.to_str().unwrap() == "stderr" {
        todo!();
    } else {
        todo!("print to `DI_LOG`");
        let old_cli_str = info.to_old_str();
        println!("{old_cli_str}");
    }
}

fn main() {
    let DI_MAIN = get_env_var("DI_MAIN", String::from("main"));
    match &DI_MAIN[..] {
        "main" | "print_info" | "noop" => _main(),
        _ => {
            error("unexpected value for DI_MAIN");
            process::exit(1);
        }
    }
}
