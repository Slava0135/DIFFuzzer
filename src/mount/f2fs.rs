use std::fmt::Display;

use super::mount::FileSystemMount;

pub struct F2FS;

impl Display for F2FS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "F2FS")
    }
}

impl FileSystemMount for F2FS {
    fn mkfs_cmd(&self) -> String {
        "mkfs.f2fs".to_owned()
    }
    fn mkfs_opts(&self) -> Option<String> {
        Some("extra_attr,inode_checksum,sb_checksum,compression".to_owned())
    }
    fn mount_t(&self) -> String {
        "f2fs".to_owned()
    }
    fn mount_opts(&self) -> Option<String> {
        Some("compress_algorithm=zstd:6,compress_chksum,atgc,gc_merge,lazytime".to_owned())
    }
}

impl F2FS {
    pub fn new() -> Self {
        Self {}
    }
}
