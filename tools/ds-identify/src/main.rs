use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use ds_identify::constants::UNAVAILABLE;
use ds_identify::info::{Info, Mode};
use ds_identify::paths::Paths;
use ds_identify::util::{debug, error, get_env_var};
use std::process;
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

enum Datasource {
    NoCloud,
}

fn is_manual_clean_and_exiting(var_lib_cloud: &Path) -> bool {
    var_lib_cloud.join("instance/manual-clean").is_file()
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
        println!("{}", old_cli_str);
    }

    const RET_DISABLED: i32 = 1;
    const RET_ENABLED: i32 = 0;

    match info.config().mode() {
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

    if let Some(dsname) = info.config().dsname() {
        debug(1, format!("datasource '{dsname}' specified."));
        // TODO: found
        return;
    }

    if is_manual_clean_and_exiting(&paths.var_lib_cloud) {
        debug(
            1,
            "manual_cache_clean enabled. Not writing datasource_list.",
        );
        write_result("# manual_cache_clean.", &paths, info.config().mode());
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
