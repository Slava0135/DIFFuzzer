use std::{fmt::Display, fs, io, path::Path, process::Command};

use anyhow::{bail, Context};
use log::debug;

const RAM_DISK_SIZE: usize = 1_000_000;
const DEVICE: &str = "/dev/ram0";

pub trait FileSystemMount: Display {
    fn setup(&self, path: &Path) -> anyhow::Result<()> {
        debug!("setting up '{}' filesystem at '{}'", self, path.display());

        fs::create_dir_all(path)
            .with_context(|| format!("failed to create mountpoint at '{}'", path.display()))?;

        let mut modprobe = Command::new("modprobe");
        modprobe
            .arg("brd")
            .arg("rd_nr=1")
            .arg(format!("rd_size={RAM_DISK_SIZE}"));
        let output = modprobe
            .output()
            .with_context(|| format!("failed to load block ram device module: {:?}", modprobe))?;
        if !output.status.success() {
            bail!(
                "failed to load block ram device module: {:?}\n{}",
                modprobe,
                String::from_utf8(output.stderr)
                    .with_context(|| format!("failed to read stderr (brd)"))?,
            );
        }

        let mut mkfs = Command::new(Self::mkfs_cmd());
        mkfs.arg(DEVICE);
        let output = mkfs.output()?;
        if !output.status.success() {
            bail!(
                "failed to create fs: {:?}\n{}",
                mkfs,
                String::from_utf8(output.stderr)
                    .with_context(|| format!("failed to read stderr (mkfs)"))?,
            );
        }

        let mut mount = Command::new("mount");
        mount.arg("-t").arg(Self::mount_t()).arg(DEVICE).arg(path);
        let output = mount.output()?;
        if !output.status.success() {
            bail!(
                "failed to mount fs: {:?}\n{}",
                mount,
                String::from_utf8(output.stderr)
                    .with_context(|| format!("failed to read stderr (mount)"))?,
            );
        }
        Ok(())
    }

    fn teardown(&self, path: &Path) -> anyhow::Result<()> {
        debug!("tearing down '{}' filesystem at '{}'", self, path.display());

        let mut umount = Command::new("umount");
        umount.arg("-fl").arg(path);
        let output = umount.output()?;
        if !output.status.success() {
            bail!(
                "failed to unmount fs: {:?}\n{}",
                umount,
                String::from_utf8(output.stderr)
                    .with_context(|| format!("failed to read stderr (umount)"))?,
            );
        }

        let mut rmmod = Command::new("rmmod");
        rmmod.arg("brd").output()?;
        let output = rmmod.output()?;
        if !output.status.success() {
            bail!(
                "failed to remove block ram device module fs: {:?}\n{}",
                rmmod,
                String::from_utf8(output.stderr)
                    .with_context(|| format!("failed to read stderr (umount)"))?,
            );
        }

        fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove mountpoint at '{}'", path.display()))?;

        Ok(())
    }

    /// Used in default implementation: command to make new FS.
    /// Example: `"mkfs.ext4"` or `"mkfs.btrfs"`
    fn mkfs_cmd() -> String {
        todo!()
    }

    /// Used in default implementation: `mount -t` argument.
    /// Example: `"ext4"` or `"btrfs"`
    fn mount_t() -> String {
        todo!()
    }
}
