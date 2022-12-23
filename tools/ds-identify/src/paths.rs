use crate::constants::*;
use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Paths {
    root: PathBuf,
    pub run: PathBuf,
    pub sys_class_dmi_id: PathBuf,
    pub var_lib_cloud: PathBuf,
    pub di_config: PathBuf,
    pub proc_cmdline: PathBuf,
    pub proc_1_cmdline: PathBuf,
    pub proc_1_environ: PathBuf,
    pub proc_uptime: PathBuf,
    pub etc_cloud: PathBuf,
    pub etc_ci_cfg: PathBuf,
    pub etc_ci_cfg_d: PathBuf,
    pub run_ci: PathBuf,
    pub run_ci_cfg: PathBuf,
    pub run_di_result: PathBuf,
}

impl Paths {
    fn with_root(root: &Path) -> Self {
        let run = Self::compose_paths(root, PATH_RUN);
        let run_ci = Self::compose_paths(&run, PATH_RUN_CI);
        let etc_cloud = Self::compose_paths(&root, PATH_ETC_CLOUD);
        Self::from_roots(root, &run, &run_ci, &etc_cloud)
    }

    fn from_roots(root: &Path, run: &Path, run_ci: &Path, etc_cloud: &Path) -> Self {
        let etc_ci_cfg = Self::compose_paths(etc_cloud, PATH_ETC_CI_CFG);
        let etc_ci_cfg_d = Self::compose_paths(etc_cloud, format!("{}.d", PATH_ETC_CI_CFG));
        Self {
            root: root.to_owned(),
            run: run.to_owned(),
            sys_class_dmi_id: Self::compose_paths(root, PATH_SYS_CLASS_DMI_ID),
            var_lib_cloud: Self::compose_paths(root, PATH_VAR_LIB_CLOUD),
            di_config: Self::compose_paths(root, PATH_DI_CONFIG),
            proc_cmdline: Self::compose_paths(root, PATH_PROC_CMDLINE),
            proc_1_cmdline: Self::compose_paths(root, PATH_PROC_1_CMDLINE),
            proc_1_environ: Self::compose_paths(root, PATH_PROC_1_ENVIRON),
            proc_uptime: Self::compose_paths(root, PATH_PROC_UPTIME),
            etc_cloud: etc_cloud.to_owned(),
            etc_ci_cfg,
            etc_ci_cfg_d,
            run_ci: run_ci.to_owned(),
            run_ci_cfg: Self::compose_paths(run_ci, PATH_RUN_CI_CFG),
            run_di_result: Self::compose_paths(run_ci, PATH_RUN_DI_RESULT),
        }
    }

    fn compose_paths<P, S>(root: P, default: S) -> PathBuf
    where
        P: AsRef<Path>,
        S: AsRef<OsStr>,
    {
        root.as_ref().join(default.as_ref())
    }

    fn path_from_env<S>(name: &str, root: Option<&Path>, default: S) -> PathBuf
    where
        S: AsRef<OsStr>,
    {
        match (env::var(name), root) {
            (Ok(path), _) => PathBuf::from(&path),
            (_, Some(root)) => Self::compose_paths(&root, default.as_ref()),
            (_, None) => PathBuf::from(default.as_ref()),
        }
    }
    pub fn from_env() -> Self {
        let root = env::var("PATH_ROOT").unwrap_or_else(|_| String::from("/"));
        let root = Path::new(&root);
        let run = Self::path_from_env("PATH_RUN", Some(&root), &PATH_RUN);
        let etc_cloud = Self::path_from_env("PATH_ETC_CLOUD", Some(&root), &PATH_ETC_CLOUD);
        let run_ci = Self::path_from_env("PATH_RUN_CI", Some(&run), &PATH_RUN_CI);

        let default_paths = Paths::from_roots(&root, &run, &run_ci, &etc_cloud);

        let sys_class_dmi_id = Self::path_from_env(
            "PATH_SYS_CLASS_DMI_ID",
            None,
            &default_paths.sys_class_dmi_id,
        );
        let var_lib_cloud =
            Self::path_from_env("PATH_VAR_LIB_CLOUD", None, &default_paths.var_lib_cloud);
        let di_config = Self::path_from_env("PATH_DI_CONFIG", None, &default_paths.di_config);
        let proc_cmdline =
            Self::path_from_env("PATH_PROC_CMDLINE", None, &default_paths.proc_cmdline);
        let proc_1_cmdline =
            Self::path_from_env("PATH_PROC_1_CMDLINE", None, &default_paths.proc_1_cmdline);
        let proc_1_environ =
            Self::path_from_env("PATH_PROC_1_ENVIRON", None, &default_paths.proc_1_environ);
        let proc_uptime = Self::path_from_env("PATH_PROC_UPTIME", None, &default_paths.proc_uptime);
        let etc_ci_cfg = Self::path_from_env("PATH_ETC_CI_CFG", None, &default_paths.etc_ci_cfg);
        let etc_ci_cfg_d =
            Self::path_from_env("PATH_ETC_CI_CFG_D", None, &default_paths.etc_ci_cfg_d);
        let run_ci_cfg = Self::path_from_env("PATH_RUN_CI_CFG", None, &default_paths.run_ci_cfg);
        let run_di_result =
            Self::path_from_env("PATH_RUN_DI_RESULT", None, &default_paths.run_di_result);

        Paths {
            root: PathBuf::from(root),
            sys_class_dmi_id,
            var_lib_cloud,
            di_config,
            run,
            proc_cmdline,
            proc_1_cmdline,
            proc_1_environ,
            proc_uptime,
            etc_cloud,
            etc_ci_cfg,
            etc_ci_cfg_d,
            run_ci,
            run_ci_cfg,
            run_di_result,
        }
    }

    // XXX: move to attr
    pub fn log(&self) -> PathBuf {
        self.run_ci.join("ds-identify.log")
    }

    pub fn etc_ci_cfg_paths(&self) -> Vec<PathBuf> {
        let mut cfg_paths = vec![self.etc_ci_cfg.clone()];

        if self.etc_ci_cfg.is_dir() {
            for entry in self.etc_ci_cfg_d.read_dir().unwrap() {
                let entry = entry.unwrap().path();
                if !entry.ends_with(".cfg") {
                    continue;
                }
                cfg_paths.push(entry.into());
            }
        }

        cfg_paths
    }
}
