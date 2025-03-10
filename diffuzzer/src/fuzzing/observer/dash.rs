/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::path::Path;

use anyhow::Context;
use dash::FileInfo;
use log::warn;
use regex::RegexSet;

use crate::{
    command::{CommandInterface, CommandWrapper},
    config::Config,
    path::{LocalPath, RemotePath},
};

use super::Observer;

pub struct DashObserver {
    dash_path: Option<RemotePath>,
    fs_dir: RemotePath,
    fs_internal: RegexSet,

    hash: u64,
    fs_state: Vec<FileInfo>,
}

impl Observer for DashObserver {
    fn pre_exec(
        &mut self,
        _cmdi: &dyn CommandInterface,
        _output_dir: &RemotePath,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn post_exec(
        &mut self,
        cmdi: &dyn CommandInterface,
        output_dir: &RemotePath,
    ) -> anyhow::Result<()> {
        if let Some(dash_path) = &self.dash_path {
            let output_path = output_dir.join("dash");
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
            self.hash = hash
                .trim()
                .parse()
                .with_context(|| format!("failed to parse hash '{}'", hash))?;
            let fs_state = cmdi
                .read_to_string(&output_path)
                .with_context(|| format!("failed to read Dash output file at '{}'", output_path))?;
            self.fs_state = serde_json::from_str(&fs_state)
                .with_context(|| "failed to parse Dash output file")?;
        }
        Ok(())
    }

    fn skip_exec(&mut self) {}
}

impl DashObserver {
    pub fn create(
        config: &Config,
        cmdi: &dyn CommandInterface,
        fs_dir: RemotePath,
        fs_internal: RegexSet,
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
            Some(remote_path)
        } else {
            warn!("dash (differential abstract state hash) observer is disabled");
            None
        };
        Ok(Self {
            dash_path,
            fs_dir,
            fs_internal,
            fs_state: vec![],
            hash: 0,
        })
    }
    pub fn fs_internal(&self) -> &RegexSet {
        &self.fs_internal
    }
    pub fn fs_state(&self) -> &Vec<FileInfo> {
        &self.fs_state
    }
    pub fn hash(&self) -> u64 {
        self.hash
    }
}
