use std::io::{BufRead, BufReader, BufWriter, Write};

use ds_identify::constants::UNAVAILABLE;
use ds_identify::dss::{Datasource, DscheckResult};
use ds_identify::info::{DatasourceList, Found, Info, Maybe, Mode, NotFound};
use ds_identify::paths::Paths;
use ds_identify::util::{ensure_sane_path, get_env_var, Logger};
use std::process::ExitCode;
use std::{env, fs, path::Path};

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

fn is_manual_clean_and_exiting(var_lib_cloud: &Path) -> bool {
    var_lib_cloud.join("instance/manual-clean").is_file()
}

fn write_result(logger: &Logger, content: &str, paths: &Paths, mode: &Mode) {
    let runcfg = &paths.run_ci_cfg;
    let error_fn = || {
        logger.error(format!("failed to write to {:?}", runcfg));
        panic!("failed to write to {:?}", runcfg);
    };

    let file = fs::File::create(&paths.run_ci_cfg);
    let mut ostream = match file {
        Err(e) => {
            eprintln!("{}", e);
            error_fn()
        }
        Ok(file) => BufWriter::new(file),
    };

    let pre = match mode {
        Mode::Report => "  ",
        _ => "",
    };
    for line in content.lines() {
        if line.len() == 0 {
            continue;
        }
        writeln!(ostream, "{}{}", pre, line).unwrap();
    }
}

fn found<S: AsRef<str>>(
    info: &Info,
    mode: Option<&Mode>,
    ds_list: &[S],
    extra_lines: Option<&str>,
) {
    let mode = mode.unwrap_or_else(|| &info.config().mode);

    let list = ds_list
        .iter()
        .map(|s| s.as_ref())
        .collect::<Vec<_>>()
        .join(", ");
    // TODO: Add ds None as fallback
    let result = format!("datasource_list: [{}]", list);
    write_result(&info.logger(), &result, &info.paths(), mode);
    if let Some(extra_lines) = extra_lines {
        write_result(&info.logger(), &extra_lines, &info.paths(), mode);
    }
}

/// in report mode, report nothing was found.
/// if not report mode: only report the negative result.
///   reporting an empty list would mean cloud-init would not search
///   any datasources.
fn record_notfound(info: &Info) {
    match info.config().mode() {
        Mode::Report => {
            found::<&str>(&info, None, &[], None);
        }
        Mode::Search => {
            let msg = format!(
                "# reporting not found result. notfound={}.",
                info.config().on_notfound.cli_repr()
            );
            found::<&str>(&info, Some(&Mode::Report), &[], Some(&msg));
        }
        _ => (),
    }
}

fn print_info() {
    let paths = Paths::from_env();
    let logger = Logger::new(&paths.log());
    let info = Info::collect_info(&logger, &paths);
    println!("{}", info.to_old_str());
}

fn ds_identify_inner(logger: &Logger, info: &Info) -> u8 {
    let di_log = info.paths().log();
    if di_log == "stderr" {
        todo!();
    } else {
        let old_cli_str = info.to_old_str();
        // TODO: print to `DI_LOG`;
        println!("{}", old_cli_str);
    }

    const RET_DISABLED: u8 = 1;
    const RET_ENABLED: u8 = 0;

    match info.config().mode() {
        Mode::Disabled => {
            logger.debug(
                1,
                format!("mode={}. returning {}", Mode::Disabled, RET_DISABLED),
            );
            return RET_DISABLED;
        }
        Mode::Enabled => {
            logger.debug(
                1,
                format!("mode={}. returning {}", Mode::Enabled, RET_ENABLED),
            );
            return RET_ENABLED;
        }
        _ => (),
    }

    if let Some(dsname) = info.config().dsname() {
        logger.debug(1, format!("datasource '{dsname}' specified."));
        found(&info, None, &[dsname], None);
        return 0;
    }

    if is_manual_clean_and_exiting(&info.paths().var_lib_cloud) {
        logger.debug(
            1,
            "manual_cache_clean enabled. Not writing datasource_list.",
        );
        write_result(
            &logger,
            "# manual_cache_clean.",
            info.paths(),
            info.config().mode(),
        );
        return 0;
    }

    // if there is only a single entry in $DI_DSLIST
    if info.dslist().only_one_not_none() {
        logger.debug(
            1,
            format!(
                "single entry in datasource_list ({}) use that.",
                info.dslist().to_old_str(),
            ),
        );
        let ds_list = info.dslist().as_old_list();
        found(&info, None, &ds_list, None);
        return 0;
    }

    // Check datasources
    let mut found_dss = DatasourceList::new();
    let mut maybe_dss = DatasourceList::new();
    let mut exfound = String::new();
    let mut exmaybe = String::new();
    for ds in info.dslist() {
        let ds_as_str = String::from(ds);
        logger.debug(2, format!("Checking for datasource '{}'", ds_as_str));
        if let Datasource::Unknown(ds) = ds {
            logger.warn(format!("No check method for datasource '{}'", ds));
            continue;
        }

        match ds.dscheck_fn()(&info) {
            DscheckResult::Found(extra_config) => {
                logger.debug(1, format!("check for '{}' returned found", ds_as_str));
                found_dss.push(ds.clone());
                if let Some(extra_config) = extra_config {
                    exfound.push_str(&extra_config);
                }
            }
            DscheckResult::Maybe(extra_config) => {
                logger.debug(1, format!("check for '{}' returned maybe", ds_as_str));
                maybe_dss.push(ds.clone());
                if let Some(extra_config) = extra_config {
                    exmaybe.push_str(&extra_config);
                }
            }
            DscheckResult::NotFound => {
                logger.debug(2, format!("check for '{}' returned not-found", ds_as_str));
            }
        }
    }

    logger.debug(2, format!("found={:?} maybe={:?}", found_dss, maybe_dss));
    if found_dss.len() > 0 {
        let first_ds = found_dss.into_iter().nth(0).expect("at leaset one");
        if found_dss.len() == 1 {
            logger.debug(
                1,
                format!("Found single datasource: {}", String::from(first_ds)),
            );
        } else {
            // found=all
            logger.debug(
                1,
                format!(
                    "Found {} datasources found={:?}: {:?}",
                    found_dss.len(),
                    info.config().on_found,
                    found_dss
                ),
            );
            if let Found::First = info.config().on_found {
                found_dss.keep_first();
            }
        }
        found(&info, None, &found_dss.as_old_list(), Some(&exfound));
        return 0;
    }

    if maybe_dss.len() > 0 && !matches!(info.config().on_maybe, Maybe::None) {
        logger.debug(
            1,
            format!(
                "{} datasources returned maybe: {:?}",
                maybe_dss.len(),
                maybe_dss
            ),
        );
        found(&info, None, &maybe_dss.as_old_list(), Some(&exmaybe));
        return 0;
    }

    // record the empty result.
    record_notfound(&info);

    let base_msg = format!(
        "No ds found [mode={}, notfound={}].",
        info.config().mode(),
        info.config().on_notfound.cli_repr()
    );

    let (msg, ret_code) = match (info.config().mode(), &info.config().on_notfound) {
        (Mode::Report, NotFound::Disabled) => {
            let msg = format!("{} Would disable cloud-init [{}]", base_msg, RET_DISABLED);
            (msg, RET_ENABLED) // XXX: Is `RET_ENABLED` correct here?
        }
        (Mode::Report, NotFound::Enabled) => {
            let msg = format!("{} Would enable cloud-init [{}]", base_msg, RET_ENABLED);
            (msg, RET_ENABLED)
        }
        (Mode::Search, NotFound::Disabled) => {
            let msg = format!("{} Disabled cloud-init [{}]", base_msg, RET_DISABLED);
            (msg, RET_DISABLED)
        }
        (Mode::Search, NotFound::Enabled) => {
            let msg = format!("{} Enabled cloud-init [{}]", base_msg, RET_ENABLED);
            (msg, RET_ENABLED)
        }
        _ => {
            logger.error("Unexpected result");
            (String::from(""), 3)
        }
    };
    logger.debug(1, msg);
    ret_code
}

fn ds_identify() -> ExitCode {
    ensure_sane_path();

    let paths = Paths::from_env();
    let di_log = paths.log();
    let logger = Logger::new(&di_log);

    let args: Vec<String> = env::args().skip(1).collect();
    let args_str: &str = &args.join(" ");

    logger.debug(
        1,
        format!(
            "[up {}s] ds-identify {}",
            read_uptime(&paths.proc_uptime),
            args_str,
        ),
    );

    let info = Info::collect_info(&logger, &paths);

    if !paths.run_ci.is_dir() {
        fs::create_dir_all(&paths.run_ci).unwrap();
    }

    // Handle cache
    let force = if let Some(first_arg) = args.get(1) {
        first_arg == "--force"
    } else {
        false
    };
    if !force && paths.run_ci_cfg.is_file() && paths.run_di_result.is_file() {
        if let Ok(previous_code) = fs::read_to_string(&paths.run_di_result) {
            let previous_code = previous_code.trim();
            match previous_code {
                "0" | "1" => {
                    logger.debug(
                        2,
                        format!(
                            "used cached result {}. pass --force to re-run.",
                            &previous_code
                        ),
                    );
                    return ExitCode::from(previous_code.parse::<u8>().expect("valid exit_code"));
                }
                _ => {
                    logger.debug(
                        1,
                        format!(
                            "prevous run returned unexpected '{}'. Re-running.",
                            &previous_code
                        ),
                    );
                }
            }
        } else {
            logger.error(format!(
                "failed to read result from {}",
                paths.run_di_result.display()
            ));
        }
    }

    let ret_code = ds_identify_inner(&logger, &info);

    let mut result_file =
        fs::File::create(&info.paths().run_di_result).expect("accessible result file");
    write!(result_file, "{}", ret_code).expect("result file accessible");

    logger.debug(
        1,
        format!(
            "[up {}s] returning {}",
            read_uptime(&paths.proc_uptime),
            ret_code,
        ),
    );

    ExitCode::from(ret_code)
}

fn main() -> ExitCode {
    let di_main = get_env_var("DI_MAIN", String::from("main"));
    match &di_main[..] {
        "main" => ds_identify(),
        "print_info" => {
            print_info();
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("unexpected value for DI_MAIN");
            ExitCode::FAILURE
        }
    }
}
