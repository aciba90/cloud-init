use crate::info::Info;


#[derive(Debug)]
pub enum Datasource {
    None,
    NoCloud,
    Unknown(String),
}

#[derive(Debug)]
pub enum DscheckResult {
    Found,
    NotFound,
    Maybe,
}

impl Datasource {
    pub fn dscheck_fn(&self) -> fn (&Info) -> DscheckResult {
        match &self {
            Self::None => dscheck_none,
            Self::NoCloud => dscheck_cloud_stack,
            _ => todo!()
        }
    }
}

impl From<&str> for Datasource {
    fn from(val: &str) -> Self {
        match &val.to_lowercase()[..] {
            "nocloud" => Self::NoCloud,
            _ => Self::Unknown(val.to_string())
        }
    }

}

impl From<&Datasource> for String {
    fn from(ds: &Datasource) -> Self {
        match ds {
            Datasource::NoCloud => "NoCloud".to_string(),
            Datasource::None => "None".to_string(),
            Datasource::Unknown(ds) => format!("Unknown({})", ds),
        }
    }
}

fn dscheck_none(_info: &Info) -> DscheckResult {
    DscheckResult::NotFound
}


fn dscheck_cloud_stack(_info: &Info) -> DscheckResult {
    todo!();
}

