use std::{env, fs, io::BufRead, io::BufReader, path::Path};

use crate::{
    constants::DI_DSLIST_DEFAULT,
    paths::Paths,
    util::{parse_yaml_array, Logger},
};

use super::Datasource;

#[derive(Debug)]
pub struct DatasourceList(Vec<Datasource>);

impl DatasourceList {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn read(logger: &Logger, paths: &Paths) -> Self {
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
            logger.debug(
                1,
                format!("{:?} set datasource_list: {}", path, found_dslist),
            );
            let dslist = parse_yaml_array(&found_dslist);
            let dslist = dslist.iter().map(|x| (*x).into()).collect();
            return Self(dslist);
        };

        DatasourceList::default()
    }

    pub fn push(&mut self, ds: Datasource) {
        self.0.push(ds);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// determines if there is only a single non-none ds entry or not
    pub fn only_one_not_none(&self) -> bool {
        if self.0.len() == 1 {
            return true;
        }
        if self.0.len() == 2 && matches!(self.0.last().expect("an element"), Datasource::None) {
            return true;
        }
        false
    }

    pub fn to_old_str(&self) -> String {
        self.0
            .iter()
            .map(String::from)
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn as_old_list(&self) -> Vec<String> {
        self.0.iter().map(|ds| ds.into()).collect::<Vec<_>>()
    }

    pub fn keep_first(&mut self) {
        self.0.truncate(1);
    }
}

impl Default for DatasourceList {
    fn default() -> Self {
        Self(DI_DSLIST_DEFAULT.split(' ').map(|s| s.into()).collect())
    }
}

impl From<&str> for DatasourceList {
    fn from(value: &str) -> Self {
        Self(value.split_whitespace().map(|s| s.into()).collect())
    }
}

impl<'a> IntoIterator for &'a DatasourceList {
    type Item = &'a Datasource;
    type IntoIter = std::slice::Iter<'a, Datasource>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromIterator<Datasource> for DatasourceList {
    fn from_iter<T: IntoIterator<Item = Datasource>>(iter: T) -> Self {
        let mut c = DatasourceList::new();

        for i in iter {
            c.push(i);
        }

        c
    }
}

/// somewhat hackily read through paths for `key`
///
/// currently does not respect any hierarchy in searching for key.
fn check_config<'a, P: AsRef<Path>>(key: &str, paths: &'a [P]) -> Option<(String, &'a Path)> {
    let mut value_path = None;

    for f in paths.iter().filter(|p| p.as_ref().is_file()) {
        let stream = BufReader::new(fs::File::open(f).unwrap());
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
