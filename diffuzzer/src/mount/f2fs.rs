/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fmt::Display;

use regex::RegexSet;

use crate::fuzzing::greybox::feedback::CoverageType;

use super::FileSystemMount;

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
    fn get_internal_dirs(&self) -> RegexSet {
        RegexSet::new([r"^/?lost\+found($|/)"]).unwrap()
    }
    fn coverage_type(&self) -> CoverageType {
        CoverageType::KCov
    }
}

impl F2FS {
    pub const fn new() -> Self {
        Self {}
    }
}
