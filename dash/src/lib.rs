/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::hash::Hasher;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use crate::FileDiff::{FileIsDifferent, OnlyOneExists};
use anyhow::{Context, Ok};
use regex::RegexSet;
use serde::{Deserialize, Serialize};
use twox_hash::XxHash64;
use walkdir::WalkDir;

pub const DIFF_FILENAME: &str = "dash-diff.txt";

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileInfo {
    /// Absolute file path (with mount `/mnt/...` prefix)
    abs_path: String,
    /// Path relative to file system root
    rel_path: String,
    /// Group ID of the owner
    gid: u32,
    /// User ID of the owner
    uid: u32,
    /// Total size of file in bytes
    size: u64,
    /// Number of hard links pointing to file
    nlink: u64,
    /// Rights applied to file
    mode: u32,

    is_dir: bool,
}

impl FileInfo {
    fn add_to_hasher(&self, hasher: &mut dyn Hasher, hasher_options: &HasherOptions) {
        hasher.write(self.rel_path.as_bytes());
        hasher.write_u32(self.gid);
        hasher.write_u32(self.uid);
        if hasher_options.size {
            hasher.write_u64(self.size);
        }
        if self.is_dir && hasher_options.dir_nlink || !self.is_dir && hasher_options.file_nlink {
            hasher.write_u64(self.nlink);
        }
        if hasher_options.mode {
            hasher.write_u32(self.mode);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FileDiff {
    FileIsDifferent { fst: FileInfo, snd: FileInfo },
    OnlyOneExists(FileInfo),
}

/// Options to include fields from [FileInfo] when calculating hash
#[derive(Default)]
pub struct HasherOptions {
    pub size: bool,
    pub file_nlink: bool,
    pub dir_nlink: bool,
    pub mode: bool,
}

impl Display for FileInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn calc_dir_hash(
    path: &Path,
    skip: &RegexSet,
    hasher_options: &HasherOptions,
) -> anyhow::Result<(u64, Vec<FileInfo>)> {
    let mut hasher = XxHash64::default();
    let mut res: Vec<FileInfo> = Vec::new();

    for entry in WalkDir::new(path).sort_by(|a, b| a.file_name().cmp(b.file_name())) {
        let entry = entry.with_context(|| "failed to get directory entry")?;
        let rel_path = entry
            .path()
            .strip_prefix(path)
            .unwrap()
            .to_string_lossy()
            .into_owned();

        if skip.is_match(&rel_path) {
            continue;
        }

        let metadata = entry
            .metadata()
            .with_context(|| "failed to get entry metadata")?;
        let file_info = FileInfo {
            abs_path: entry.path().to_string_lossy().into_owned(),
            rel_path: rel_path.to_owned(),
            gid: metadata.gid(),
            uid: metadata.uid(),
            size: metadata.size(),
            nlink: metadata.nlink(),
            mode: metadata.mode(),
            is_dir: metadata.is_dir()
        };
        file_info.add_to_hasher(&mut hasher, hasher_options);
        res.push(file_info);
    }

    Ok((hasher.finish(), res))
}

pub fn calc_fileinfo_hash(
    vec: &Vec<FileInfo>,
    rel_path: &str,
    hasher_options: &HasherOptions,
) -> u64 {
    let mut hasher = XxHash64::default();
    for file_info in vec {
        if file_info.rel_path.starts_with(rel_path) {
            file_info.add_to_hasher(&mut hasher, hasher_options);
        }
    }
    hasher.finish()
}

pub fn get_diff(
    vec_fst: &Vec<FileInfo>,
    vec_snd: &Vec<FileInfo>,
    fst_skip: &RegexSet,
    snd_skip: &RegexSet,
    hasher_options: &HasherOptions,
) -> Vec<FileDiff> {
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
                    calc_fileinfo_hash(vec_fst, &vec_fst[i_fst].rel_path, hasher_options);
                let hash_snd =
                    calc_fileinfo_hash(vec_snd, &vec_snd[i_snd].rel_path, hasher_options);
                if hash_fst != hash_snd {
                    res.push(FileIsDifferent {
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
                res.push(OnlyOneExists(vec_fst[i_fst].clone()));
                if i_fst == 0 {
                    break;
                }
                i_fst -= 1;
            }
            Ordering::Less => {
                res.push(OnlyOneExists(vec_snd[i_snd].clone()));
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

fn handle_last_diff(mut i: usize, vec_data: &[FileInfo], res: &mut Vec<FileDiff>) {
    if i > 0 {
        loop {
            res.push(OnlyOneExists(vec_data[i].clone()));
            if i == 0 {
                break;
            }
            i -= 1;
        }
    }
}
