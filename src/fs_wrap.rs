use std::{fmt::Display, fs, io, path::Path, process::Command};

use log::info;

const RAM_DISK_SIZE: usize = 1_000_000;
const DEVICE: &str = "/dev/ram0";

pub enum FileSystemType {
    EXT4,
    BTRFS,
}

impl Display for FileSystemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileSystemType::EXT4 => write!(f, "ext4"),
            FileSystemType::BTRFS => write!(f, "btrfs"),
        }
    }
}

pub fn setup(path: &Path, fs: FileSystemType) -> io::Result<()> {
    info!("setting up '{}' filesystem at '{}'", fs, path.display());

    let mkfs_cmd = match fs {
        FileSystemType::EXT4 => "mkfs.ext4".to_owned(),
        FileSystemType::BTRFS => "mkfs.btrfs".to_owned(),
    };

    let mount_t = match fs {
        FileSystemType::EXT4 => "ext4".to_owned(),
        FileSystemType::BTRFS => "btrfs".to_owned(),
    };

    info!("creating mountpoint at '{}'", path.display());
    fs::create_dir_all(path)?;

    let mut modprobe = Command::new("modprobe");
    modprobe
        .arg("brd")
        .arg("rd_nr=1")
        .arg(format!("rd_size={RAM_DISK_SIZE}"));
    info!("loading block ram device module: {}", format!("{:?}", modprobe));
    modprobe.output()?;

    let mut mkfs = Command::new(mkfs_cmd);
    mkfs.arg(DEVICE);
    info!("creating fs: {}", format!("{:?}", mkfs));
    mkfs.output()?;

    let mut mount = Command::new("mount");
    mount.arg("-t").arg(mount_t).arg(DEVICE).arg(path);
    info!("mounting fs: {}", format!("{:?}", mount));
    mount.output()?;

    Ok(())
}

pub fn teardown(path: &Path, fs: FileSystemType) -> io::Result<()> {
    info!("tearing down '{}' filesystem at '{}'", fs, path.display());

    let mut umount = Command::new("umount");
    umount.arg("-fl").arg(path);
    info!("unmounting fs: {}", format!("{:?}", umount));
    umount.output()?;

    let mut rmmod = Command::new("rmmod");
    rmmod.arg("brd").output()?;
    info!("removing block ram device module: {}", format!("{:?}", rmmod));
    rmmod.output()?;

    info!("removing mountpoint at '{}'", path.display());
    fs::remove_dir_all(path)?;

    Ok(())
}
