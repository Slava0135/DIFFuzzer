use std::fmt::Display;

use super::mount::FileSystemMount;

pub struct Btrfs;

impl Display for Btrfs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Btrfs")
    }
}

impl FileSystemMount for Btrfs {}

impl Btrfs {
    pub const fn new() -> Self {
        Self {}
    }
}
