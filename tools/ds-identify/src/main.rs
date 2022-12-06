use std::fmt::Display;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
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
const DI_ENABLED: &str = "enabled";
const DI_DISABLED: &str = "disabled";

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
const PATH_RUN_CI_CFG: &str = "cloud.cfg";
const PATH_RUN_DI_RESULT: &str = ".ds-identify.result";

const DI_DSLIST_DEFAULT: &str = "MAAS ConfigDrive NoCloud AltCloud Azure Bigstep \
CloudSigma CloudStack DigitalOcean Vultr AliYun Ec2 GCE OpenNebula OpenStack \
OVF SmartOS Scaleway Hetzner IBMCloud Oracle Exoscale RbxCloud UpCloud VMware \
LXD NWCS";

struct Paths {
    root: PathBuf,
    pub var_lib_cloud: PathBuf,
    pub di_config: PathBuf,
    pub run: PathBuf,
    pub proc_cmdline: PathBuf,
    pub proc_1_cmdline: PathBuf,
    pub proc_1_environ: PathBuf,
    pub proc_uptime: PathBuf,
    pub run_ci: PathBuf,
    pub run_ci_cfg: PathBuf,
    pub run_di_result: PathBuf,
}

impl Paths {
    fn with_root(root: &Path) -> Self {
        let run = Self::compose_paths(root, PATH_RUN);
        let run_ci = Self::compose_paths(&run, PATH_RUN_CI);
        Self::from_roots(root, &run, &run_ci)
    }

    fn from_roots(root: &Path, run: &Path, run_ci: &Path) -> Self {
        Self {
            root: root.to_owned(),
            run: run.to_owned(),
            var_lib_cloud: Self::compose_paths(root, PATH_VAR_LIB_CLOUD),
            di_config: Self::compose_paths(root, PATH_DI_CONFIG),
            proc_cmdline: Self::compose_paths(root, PATH_PROC_CMDLINE),
            proc_1_cmdline: Self::compose_paths(root, PATH_PROC_1_CMDLINE),
            proc_1_environ: Self::compose_paths(root, PATH_PROC_1_ENVIRON),
            proc_uptime: Self::compose_paths(root, PATH_PROC_UPTIME),
            run_ci: run_ci.to_owned(),
            run_ci_cfg: Self::compose_paths(run_ci, PATH_RUN_CI_CFG),
            run_di_result: Self::compose_paths(run_ci, PATH_RUN_DI_RESULT),
        }
    }

    fn compose_paths<P: AsRef<Path>, S: AsRef<OsStr>>(root: P, default: S) -> PathBuf {
        root.as_ref().join(default.as_ref())
    }

    fn path_from_env<P: AsRef<Path>, S: AsRef<OsStr>>(root: P, name: &str, default: S) -> PathBuf {
        env::var(name).map_or_else(|_| Self::compose_paths(&root, &default), PathBuf::from)
    }
    pub fn from_env() -> Self {
        let root = env::var("PATH_ROOT").unwrap_or_else(|_| String::from("/"));
        let root = Path::new(&root);
        let run = Self::path_from_env(root, "PATH_RUN", Self::compose_paths(&root, &PATH_RUN));
        let run_ci =
            Self::path_from_env(root, "PATH_RUN_CI", Self::compose_paths(&run, &PATH_RUN_CI));

        let default_paths = Paths::from_roots(&root, &run, &run_ci);

        let var_lib_cloud =
            Self::path_from_env(root, "PATH_VAR_LIB_CLOUD", &default_paths.var_lib_cloud);
        let di_config = Self::path_from_env(root, "PATH_DI_CONFIG", &default_paths.di_config);
        let proc_cmdline =
            Self::path_from_env(root, "PATH_PROC_CMDLINE", &default_paths.proc_cmdline);
        let proc_1_cmdline =
            Self::path_from_env(root, "PATH_PROC_1_CMDLINE", &default_paths.proc_1_cmdline);
        let proc_1_environ =
            Self::path_from_env(root, "PATH_PROC_1_ENVIRON", &default_paths.proc_1_environ);
        let proc_uptime = Self::path_from_env(root, "PATH_PROC_UPTIME", &default_paths.proc_uptime);
        let run_ci_cfg = Self::path_from_env(root, "PATH_RUN_CI_CFG", &default_paths.run_ci_cfg);
        let run_di_result =
            Self::path_from_env(root, "PATH_RUN_DI_RESULT", &default_paths.run_di_result);

        Paths {
            root: PathBuf::from(root),
            var_lib_cloud,
            di_config,
            run,
            proc_cmdline,
            proc_1_cmdline,
            proc_1_environ,
            proc_uptime,
            run_ci,
            run_ci_cfg,
            run_di_result,
        }
    }

    fn log(&self) -> PathBuf {
        self.run_ci.join("ds-identify.log")
    }
}

fn error<S: AsRef<str>>(msg: S) {
    let msg = format!("Error: {}", msg.as_ref());
    debug(0, &msg);
    eprintln!("{}", &msg);
}

// TODO: as macro
fn debug<S: AsRef<str>>(level: i32, msg: S) {
    // TODO: Find a way to not recompute this value in every call
    let debug_level: i32 = get_env_var("DEBUG_LEVEL", String::from("-1"))
        .parse()
        .unwrap();
    if level >= debug_level {
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
    virt: Virt,
    pid1_prod_name: String,
    kernel_cmdline: String,
    config: Config,
}

impl Info {
    fn collect_info(paths: &Paths) -> Self {
        let uname_info = UnameInfo::read();
        let virt = Virt::from(&uname_info);
        let pid1_prod_name = Self::read_pid1_product_name(&paths.proc_1_environ);
        let kernel_cmdline = Self::read_kernel_cmdline(&paths, virt.is_container());
        let config = Config::read(&paths, &kernel_cmdline, &uname_info);
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

fn is_systemd() -> bool {
    path::Path::new("/run/systemd").is_dir()
}

struct Virt(String);

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

#[derive(Debug)]
enum Mode {
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

struct Config {
    dsname: Option<String>,
    mode: Mode,
    on_found: Found,
    on_maybe: Maybe,
    on_notfound: NotFound,
}

impl Config {
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
                    let val = val.trim();
                    // TODO: unquote `val`
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

struct DatasourceList;

impl DatasourceList {
    fn found() {
        todo!();
    }
}

fn is_manual_clean_and_exiting(var_lib_cloud: &Path) -> bool {
    return var_lib_cloud.join("instance/manual-clean").is_file();
}

fn write_result(content: &str, paths: &Paths, mode: &Mode) {
    // TODO: error handling
    let mut stream = BufWriter::new(File::open(&paths.run_ci_cfg).unwrap());
    let mut pre = "";
    if let Mode::Report = mode {
        stream.write(b"di_report:\n").unwrap();
        pre = "  ";
    }
    for line in content.lines() {
        writeln!(stream, "{}{}", pre, line).unwrap();
    }
}

fn _main() {
    // TODO: ensure_sane_path

    let args: Vec<String> = env::args().collect();
    let args_str: &str = &args[1..].join(" ");

    let paths = Paths::from_env();
    let di_log = paths.log();

    debug(
        1,
        format!(
            "[up {}s] ds-identify {args_str}",
            read_uptime(&paths.proc_uptime)
        ),
    );

    let info = Info::collect_info(&paths);

    if di_log.to_str().unwrap() == "stderr" {
        todo!();
    } else {
        let old_cli_str = info.to_old_str();
        // TODO: print to `DI_LOG`;
        println!("{old_cli_str}");
    }

    const RET_DISABLED: i32 = 1;
    const RET_ENABLED: i32 = 0;

    match info.config.mode {
        Mode::Disabled => {
            debug(
                1,
                format!("mode={}. returning {}", Mode::Disabled, RET_DISABLED),
            );
            process::exit(RET_DISABLED);
        }
        Mode::Enabled => {
            debug(
                1,
                format!("mode={}. returning {}", Mode::Enabled, RET_ENABLED),
            );
            process::exit(RET_ENABLED);
        }
        _ => (),
    }

    if let Some(dsname) = info.config.dsname {
        debug(1, format!("datasource '{dsname}' specified."));
        // TODO: found
        return;
    }

    if is_manual_clean_and_exiting(&paths.var_lib_cloud) {
        debug(
            1,
            "manual_cache_clean enabled. Not writing datasource_list.",
        );
        write_result("# manual_cache_clean.", &paths, &info.config.mode);
        return;
    }
}

fn main() {
    let di_main = get_env_var("DI_MAIN", String::from("main"));
    match &di_main[..] {
        "main" | "print_info" | "noop" => _main(),
        _ => {
            error("unexpected value for DI_MAIN");
            process::exit(1);
        }
    }
}
