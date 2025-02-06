use std::fmt::Display;

use super::mount::FileSystemMount;

pub struct XFS;

impl Display for XFS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "XFS")
    }
}

impl FileSystemMount for XFS {}

impl XFS {
    pub const fn new() -> Self {
        Self {}
    }
}
