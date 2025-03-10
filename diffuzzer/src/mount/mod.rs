/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

pub mod btrfs;
pub mod ext4;
pub mod f2fs;
pub mod littlefs;
pub mod xfs;

use std::fmt::Display;

use anyhow::Context;
use log::debug;
use regex::RegexSet;

use crate::{
    command::{CommandInterface, CommandWrapper},
    fuzzing::greybox::feedback::CoverageType,
    path::RemotePath,
};

const RAM_DISK_SIZE: usize = 1_000_000;
const DEVICE: &str = "/dev/ram0";

pub trait FileSystemMount: Display {
    fn setup(&self, cmdi: &dyn CommandInterface, path: &RemotePath) -> anyhow::Result<()> {
        debug!("setup '{}' filesystem at '{}'", self, path);

        cmdi.create_dir_all(path)
            .with_context(|| "failed to create mountpoint")?;

        let mut modprobe = CommandWrapper::new("modprobe");
        modprobe
            .arg("brd")
            .arg("rd_nr=1")
            .arg(format!("rd_size={RAM_DISK_SIZE}"));
        cmdi.exec(modprobe, None)
            .with_context(|| "failed to load module 'brd'")?;

        let mut mkfs = CommandWrapper::new(self.mkfs_cmd());
        if let Some(opts) = self.mkfs_opts() {
            mkfs.arg("-O");
            mkfs.arg(opts);
        }
        mkfs.arg(DEVICE);
        cmdi.exec(mkfs, None)
            .with_context(|| "failed to make filesystem")?;

        let mut mount = CommandWrapper::new("mount");
        mount.arg("-t").arg(self.mount_t());
        if let Some(opts) = self.mount_opts() {
            mount.arg("-o");
            mount.arg(opts);
        }
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

    /// Used in default implementation: `mkfs` command to make new FS.
    /// Example: `"mkfs.ext4"` or `"mkfs.btrfs"`
    fn mkfs_cmd(&self) -> String {
        todo!()
    }

    /// Used in default implementation: `mkfs -O` argument.
    /// Example: `extra_attr,inode_checksum,sb_checksum,compression`
    fn mkfs_opts(&self) -> Option<String> {
        None
    }

    /// Used in default implementation: `mount -t` argument.
    /// Example: `"ext4"` or `"btrfs"`
    fn mount_t(&self) -> String {
        todo!()
    }

    /// Used in default implementation: `mount -o` argument.
    /// Example: `compress_algorithm=zstd:6,compress_chksum,atgc,gc_merge,lazytime`
    fn mount_opts(&self) -> Option<String> {
        None
    }

    fn get_internal_dirs(&self) -> RegexSet {
        RegexSet::new::<_, &str>([]).unwrap()
    }

    fn coverage_type(&self) -> CoverageType;

    /// Directory with source files, if exists.
    /// Required for LCov.
    fn source_dir(&self) -> Option<RemotePath> {
        None
    }
}
