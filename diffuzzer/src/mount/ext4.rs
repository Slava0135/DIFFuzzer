use std::fmt::Display;

use regex::RegexSet;

use super::mount::FileSystemMount;

pub struct Ext4;

impl Display for Ext4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ext4")
    }
}

impl FileSystemMount for Ext4 {
    fn mkfs_cmd(&self) -> String {
        "mkfs.ext4".to_owned()
    }
    fn mount_t(&self) -> String {
        "ext4".to_owned()
    }

    fn get_internal_dirs(&self) -> RegexSet {
        RegexSet::new([r"^/?lost\+found($|/)"]).unwrap()
    }
}

impl Ext4 {
    pub const fn new() -> Self {
        Self {}
    }
}
