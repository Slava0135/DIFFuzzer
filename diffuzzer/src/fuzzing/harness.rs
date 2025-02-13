/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use anyhow::Context;

use crate::command::{CommandInterface, CommandWrapper};
use crate::fuzzing::objective::hash::HashHolder;
use crate::mount::mount::FileSystemMount;
use crate::path::{LocalPath, RemotePath};

use super::outcome::Outcome;

pub struct Harness {
    fs_mount: &'static dyn FileSystemMount,
    fs_dir: RemotePath,
    exec_dir: RemotePath,
    outcome_dir: LocalPath,
}

impl Harness {
    pub fn new(
        fs_mount: &'static dyn FileSystemMount,
        fs_dir: RemotePath,
        exec_dir: RemotePath,
        outcome_dir: LocalPath,
    ) -> Self {
        Self {
            fs_mount,
            fs_dir,
            exec_dir,
            outcome_dir,
        }
    }
    pub fn run(
        &self,
        cmdi: &dyn CommandInterface,
        binary_path: &RemotePath,
        keep_fs: bool,
        hash_holder: Option<&mut HashHolder>,
    ) -> anyhow::Result<Outcome> {
        self.fs_mount.setup(cmdi, &self.fs_dir).with_context(|| {
            format!(
                "failed to setup fs '{}' at '{}'",
                self.fs_mount, self.fs_dir
            )
        })?;

        let mut exec = CommandWrapper::new(binary_path.base.as_ref());
        exec.arg(self.fs_dir.base.as_ref());
        let output = cmdi
            .exec_in_dir(exec, &self.exec_dir)
            .with_context(|| "failed to run test binary")?;

        match hash_holder {
            Some(holder) => holder.calc_and_save_hash(),
            _ => {}
        }

        if !keep_fs {
            self.fs_mount
                .teardown(cmdi, &self.fs_dir)
                .with_context(|| {
                    format!(
                        "failed to teardown fs '{}' at '{}'",
                        self.fs_mount, self.fs_dir
                    )
                })?;
        }

        let stdout = String::from_utf8(output.stdout)
            .with_context(|| format!("failed to convert stdout to string"))?;
        let stderr = String::from_utf8(output.stderr)
            .with_context(|| format!("failed to convert stderr to string"))?;

        cmdi.copy_dir_from_remote(&self.exec_dir, &self.outcome_dir)
            .with_context(|| format!("failed to copy test output files"))?;

        Ok(Outcome {
            exit_status: output.status,
            dir: self.outcome_dir.clone(),
            stdout,
            stderr,
        })
    }
}
