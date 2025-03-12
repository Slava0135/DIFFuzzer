/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fmt::Display;

use anyhow::Context;
use log::debug;
use regex::RegexSet;

use crate::{
    command::CommandWrapper,
    fuzzing::greybox::feedback::CoverageType,
    mount::{DEVICE, setup_modprobe},
};

use super::FileSystemMount;

pub struct BcacheFS;

impl Display for BcacheFS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BcacheFS")
    }
}

impl FileSystemMount for BcacheFS {
    fn setup(
        &self,
        cmdi: &dyn crate::command::CommandInterface,
        path: &crate::path::RemotePath,
    ) -> anyhow::Result<()> {
        debug!("setup '{}' filesystem at '{}'", self, path);

        cmdi.create_dir_all(path)
            .with_context(|| "failed to create mountpoint")?;

        setup_modprobe(cmdi)?;

        let mut format = CommandWrapper::new("bcachefs");
        format.arg("format").arg(DEVICE);
        cmdi.exec(format, None)
            .with_context(|| format!("failed to format device '{}'", DEVICE))?;

        // mount -t bcachefs /dev/sda1 /mnt
        let mut mount = CommandWrapper::new("mount");
        mount.arg("-t").arg("bcachefs");
        mount.arg(DEVICE).arg(path.base.as_ref());
        cmdi.exec(mount, None)
            .with_context(|| format!("failed to mount filesystem at '{}'", path))?;

        Ok(())
    }
    fn get_internal_dirs(&self) -> RegexSet {
        RegexSet::new([r"^/?lost\+found($|/)"]).unwrap()
    }
    fn coverage_type(&self) -> CoverageType {
        CoverageType::KCov
    }
}

impl BcacheFS {
    pub const fn new() -> Self {
        Self {}
    }
}
