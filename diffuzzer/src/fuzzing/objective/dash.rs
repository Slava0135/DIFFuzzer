/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::path::Path;

use anyhow::Context;
use log::{debug, warn};
use regex::RegexSet;

use crate::{
    command::{CommandInterface, CommandWrapper},
    config::Config,
    path::{LocalPath, RemotePath},
};

use dash::{FileDiff, FileInfo, HasherOptions, get_diff};

struct DashState {
    fs_dir: RemotePath,
    fs_internal: RegexSet,
    fs_state: Vec<FileInfo>,
    hash: u64,
}

impl DashState {
    pub fn update(
        &mut self,
        cmdi: &dyn CommandInterface,
        dash_path: &RemotePath,
        output_path: &RemotePath,
    ) -> anyhow::Result<()> {
        let mut dash = CommandWrapper::new(dash_path.base.as_ref());
        dash.arg("-t").arg(self.fs_dir.base.as_ref());
        dash.arg("-o").arg(output_path.base.as_ref());
        for pat in self.fs_internal.patterns() {
            dash.arg("-e").arg(pat);
        }
        let output = cmdi
            .exec(dash, None)
            .with_context(|| "failed to execute Dash")?;
        let hash = String::from_utf8(output.stdout)
            .with_context(|| "failed to convert Dash stdout to string")?;
        let hash: u64 = hash
            .trim()
            .parse()
            .with_context(|| format!("failed to parse hash '{}'", hash))?;
        let fs_state = cmdi
            .read_to_string(output_path)
            .with_context(|| format!("failed to read Dash output file at '{}'", output_path))?;
        let fs_state = serde_json::from_str(&fs_state)
            .with_context(|| "failed to parse Dash output file")?;
        self.fs_state = fs_state;
        self.hash = hash;
        Ok(())
    }
}

pub struct DashObjective {
    fst: DashState,
    snd: DashState,
    enabled: bool,
    hasher_options: HasherOptions,
    dash_path: RemotePath,
    output_path: RemotePath,
}

impl DashObjective {
    pub fn create(
        cmdi: &dyn CommandInterface,
        fst_fs_dir: RemotePath,
        snd_fs_dir: RemotePath,
        fst_fs_internal: RegexSet,
        snd_fs_internal: RegexSet,
        config: &Config,
    ) -> anyhow::Result<Self> {
        let dash_path = if config.dash.enabled {
            let dash_binary_path = if cfg!(debug_assertions) {
                config.dash.debug_binary_path.clone()
            } else {
                config.dash.release_binary_path.clone()
            };
            let binary_path = LocalPath::new(Path::new(&dash_binary_path));
            let remote_path = RemotePath::new_tmp("dash");
            cmdi.copy_to_remote(&binary_path, &remote_path)
                .with_context(|| "failed to copy dash binary")?;
            remote_path
        } else {
            warn!("dash (differential abstract state hash) objective is disabled");
            RemotePath::new(Path::new(""))
        };
        let output_path = RemotePath::new(Path::new(&config.dash.output_path));
        Ok(Self {
            fst: DashState {
                fs_dir: fst_fs_dir,
                fs_internal: fst_fs_internal,
                fs_state: vec![],
                hash: 0,
            },
            snd: DashState {
                fs_dir: snd_fs_dir,
                fs_internal: snd_fs_internal,
                fs_state: vec![],
                hash: 0,
            },
            enabled: config.dash.enabled,
            hasher_options: Default::default(),
            dash_path,
            output_path,
        })
    }

    pub fn update_first(&mut self, cmdi: &dyn CommandInterface) -> anyhow::Result<()> {
        if self.enabled {
            self.fst
                .update(cmdi, &self.dash_path, &self.output_path)
                .with_context(|| "failed to update first dash state")
        } else {
            Ok(())
        }
    }

    pub fn update_second(&mut self, cmdi: &dyn CommandInterface) -> anyhow::Result<()> {
        if self.enabled {
            self.snd
                .update(cmdi, &self.dash_path, &self.output_path)
                .with_context(|| "failed to update second dash state")
        } else {
            Ok(())
        }
    }

    pub fn is_interesting(&self) -> anyhow::Result<bool> {
        debug!("do hash objective");
        if self.enabled {
            Ok(self.fst.hash != self.snd.hash)
        } else {
            Ok(false)
        }
    }

    pub fn get_diff(&mut self) -> Vec<FileDiff> {
        get_diff(
            &self.fst.fs_state,
            &self.snd.fs_state,
            &self.fst.fs_internal,
            &self.snd.fs_internal,
            &self.hasher_options,
        )
    }
}
