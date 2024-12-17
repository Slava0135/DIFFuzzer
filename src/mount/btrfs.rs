use std::fmt::Display;

use super::mount::FileSystemMount;

pub struct Btrfs;

impl Display for Btrfs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Btrfs")
    }
}

impl FileSystemMount for Btrfs {
    fn mkfs_cmd() -> String {
        "mkfs.btrfs".to_owned()
    }
    fn mount_t(&self) -> String {
        "btrfs".to_owned()
    }
}

impl Btrfs {
    pub fn new() -> Self {
        Self {}
    }
}
