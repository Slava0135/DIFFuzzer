use std::fmt::Display;

use super::mount::FileSystemMount;

pub struct XFS;

impl Display for XFS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "XFS")
    }
}

impl FileSystemMount for XFS {
    fn mkfs_cmd(&self) -> String {
        "mkfs.xfs".to_owned()
    }
    fn mount_t(&self) -> String {
        "xfs".to_owned()
    }
}

impl XFS {
    pub const fn new() -> Self {
        Self {}
    }
}
