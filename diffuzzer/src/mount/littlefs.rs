/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{fmt::Display, path::Path};

use anyhow::Context;
use log::debug;

use crate::{
    command::{CommandInterface, CommandWrapper},
    fuzzing::greybox::feedback::CoverageType,
    mount::{DEVICE, RAM_DISK_SIZE},
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
        debug!("setup '{}' filesystem at '{}'", self, path);

        cmdi.create_dir_all(path)
            .with_context(|| "failed to create mountpoint")?;

        let mut modprobe = CommandWrapper::new("modprobe");
        modprobe
            .arg("brd")
            .arg("rd_nr=1")
            .arg(format!("rd_size={}", RAM_DISK_SIZE));
        cmdi.exec(modprobe, None)
            .with_context(|| "failed to load module 'brd'")?;

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
    fn teardown(&self, cmdi: &dyn CommandInterface, path: &RemotePath) -> anyhow::Result<()> {
        debug!("teardown '{}' filesystem at '{}'", self, path);

        let mut umount = CommandWrapper::new("umount");
        umount.arg("-fl").arg(path.base.as_ref());
        cmdi.exec(umount, None)
            .with_context(|| format!("failed to unmount filesystem at '{}'", path))?;

        let mut rmmod = CommandWrapper::new("rmmod");
        rmmod.arg("brd");
        cmdi.exec(rmmod, None)
            .with_context(|| "failed to remove module 'brd'")?;

        cmdi.remove_dir_all(path)
            .with_context(|| "failed to remove mountpoint")?;

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
