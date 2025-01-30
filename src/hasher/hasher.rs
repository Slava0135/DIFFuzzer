use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::hash::Hasher;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use regex::RegexSet;
use twox_hash::XxHash64;
use walkdir::WalkDir;

use crate::hasher::hasher::FileDiff::DifferentHash;
use crate::hasher::hasher::FileDiff::OneExists;

pub const DIFF_HASH_FILENAME: &str = "diff_hash.txt";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileInfo {
    abs_path: String,
    rel_path: String,

    gid: u32,
    uid: u32,
    size: u64,
    nlink: u64,
    mode: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileDiff {
    DifferentHash { fst: FileInfo, snd: FileInfo },
    OneExists(FileInfo),
}

pub struct HasherOptions {
    size: bool,
    nlink: bool,
    mode: bool,
}

impl Default for HasherOptions {
    fn default() -> Self {
        Self {
            size: false,
            nlink: false,
            mode: false,
        }
    }
}

impl Display for FileInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn calc_dir_hash(path: &Path, skip: &RegexSet, hasher_options: &HasherOptions) -> u64 {
    let mut hasher = XxHash64::default();

    for entry in WalkDir::new(path).sort_by(|a, b| a.file_name().cmp(b.file_name())) {
        let entry = entry.unwrap();
        let rel_path = entry.path().strip_prefix(path).unwrap().to_str().unwrap();

        if skip.is_match(rel_path) {
            continue;
        }

        let metadata = entry.metadata().unwrap();
        hasher.write(rel_path.as_bytes());
        hasher.write_u32(metadata.gid());
        hasher.write_u32(metadata.uid());
        if hasher_options.size {
            hasher.write_u64(metadata.size());
        }
        if hasher_options.nlink {
            hasher.write_u64(metadata.nlink());
        }
        if hasher_options.mode {
            hasher.write_u32(metadata.mode());
        }
    }

    return hasher.finish();
}

pub fn get_diff(
    path_fst: &Path,
    path_snd: &Path,
    fst_skip: &RegexSet,
    snd_skip: &RegexSet,
    hasher_options: &HasherOptions,
) -> Vec<FileDiff> {
    let vec_fst = get_dir_content(path_fst);
    let vec_snd = get_dir_content(path_snd);
    let mut i_fst = vec_fst.len() - 1;
    let mut i_snd = vec_snd.len() - 1;
    let mut res: Vec<FileDiff> = Vec::new();
    // break when iterated over all elements in at least one directory
    loop {
        if fst_skip.is_match(vec_fst[i_fst].rel_path.as_str()) {
            if i_fst == 0 {
                break;
            }
            i_fst -= 1;
            continue;
        }

        if snd_skip.is_match(vec_snd[i_snd].rel_path.as_str()) {
            if i_snd == 0 {
                break;
            }
            i_snd -= 1;
            continue;
        }

        let cmp_res = vec_fst[i_fst].rel_path.cmp(&vec_snd[i_snd].rel_path);
        match cmp_res {
            Ordering::Equal => {
                let hash_fst =
                    calc_dir_hash(vec_fst[i_fst].abs_path.as_ref(), fst_skip, &hasher_options);
                let hash_snd =
                    calc_dir_hash(vec_snd[i_snd].abs_path.as_ref(), snd_skip, &hasher_options);
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
