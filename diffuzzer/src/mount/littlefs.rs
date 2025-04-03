/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{fmt::Display, path::Path};

use anyhow::Context;

use crate::{
    command::{CommandInterface, CommandWrapper},
    fuzzing::greybox::feedback::CoverageType,
    mount::{DEVICE, setup_modprobe},
    path::RemotePath,
};

use super::FileSystemMount;

pub struct LittleFS;

impl Display for LittleFS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LittleFS")
    }
}

impl FileSystemMount for LittleFS {
    fn setup(&self, cmdi: &dyn CommandInterface, path: &RemotePath) -> anyhow::Result<()> {
        cmdi.create_dir_all(path)
            .with_context(|| "failed to create mountpoint")?;

        setup_modprobe(cmdi)?;

        let lfs_path = self
            .source_dir()
            .with_context(|| "Source directory with binary missing")?
            .join("lfs");

        let mut format = CommandWrapper::new(lfs_path.base.as_ref());
        format.arg("--format").arg(DEVICE);
        cmdi.exec(format, None)
            .with_context(|| format!("failed to format device '{}'", DEVICE))?;

        let mut mount = CommandWrapper::new(lfs_path.base.as_ref());
        mount.arg(DEVICE).arg(path.base.as_ref());
        cmdi.exec(mount, None)
            .with_context(|| format!("failed to mount filesystem at '{}'", path))?;

        Ok(())
    }
    fn coverage_type(&self) -> CoverageType {
        CoverageType::LCov
    }
    fn source_dir(&self) -> Option<RemotePath> {
        Some(RemotePath::new(Path::new("/root/littlefs-fuse")))
    }
}

impl LittleFS {
    pub const fn new() -> Self {
        Self {}
    }
}
