use std::path::Path;

use log::debug;
use regex::RegexSet;

use crate::hasher::hasher::{calc_dir_hash, get_diff, FileDiff, FileInfo, HasherOptions};

pub struct HashObjective {
    fst_fs_dir: Box<Path>,
    snd_fs_dir: Box<Path>,
    fst_fs_internal: RegexSet,
    snd_fs_internal: RegexSet,
    hasher_options: HasherOptions,
    enabled: bool,
    fst_fs_content: Vec<FileInfo>,
    snd_fs_content: Vec<FileInfo>,
}

impl HashObjective {
    pub fn new(
        fst_fs_dir: Box<Path>,
        snd_fs_dir: Box<Path>,
        fst_fs_internal: RegexSet,
        snd_fs_internal: RegexSet,
        enabled: bool,
    ) -> Self {
        Self {
            fst_fs_dir,
            snd_fs_dir,
            fst_fs_internal,
            snd_fs_internal,
            hasher_options: Default::default(),
            enabled,
            fst_fs_content: vec![],
            snd_fs_content: vec![],
        }
    }

    pub fn is_interesting(&mut self) -> anyhow::Result<bool> {
        debug!("do hash objective");
        if !self.enabled {
            return Ok(false);
        }
        let (fst_hash, fst_fs_content) = calc_dir_hash(
            &self.fst_fs_dir,
            &self.fst_fs_internal,
            &self.hasher_options,
        );
        let (snd_hash, snd_fs_content) = calc_dir_hash(
            &self.snd_fs_dir,
            &self.snd_fs_internal,
            &self.hasher_options,
        );
        self.fst_fs_content = fst_fs_content;
        self.snd_fs_content = snd_fs_content;
        Ok(fst_hash != snd_hash)
    }

    pub fn get_diff(&mut self) -> Vec<FileDiff> {
        get_diff(
            &self.fst_fs_content,
            &self.snd_fs_content,
            &self.fst_fs_internal,
            &self.snd_fs_internal,
            &self.hasher_options,
        )
    }
}
