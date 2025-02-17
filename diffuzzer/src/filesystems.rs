/* Any copyright is dedicated to the Public Domain.
 * https://creativecommons.org/publicdomain/zero/1.0/ */

use crate::mount::{btrfs::Btrfs, ext4::Ext4, f2fs::F2FS, mount::FileSystemMount, xfs::Xfs};

pub const FILESYSTEMS: &[&dyn FileSystemMount] = &[
    &Ext4::new(),
    &Btrfs::new(),
    &F2FS::new(),
    &Xfs::new(),
    // your filesystem here
];

pub fn filesystems_available() -> Vec<String> {
    FILESYSTEMS
        .iter()
        .map(|fs| fs.to_string().to_lowercase())
        .collect()
}

impl From<String> for &'static dyn FileSystemMount {
    fn from(value: String) -> Self {
        let value = value.to_lowercase();
        for fs in FILESYSTEMS {
            if fs.to_string().to_lowercase() == value {
                return *fs;
            }
        }
        panic!("unknown filesystem '{}'", value);
    }
}
