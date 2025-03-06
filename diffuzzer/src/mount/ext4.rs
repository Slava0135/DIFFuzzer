/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fmt::Display;

use regex::RegexSet;

use crate::fuzzing::greybox::feedback::CoverageType;

use super::FileSystemMount;

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
    fn coverage_type(&self) -> CoverageType {
        CoverageType::KCov
    }
}

impl Ext4 {
    pub const fn new() -> Self {
        Self {}
    }
}
