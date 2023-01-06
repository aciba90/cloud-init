use std::io::Write;
use std::{
    cell::RefCell,
    env,
    ffi::OsStr,
    fs,
    io::{self, BufWriter},
    path,
};

pub fn ensure_sane_path() {
    let mut path = env::var("PATH").expect("$PATH set");
    for p in &["/sbin", "/usr/sbin", "/bin", "/usr/bin"] {
        if path.contains(&format!(":{}:", p)) || path.contains(&format!(":{}:/", p)) {
            continue;
        }
        path.push(':');
        path.push_str(p);
    }
}

/// Remove quotes from quoted value.
pub fn unquote(val: &str) -> &str {
    const QUOTE: char = '"';
    const TICK: char = '\'';

    if val.starts_with(TICK) && val.ends_with(TICK) {
        return val.strip_prefix(TICK).unwrap().strip_suffix(TICK).unwrap();
    }

    // XXX: duplicated code
    if val.starts_with(QUOTE) && val.ends_with(QUOTE) {
        return val
            .strip_prefix(QUOTE)
            .unwrap()
            .strip_suffix(QUOTE)
            .unwrap();
    }

    val
}

/// parse a yaml single line array value ([1,2,3], not key: [1,2,3]).
/// supported with or without leading and closing brackets
///   ['1'] or [1]
///   '1', '2'
pub fn parse_yaml_array(val: &str) -> Vec<&str> {
    let val = val.strip_prefix('[').unwrap_or_else(|| val);
    let val = val.strip_prefix(']').unwrap_or_else(|| val);
    val.split(',').map(|tok| unquote(tok.trim())).collect()
}

pub fn get_env_var<K: AsRef<OsStr>>(key: K, default: String) -> String {
    env::var(key).unwrap_or_else(|_| default)
}

pub struct Logger {
    level: i32,
    writer: RefCell<BufWriter<Box<dyn io::Write>>>,
}

impl Logger {
    pub fn new<S: AsRef<str>>(di_log: S) -> Self {
        let level: i32 = get_env_var("DEBUG_LEVEL", String::from("-1"))
            .parse()
            .unwrap();

        let mut log_file = di_log.as_ref().trim();

        match log_file {
            "stderr" => (),
            _ => {
                if log_file.contains("/") {
                    // Create parent directories
                    // TODO: unit test
                    if let Some(parent_dir) = path::PathBuf::from(log_file).parent() {
                        if let Err(_) = ::std::fs::create_dir_all(parent_dir) {
                            eprintln!("ERROR: cannot write to {}", di_log.as_ref());
                            log_file = "stderr";
                        }
                    }
                }
            }
        }

        let writer: BufWriter<Box<dyn io::Write>> = match log_file {
            "stderr" => {
                dbg!("log to stderr");
                BufWriter::new(Box::new(io::stderr().lock()))
            }
            _ => {
                dbg!("log to file: {}", log_file);
                let file = fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(log_file)
                    .unwrap();
                BufWriter::new(Box::new(file))
            }
        };
        let writer = RefCell::new(writer);

        Self { level, writer }
    }

    fn log<S: AsRef<str>>(&self, level: i32, msg: S) {
        if level < self.level {
            return;
        }
        self.write_always(msg);
    }

    pub fn debug<S: AsRef<str>>(&self, level: i32, msg: S) {
        self.log(level, msg);
    }

    pub fn warn<S: AsRef<str>>(&self, msg: S) {
        let msg = format!("WARN: {}", msg.as_ref());
        self.debug(0, &msg);
        eprintln!("{}", &msg);
    }
    pub fn error<S: AsRef<str>>(&self, msg: S) {
        let msg = format!("ERROR: {}", msg.as_ref());
        self.debug(0, &msg);
        eprintln!("{}", &msg);
    }

    pub fn write_always<S: AsRef<str>>(&self, msg: S) {
        write!(self.writer.borrow_mut(), "{}\n", msg.as_ref()).expect("writable file");
    }
}

#[cfg(test)]
mod utils {
    use super::*;

    #[test]
    fn test_unquote() {
        assert_eq!("a", unquote("a"));
        assert_eq!("a", unquote("'a'"));
        assert_eq!("a", unquote("\"a\""));
    }

    #[test]
    fn test_parse_yaml_array() {
        assert_eq!(vec!["a"], parse_yaml_array("a"));
        assert_eq!(vec!["a", "b"], parse_yaml_array("a,b"));
        assert_eq!(vec!["a", "b"], parse_yaml_array("'a' ,  \"b\""));
    }
}
