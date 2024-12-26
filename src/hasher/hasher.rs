use std::collections::HashSet;
use std::hash::Hasher;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use walkdir::WalkDir;

use twox_hash::XxHash64;



// if nlink = True, include nlink to hash. Same for mode.
pub fn get_hash_for_dir(path: &Path, seed: u64, nlink: bool, mode: bool) -> u64 {
    let mut hasher = XxHash64::with_seed(seed);

    for entry in WalkDir::new(path).sort_by(|a, b| a.file_name().cmp(b.file_name())) {
        let entry = entry.unwrap();

        //todo: skip nfs temp files
        let metadata = entry.metadata().unwrap();

        hasher.write(entry.path().strip_prefix(path).unwrap().to_str().unwrap().as_bytes());
        hasher.write(&metadata.gid().to_le_bytes());
        hasher.write(&metadata.uid().to_le_bytes());
        if nlink { hasher.write(&metadata.nlink().to_le_bytes()); }
        if mode { hasher.write(&metadata.mode().to_le_bytes()); }

        entry.metadata().unwrap();
    }

    return hasher.finish();
}

//todo: check valid
const NTFS_INTERNAL_DIRS: HashSet<&str> = HashSet::from(["/lost+found", "/.nilfs", "/.mcfs_dummy"]);

fn is_nfs_tmp(path: &Path) -> bool {
    return NTFS_INTERNAL_DIRS.contains(path.to_str().unwrap());
}

/*
std::unordered_set<std::string> exclusion_list = {
        {"/lost+found"},
        {"/.nilfs"},
        {"/.mcfs_dummy"},
        {"/build"}
};

/* Also ignore NFS temp files "/.nfsXXXX" */
static inline bool is_excluded(const std::string &path) {
    return (exclusion_list.find(path) != exclusion_list.end() || path.rfind("./nfs", 0) == 0);
}

*/