/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use anyhow::Context;
use log::debug;
use regex::RegexSet;

use crate::path::RemotePath;

use dash::{FileDiff, FileInfo, HasherOptions, calc_dir_hash, get_diff};

struct DashState {
    fs_dir: RemotePath,
    fs_internal: RegexSet,
    fs_content: Vec<FileInfo>,
    hash: u64,
}

impl DashState {
    pub fn update(&mut self, hasher_options: &HasherOptions) -> anyhow::Result<()> {
        let (hash, fs_content) =
            calc_dir_hash(&self.fs_dir.base, &self.fs_internal, hasher_options).with_context(
                || format!("failed to calculate directory hash at '{}'", self.fs_dir),
            )?;
        self.fs_content = fs_content;
        self.hash = hash;
        Ok(())
    }
}

pub struct DashObjective {
    fst: DashState,
    snd: DashState,
    enabled: bool,
    hasher_options: HasherOptions,
}

impl DashObjective {
    pub fn create(
        fst_fs_dir: RemotePath,
        snd_fs_dir: RemotePath,
        fst_fs_internal: RegexSet,
        snd_fs_internal: RegexSet,
        enabled: bool,
    ) -> Self {
        Self {
            fst: DashState {
                fs_dir: fst_fs_dir,
                fs_internal: fst_fs_internal,
                fs_content: vec![],
                hash: 0,
            },
            snd: DashState {
                fs_dir: snd_fs_dir,
                fs_internal: snd_fs_internal,
                fs_content: vec![],
                hash: 0,
            },
            enabled,
            hasher_options: Default::default(),
        }
    }

    pub fn update_first(&mut self) -> anyhow::Result<()> {
        if self.enabled {
            self.fst.update(&self.hasher_options)
        } else {
            Ok(())
        }
    }

    pub fn update_second(&mut self) -> anyhow::Result<()> {
        if self.enabled {
            self.snd.update(&self.hasher_options)
        } else {
            Ok(())
        }
    }

    pub fn is_interesting(&self) -> anyhow::Result<bool> {
        debug!("do hash objective");
        if self.enabled {
            Ok(self.fst.hash != self.snd.hash)
        } else {
            return Ok(false);
        }
    }

    pub fn get_diff(&mut self) -> Vec<FileDiff> {
        get_diff(
            &self.fst.fs_content,
            &self.snd.fs_content,
            &self.fst.fs_internal,
            &self.snd.fs_internal,
            &self.hasher_options,
        )
    }
}
