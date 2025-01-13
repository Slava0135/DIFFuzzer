use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::hash::Hasher;
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::OnceLock;

use rand::random;
use twox_hash::XxHash64;
use walkdir::WalkDir;

struct FileInfo {
    abs_path: String,
    rel_path: String,

    gid: u32,
    uid: u32,
    size: u64,
    nlink: u64,
    mode: u32,
}

impl Display for FileInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "abs_path: {}\ngid: {}\n, uid: {}\n, size:{}\n, nlink: {}\n, mode: {}\n",
            self.abs_path, self.gid, self.uid, self.size, self.nlink, self.mode
        )
    }
}

// if nlink = True, include nlink to hash. Same for mode.
pub fn get_hash_for_dir(path: &Path, seed: u64, nlink: bool, mode: bool) -> u64 {
    let mut hasher = XxHash64::with_seed(seed);

    for entry in WalkDir::new(path).sort_by(|a, b| a.file_name().cmp(b.file_name())) {
        let entry = entry.unwrap();
        let rel_path = entry.path().strip_prefix(path).unwrap().to_str().unwrap();

        if is_nfs_tmp(rel_path) {
            continue;
        }

        let metadata = entry.metadata().unwrap();
        hasher.write(rel_path.as_bytes());
        hasher.write(&metadata.gid().to_le_bytes());
        hasher.write(&metadata.uid().to_le_bytes());
        hasher.write(&metadata.size().to_le_bytes());
        if nlink {
            hasher.write(&metadata.nlink().to_le_bytes());
        }
        if mode {
            hasher.write(&metadata.mode().to_le_bytes());
        }
    }

    return hasher.finish();
}

pub fn get_diff<T: Write>(path_a: &Path, path_b: &Path, mut output: T, nlink: bool, mode: bool) {
    let vec_a = get_dir_content(path_a);
    let vec_b = get_dir_content(path_b);
    let mut i_a = vec_a.len() - 1;
    let mut i_b = vec_b.len() - 1;

    // break when iterated over all elements in at least one directory
    loop {
        let cmp_res = vec_a[i_a].rel_path.cmp(&vec_b[i_b].rel_path);
        match cmp_res {
            Ordering::Equal => {
                let seed = random();
                let hash_a = get_hash_for_dir(vec_a[i_a].abs_path.as_ref(), seed, nlink, mode);
                let hash_b = get_hash_for_dir(vec_b[i_b].abs_path.as_ref(), seed, nlink, mode);
                if hash_a != hash_b {
                    write!(output, "========Diff hash for files:========\n")
                        .expect("panic at write msg");
                    write!(output, "{}\n", vec_a[i_a].to_string()).expect("panic at write a");
                    write!(output, "{}\n", vec_b[i_b].to_string()).expect("panic at write b");
                }
                if i_a == 0 || i_b == 0 {
                    break;
                }
                i_a -= 1;
                i_b -= 1;
            }
            Ordering::Greater => {
                write!(output, "File exist only in 1st seq:\n").expect("panic at write greater a");
                write!(output, "{}\n", vec_a[i_a].to_string()).expect("panic at write greater a");
                if i_a == 0 {
                    break;
                }
                i_a -= 1;
            }
            Ordering::Less => {
                write!(output, "File exist only in 2nd seq:\n").expect("panic at write greater b");
                write!(output, "{}\n", vec_b[i_b].to_string()).expect("panic at write greater b");
                if i_b == 0 {
                    break;
                }
                i_b -= 1;
            }
        }
    }
    if i_a > 0 {
        loop {
            write!(output, "File exist only in 1st seq:\n").expect("panic at write greater a");
            write!(output, "{}\n", vec_a[i_a].to_string()).expect("panic at write greater a");
            if i_a == 0 {
                break;
            }
            i_a -= 1;
        }
    }

    if i_b > 0 {
        loop {
            write!(output, "File exist only in 2nd seq:\n").expect("panic at write greater b");
            write!(output, "{}\n", vec_b[i_b].to_string()).expect("panic at write greater b");
            if i_b == 0 {
                break;
            }
            i_b -= 1;
        }
    }
}

fn get_dir_content(path: &Path) -> Vec<FileInfo> {
    let mut v = Vec::new();
    for entry in WalkDir::new(path).sort_by(|a, b| a.file_name().cmp(b.file_name())) {
        let entry = entry.unwrap();
        let rel_path = entry
            .path()
            .strip_prefix(path)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();

        let metadata = entry.metadata().unwrap();

        v.push(FileInfo {
            abs_path: entry.path().to_str().unwrap().to_owned(),
            rel_path,
            gid: metadata.gid(),
            uid: metadata.uid(),
            size: metadata.size(),
            nlink: metadata.nlink(),
            mode: metadata.mode(),
        });
    }
    return v;
}

fn get_nfs_internal_dirs() -> &'static HashSet<&'static str> {
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
