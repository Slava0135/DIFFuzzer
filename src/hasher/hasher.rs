use std::collections::HashSet;
use std::hash::Hasher;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use walkdir::WalkDir;
use std::sync::OnceLock;
use twox_hash::XxHash64;


// if nlink = True, include nlink to hash. Same for mode.
pub fn get_hash_for_dir(path: &Path, seed: u64, nlink: bool, mode: bool) -> u64 {
    let mut hasher = XxHash64::with_seed(seed);

    for entry in WalkDir::new(path).sort_by(|a, b| a.file_name().cmp(b.file_name())) {
        let entry = entry.unwrap();
        let strip_path= entry.path().strip_prefix(path).unwrap().to_str().unwrap();

        if is_nfs_tmp(strip_path) { continue; }

        let metadata = entry.metadata().unwrap();
        hasher.write(strip_path.as_bytes());
        hasher.write(&metadata.gid().to_le_bytes());
        hasher.write(&metadata.uid().to_le_bytes());
        if nlink { hasher.write(&metadata.nlink().to_le_bytes()); }
        if mode { hasher.write(&metadata.mode().to_le_bytes()); }

        entry.metadata().unwrap();
    }

    return hasher.finish();
}


fn get_ntfs_internal_dirs() -> &'static HashSet<&'static str> {
    static HASHSET: OnceLock<HashSet<&str>> = OnceLock::new();
    HASHSET.get_or_init(|| {
        let mut m = HashSet::new();
        m.insert("/lost+found");
        m.insert("/.nilfs");
        m.insert("/.mcfs_dummy");
        m
    })
}


fn is_nfs_tmp(path: &str) -> bool {
    return get_ntfs_internal_dirs().contains(path) || path.starts_with("/.nfs");
}