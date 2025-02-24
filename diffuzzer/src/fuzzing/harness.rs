/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use anyhow::{Context, bail};

use crate::command::{CommandInterface, CommandWrapper, ExecError};
use crate::event::EventHandler;
use crate::fuzzing::objective::hash::HashHolder;
use crate::mount::FileSystemMount;
use crate::path::{LocalPath, RemotePath};

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
    pub fn run(
        &self,
        cmdi: &dyn CommandInterface,
        binary_path: &RemotePath,
        keep_fs: bool,
        hash_holder: Option<&mut HashHolder>,
        event_handler: Option<&mut EventHandler>,
    ) -> anyhow::Result<Outcome> {
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
                if let Some(holder) = hash_holder {
                    holder.calc_and_save_hash()?;
                }

                if !keep_fs {
                    self.teardown(cmdi)?;
                }

                let stdout = String::from_utf8(output.stdout)
                    .with_context(|| "failed to convert stdout to string")?;
                let stderr = String::from_utf8(output.stderr)
                    .with_context(|| "failed to convert stderr to string")?;

                cmdi.copy_dir_from_remote(&self.exec_dir, &self.outcome_dir)
                    .with_context(|| "failed to copy test output files")?;

                Ok(Outcome::Completed(Completed {
                    dir: self.outcome_dir.clone(),
                    stdout,
                    stderr,
                }))
            }
            Err(ExecError::TimedOut(msg)) => match event_handler {
                Some(event_handler) => {
                    if event_handler.panicked()? {
                        Ok(Outcome::Panicked)
                    } else {
                        if !keep_fs {
                            self.teardown(cmdi)?;
                        }
                        Ok(Outcome::TimedOut { msg })
                    }
                }
                None => {
                    if !keep_fs {
                        self.teardown(cmdi)?;
                    }
                    Ok(Outcome::TimedOut { msg })
                }
            },
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
