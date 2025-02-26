/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use anyhow::{Context, bail};

use crate::command::{CommandInterface, CommandWrapper, ExecError};
use crate::fuzzing::objective::Dash::{DashHolder, DashProducer};
use crate::mount::FileSystemMount;
use crate::path::{LocalPath, RemotePath};
use crate::supervisor::Supervisor;

use super::outcome::{Completed, Outcome};

pub struct Harness {
    fs_mount: &'static dyn FileSystemMount,
    fs_dir: RemotePath,
    exec_dir: RemotePath,
    outcome_dir: LocalPath,
    timeout: u8,
}

impl Harness {
    pub fn new(
        fs_mount: &'static dyn FileSystemMount,
        fs_dir: RemotePath,
        exec_dir: RemotePath,
        outcome_dir: LocalPath,
        timeout: u8,
    ) -> Self {
        Self {
            fs_mount,
            fs_dir,
            exec_dir,
            outcome_dir,
            timeout,
        }
    }
    pub fn run<C: FnMut(&dyn CommandInterface) -> anyhow::Result<()>>(
        &self,
        cmdi: &dyn CommandInterface,
        binary_path: &RemotePath,
        keep_fs: bool,
        supervisor: &mut dyn Supervisor,
        mut completion_callback: C,
    ) -> anyhow::Result<Outcome> {
        supervisor.reset_events()?;

        self.fs_mount.setup(cmdi, &self.fs_dir).with_context(|| {
            format!(
                "failed to setup fs '{}' at '{}'",
                self.fs_mount, self.fs_dir
            )
        })?;

        let mut exec = CommandWrapper::new(binary_path.base.as_ref());
        exec.arg(self.fs_dir.base.as_ref());

        let output = cmdi.exec_in_dir(exec, &self.exec_dir, Some(self.timeout));

        match output {
            Ok(output) => {
                let dash_holder = completion_callback(cmdi)
                    .with_context(|| "completion callback failed")?;

                if !keep_fs {
                    self.teardown(cmdi)?;
                }

                let stdout = String::from_utf8(output.stdout)
                    .with_context(|| "failed to convert stdout to string")?;
                let stderr = String::from_utf8(output.stderr)
                    .with_context(|| "failed to convert stderr to string")?;

                cmdi.copy_dir_from_remote(&self.exec_dir, &self.outcome_dir)
                    .with_context(|| "failed to copy test output files")?;

                Ok(Outcome::Completed(Completed::new(
                    stdout,
                    stderr,
                    self.outcome_dir.clone(),
                    dash_holder,
                )))
            }
            Err(ExecError::TimedOut(_)) => {
                if supervisor.had_panic_event()? {
                    Ok(Outcome::Panicked)
                } else {
                    if !keep_fs {
                        self.teardown(cmdi)?;
                    }
                    Ok(Outcome::TimedOut)
                }
            }
            Err(ExecError::IoError(msg)) => {
                bail!("failed to run test binary: {}", msg);
            }
        }
    }
    fn teardown(&self, cmdi: &dyn CommandInterface) -> anyhow::Result<()> {
        self.fs_mount.teardown(cmdi, &self.fs_dir).with_context(|| {
            format!(
                "failed to teardown fs '{}' at '{}'",
                self.fs_mount, self.fs_dir
            )
        })
    }
}
