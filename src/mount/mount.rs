use std::{fmt::Display, fs, io, path::Path, process::Command};

use log::info;

const RAM_DISK_SIZE: usize = 1_000_000;
const DEVICE: &str = "/dev/ram0";

pub trait FileSystemMount: Display {
    fn setup(&self, path: &Path) -> io::Result<()> {
        info!("setting up '{}' filesystem at '{}'", self, path.display());

        info!("creating mountpoint at '{}'", path.display());
        fs::create_dir_all(path)?;

        let mut modprobe = Command::new("modprobe");
        modprobe
            .arg("brd")
            .arg("rd_nr=1")
            .arg(format!("rd_size={RAM_DISK_SIZE}"));
        info!(
            "loading block ram device module: {}",
            format!("{:?}", modprobe)
        );
        modprobe.output()?;

        let mut mkfs = Command::new(Self::mkfs_cmd());
        mkfs.arg(DEVICE);
        info!("creating fs: {}", format!("{:?}", mkfs));
        mkfs.output()?;

        let mut mount = Command::new("mount");
        mount.arg("-t").arg(Self::mount_t()).arg(DEVICE).arg(path);
        info!("mounting fs: {}", format!("{:?}", mount));
        mount.output()?;

        Ok(())
    }

    fn teardown(&self, path: &Path) -> io::Result<()> {
        info!("tearing down '{}' filesystem at '{}'", self, path.display());

        let mut umount = Command::new("umount");
        umount.arg("-fl").arg(path);
        info!("unmounting fs: {}", format!("{:?}", umount));
        umount.output()?;

        let mut rmmod = Command::new("rmmod");
        rmmod.arg("brd").output()?;
        info!(
            "removing block ram device module: {}",
            format!("{:?}", rmmod)
        );
        rmmod.output()?;

        info!("removing mountpoint at '{}'", path.display());
        fs::remove_dir_all(path)?;

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
