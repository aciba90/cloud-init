use std::{fmt::Display, fs, path::Path};

use crate::{
    constants::{DI_DISABLED, DI_ENABLED},
    paths::Paths,
    util::unquote,
};

use super::uname::UnameInfo;

pub struct Config {
    dsname: Option<String>,
    pub mode: Mode,
    pub on_found: Found,
    pub on_maybe: Maybe,
    pub on_notfound: NotFound,
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
pub enum Found {
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
pub enum Maybe {
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
pub enum NotFound {
    /// disable cloud-init
    #[default]
    Disabled,
    /// enable cloud-init
    Enabled,
}

impl NotFound {
    pub fn cli_repr(&self) -> String {
        match self {
            Self::Disabled => "disable".to_owned(),
            Self::Enabled => "enable".to_owned(),
        }
    }
}

// TODO: test fixing default modes
// XXX: == Config? who uses this?
#[derive(Debug)]
struct Policy {
    mode: Mode,
    on_found: Found,
    on_maybe: Maybe,
    on_notfound: NotFound,
    _report: bool,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            mode: Mode::Search,
            on_found: Found::default(),
            on_maybe: Maybe::default(),
            on_notfound: NotFound::default(),
            _report: false,
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
        #[allow(clippy::wildcard_in_or_patterns)]
        match &uname.machine[..] {
            // these have dmi data
            "i686" | "i386" | "x86_64" => Policy::default(),
            // aarch64 has dmi, but not currently used (LP: #1663304)
            "aarch64" | _ => Policy::default_no_dmi(),
        }
    }

    fn parse_from_str(policy_str: &str, uname: &UnameInfo) -> Self {
        let mut policy = Policy::parse_from_uname(uname);

        let mut mode = None;
        let mut found = None;
        let mut maybe = None;
        let mut notfound = None;
        for tok in policy_str.trim().split(',') {
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
