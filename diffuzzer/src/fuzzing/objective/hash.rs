/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use log::debug;
use regex::RegexSet;

use crate::path::RemotePath;

use hasher::{calc_dir_hash, get_diff, FileDiff, FileInfo, HasherOptions};

pub struct HashHolder {
    fs_dir: RemotePath,
    fs_internal: RegexSet,
    fs_content: Vec<FileInfo>,
    hash: u64,
    hasher_options: HasherOptions,
}

impl HashHolder {
    pub fn calc_and_save_hash(&mut self) {
        let (hash, fs_content) =
            calc_dir_hash(&self.fs_dir.base, &self.fs_internal, &self.hasher_options);
        self.fs_content = fs_content;
        self.hash = hash;
    }
}

pub struct HashObjective {
    pub fst_fs: HashHolder,
    pub snd_fs: HashHolder,
    enabled: bool,
}

impl HashObjective {
    pub fn new(
        fst_fs_dir: RemotePath,
        snd_fs_dir: RemotePath,
        fst_fs_internal: RegexSet,
        snd_fs_internal: RegexSet,
        enabled: bool,
    ) -> Self {
        Self {
            fst_fs: HashHolder {
                fs_dir: fst_fs_dir,
                fs_internal: fst_fs_internal,
                fs_content: vec![],
                hash: 0,
                hasher_options: Default::default(),
            },
            snd_fs: HashHolder {
                fs_dir: snd_fs_dir,
                fs_internal: snd_fs_internal,
                fs_content: vec![],
                hash: 0,
                hasher_options: Default::default(),
            },
            enabled,
        }
    }

    pub fn is_interesting(&self) -> anyhow::Result<bool> {
        debug!("do hash objective");
        if !self.enabled {
            return Ok(false);
        }

        Ok(self.fst_fs.hash != self.snd_fs.hash)
    }

    pub fn get_diff(&mut self) -> Vec<FileDiff> {
        get_diff(
            &self.fst_fs.fs_content,
            &self.snd_fs.fs_content,
            &self.fst_fs.fs_internal,
            &self.snd_fs.fs_internal,
            &self.fst_fs.hasher_options,
        )
    }
}
