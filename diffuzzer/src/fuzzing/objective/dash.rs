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

#[derive(Default)]
pub struct DashState {
    pub fs_state: Vec<FileInfo>,
    pub hash: u64,
}

struct DashProducer {
    fs_dir: RemotePath,
    fs_internal: RegexSet,
}

impl DashProducer {
    pub fn calculate(
        &mut self,
        cmdi: &dyn CommandInterface,
        dash_path: &RemotePath,
        output_path: &RemotePath,
    ) -> anyhow::Result<DashState> {
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
        let fs_state =
            serde_json::from_str(&fs_state).with_context(|| "failed to parse Dash output file")?;

        Ok(DashState { fs_state, hash })
    }
}

pub struct DashObjective {
    fst: DashProducer,
    snd: DashProducer,
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
            fst: DashProducer {
                fs_dir: fst_fs_dir,
                fs_internal: fst_fs_internal,
            },
            snd: DashProducer {
                fs_dir: snd_fs_dir,
                fs_internal: snd_fs_internal,
            },
            enabled: config.dash.enabled,
            hasher_options: Default::default(),
            dash_path,
            output_path,
        })
    }

    pub fn calculate_fst(&mut self, cmdi: &dyn CommandInterface) -> anyhow::Result<DashState> {
        if self.enabled {
            self.fst.calculate(cmdi, &self.dash_path, &self.output_path)
        } else {
            Ok(DashState::default())
        }
    }

    pub fn calculate_snd(&mut self, cmdi: &dyn CommandInterface) -> anyhow::Result<DashState> {
        if self.enabled {
            self.snd.calculate(cmdi, &self.dash_path, &self.output_path)
        } else {
            Ok(DashState::default())
        }
    }

    pub fn is_interesting(&self, fst: &DashState, snd: &DashState) -> anyhow::Result<bool> {
        debug!("do hash objective");
        if !self.enabled {
            return Ok(false);
        }

        Ok(fst.hash != snd.hash)
    }

    pub fn get_diff(&mut self, fst: &DashState, snd: &DashState) -> Vec<FileDiff> {
        get_diff(
            &fst.fs_state,
            &snd.fs_state,
            &self.fst.fs_internal,
            &self.snd.fs_internal,
            &self.hasher_options,
        )
    }
}
