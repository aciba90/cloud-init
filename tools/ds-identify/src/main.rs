#![allow(non_snake_case)]

use std::io::{BufRead, BufReader, Error, Write};
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

fn collect_info(paths: &Paths) {
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
    for line in buffered.split(b'\0').map(|w| String::from_utf8(w)) {
        let (key, value) = line.split_once('=');
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
    pub fn read() {
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

struct Paths {
    root: PathBuf,

    pub run: PathBuf,
    pub proc_1_environ: PathBuf,
}

impl Paths {
    fn compose_default<T: AsRef<str>, U: AsRef<str>>(root: T, default: U) -> PathBuf {
        PathBuf::from(format!("{}{}", root.as_ref(), default.as_ref()))
    }

    pub fn new() -> Self {
        let root = env::var("PATH_ROOT").unwrap_or_else(|_| String::from(""));
        let run = env::var("PATH_RUN")
            .map_or_else(|_| Self::compose_default(&root, "/run"), PathBuf::from);
        let proc_1_environ = env::var("PATH_PROC_1_ENVIRON").map_or_else(
            |_| Self::compose_default(&root, "/proc/1/environ"),
            PathBuf::from,
        );

        Paths {
            root: PathBuf::from(root),
            run,
            proc_1_environ,
        }
    }
}

fn _main() {
    // TODO: ensure_sane_path

    let args: Vec<String> = env::args().collect();
    let args_str: &str = &args[1..].join(" ");

    let paths = Paths::new();
    let PATH_ROOT = env::var("PATH_ROOT").unwrap_or_else(|_| String::from(""));
    let PATH_RUN = get_env_var("PATH_RUN", format!("{PATH_ROOT}/run"));
    let PATH_SYS_CLASS_DMI_ID = get_env_var(
        "PATH_SYS_CLASS_DMI_ID",
        format!("{PATH_ROOT}/sys/class/dmi/id"),
    );
    let PATH_SYS_HYPERVISOR =
        get_env_var("PATH_SYS_HYPERVISOR", format!("{PATH_ROOT}/sys/hypervisor"));
    let PATH_SYS_CLASS_BLOCK = get_env_var(
        "PATH_SYS_CLASS_BLOCK",
        format!("{PATH_ROOT}/sys/class/block"),
    );
    let PATH_DEV_DISK = get_env_var("PATH_DEV_DISK", format!("{PATH_ROOT}/dev/disk"));
    let PATH_VAR_LIB_CLOUD =
        get_env_var("PATH_VAR_LIB_CLOUD", format!("{PATH_ROOT}/var/lib/cloud"));
    let PATH_DI_CONFIG = get_env_var(
        "PATH_DI_CONFIG",
        format!("{PATH_ROOT}/etc/cloud/ds-identify.cfg"),
    );
    let PATH_PROC_CMDLINE = get_env_var("PATH_PROC_CMDLINE", format!("{PATH_ROOT}/proc/cmdline"));
    let PATH_PROC_1_CMDLINE =
        get_env_var("PATH_PROC_1_CMDLINE", format!("{PATH_ROOT}/proc/1/cmdline"));
    let PATH_PROC_UPTIME = get_env_var("PATH_PROC_UPTIME", format!("{PATH_ROOT}/proc/uptime"));
    // let PATH_ETC_CLOUD=  get_env_var("PATH_ETC_CLOUD:-${PATH_ROOT}/etc/cloud}";
    // let PATH_ETC_CI_CFG=  get_env_var("PATH_ETC_CI_CFG:-${PATH_ETC_CLOUD}/cloud.cfg}";
    // let PATH_ETC_CI_CFG_D=  get_env_var("PATH_ETC_CI_CFG_D:-${PATH_ETC_CI_CFG}.d}";
    let PATH_RUN_CI = get_env_var("PATH_RUN_CI", format!("{PATH_RUN}/cloud-init"));
    // let PATH_RUN_CI_CFG=${PATH_RUN_CI_CFG:-${PATH_RUN_CI}/cloud.cfg};
    // let PATH_RUN_DI_RESULT=${PATH_RUN_DI_RESULT:-${PATH_RUN_CI}/.ds-identify.result};

    let DI_LOG = get_env_var("DI_LOG", format!("{PATH_RUN_CI}/ds-identify.log"));

    debug(
        1,
        format!(
            "[up {}s] ds-identify {args_str}",
            read_uptime(PATH_PROC_UPTIME)
        ),
    );

    collect_info(&paths);
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
