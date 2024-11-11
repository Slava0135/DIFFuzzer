use std::{fs, io, path::Path, process::Command};

const RAM_DISK_SIZE: usize = 1_000_000;
const DEVICE: &str = "/dev/ram0";

pub enum FileSystemType {
    EXT4,
    BTRFS,
}

pub fn setup(path: &Path, fs: FileSystemType) -> io::Result<()> {
    let mkfs_cmd = match fs {
        FileSystemType::EXT4 => "mkfs.ext4".to_owned(),
        FileSystemType::BTRFS => "mkfs.btrfs".to_owned(),
    };
    let mount_t = match fs {
        FileSystemType::EXT4 => "ext4".to_owned(),
        FileSystemType::BTRFS => "btrfs".to_owned(),
    };
    fs::create_dir_all(path)?;
    Command::new("modprobe")
        .arg("brd")
        .arg("rd_nr=1")
        .arg(format!("rd_size={RAM_DISK_SIZE}"))
        .output()?;
    Command::new(mkfs_cmd).arg(DEVICE).output()?;
    Command::new("mount")
        .arg("-t")
        .arg(mount_t)
        .arg(DEVICE)
        .arg(path)
        .output()?;
    Ok(())
}

pub fn teardown(path: &Path, _fs: FileSystemType) -> io::Result<()> {
    Command::new("umount").arg("-fl").arg(path).output()?;
    Command::new("rmmod").arg("brd").output()?;
    fs::remove_dir_all(path)?;
    Ok(())
}
