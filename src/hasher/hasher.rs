use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::hash::Hasher;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::OnceLock;

use crate::hasher::hasher::FileDiff::DifferentHash;
use crate::hasher::hasher::FileDiff::OneExists;
use rand::random;
use twox_hash::XxHash64;
use walkdir::WalkDir;

pub const DIFF_HASH_FILENAME: &str = "diff_hash.txt";

#[derive(Clone)]
pub struct FileInfo {
    abs_path: String,
    rel_path: String,

    gid: u32,
    uid: u32,
    size: u64,
    nlink: u64,
    mode: u32,
}

pub enum FileDiff {
    DifferentHash { fst: FileInfo, snd: FileInfo },
    OneExists(FileInfo),
}

pub struct HasherOptions {
    nlink: bool,
    mode: bool,
}

impl Default for HasherOptions {
    fn default() -> Self {
        Self {
            nlink: false,
            mode: false,
        }
    }
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
pub fn calc_hash_for_dir(path: &Path, seed: u64, hasher_options: &HasherOptions) -> u64 {
    let mut hasher = XxHash64::with_seed(seed);

    for entry in WalkDir::new(path).sort_by(|a, b| a.file_name().cmp(b.file_name())) {
        let entry = entry.unwrap();
        let rel_path = entry.path().strip_prefix(path).unwrap().to_str().unwrap();

        //todo: uncomment after adding nfs support
        // if is_nfs_tmp(rel_path) {
        //     continue;
        // }

        let metadata = entry.metadata().unwrap();
        hasher.write(rel_path.as_bytes());
        hasher.write_u32(metadata.gid());
        hasher.write_u32(metadata.uid());
        hasher.write_u64(metadata.size());
        if hasher_options.nlink {
            hasher.write(&metadata.nlink().to_le_bytes());
        }
        if hasher_options.mode {
            hasher.write(&metadata.mode().to_le_bytes());
        }
    }

    return hasher.finish();
}

pub fn get_diff(path_fst: &Path, path_snd: &Path, hasher_options: &HasherOptions) -> Vec<FileDiff> {
    let vec_fst = get_dir_content(path_fst);
    let vec_snd = get_dir_content(path_snd);
    let mut i_fst = vec_fst.len() - 1;
    let mut i_snd = vec_snd.len() - 1;
    let mut res: Vec<FileDiff> = Vec::new();
    // break when iterated over all elements in at least one directory
    loop {
        let cmp_res = vec_fst[i_fst].rel_path.cmp(&vec_snd[i_snd].rel_path);
        match cmp_res {
            Ordering::Equal => {
                let seed = random();
                let hash_fst =
                    calc_hash_for_dir(vec_fst[i_fst].abs_path.as_ref(), seed, &hasher_options);
                let hash_snd =
                    calc_hash_for_dir(vec_snd[i_snd].abs_path.as_ref(), seed, &hasher_options);
                if hash_fst != hash_snd {
                    res.push(DifferentHash {
                        fst: vec_fst[i_fst].clone(),
                        snd: vec_snd[i_snd].clone(),
                    });
                }
                if i_fst == 0 || i_snd == 0 {
                    break;
                }
                i_fst -= 1;
                i_snd -= 1;
            }
            Ordering::Greater => {
                res.push(OneExists(vec_fst[i_fst].clone()));
                if i_fst == 0 {
                    break;
                }
                i_fst -= 1;
            }
            Ordering::Less => {
                res.push(OneExists(vec_snd[i_snd].clone()));
                if i_snd == 0 {
                    break;
                }
                i_snd -= 1;
            }
        }
    }

    handle_last_diff(i_fst, vec_fst, &mut res);
    handle_last_diff(i_snd, vec_snd, &mut res);

    res
}

fn handle_last_diff(mut i: usize, vec_data: Vec<FileInfo>, res: &mut Vec<FileDiff>) {
    if i > 0 {
        loop {
            res.push(OneExists(vec_data[i].clone()));
            if i == 0 {
                break;
            }
            i -= 1;
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
    return get_nfs_internal_dirs().contains(path) || path.starts_with("/.nfs");
}
