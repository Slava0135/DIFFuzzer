use crate::mount::{btrfs::Btrfs, ext4::Ext4, f2fs::F2FS, mount::FileSystemMount};

pub const FILESYSTEMS: &[&dyn FileSystemMount] = &[&Ext4 {}, &Btrfs {}, &F2FS {}];

pub fn filesystem_available() -> Vec<String> {
    FILESYSTEMS
        .iter()
        .map(|fs| fs.to_string().to_lowercase())
        .collect()
}

impl TryFrom<String> for &'static dyn FileSystemMount {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value = value.to_lowercase();
        for fs in FILESYSTEMS {
            if fs.to_string().to_lowercase() == value {
                return Ok(*fs);
            }
        }
        Err(format!("unknown filesystem '{}'", value))
    }
}
