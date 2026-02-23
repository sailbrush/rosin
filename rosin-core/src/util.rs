use std::{path::PathBuf, time::SystemTime};

#[derive(Debug, Clone)]
pub(crate) struct ResourceInfo {
    pub last_modified: SystemTime,
    pub path: PathBuf,
}
