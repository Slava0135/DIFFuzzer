/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fmt::Display;

use crate::fuzzing::greybox::feedback::CoverageType;

use super::FileSystemMount;

pub struct Btrfs;

impl Display for Btrfs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Btrfs")
    }
}

impl FileSystemMount for Btrfs {
    fn mkfs_cmd(&self) -> String {
        "mkfs.btrfs".to_owned()
    }
    fn mount_t(&self) -> String {
        "btrfs".to_owned()
    }
    fn coverage_type(&self) -> CoverageType {
        CoverageType::KCov
    }
}

impl Btrfs {
    pub const fn new() -> Self {
        Self {}
    }
}
