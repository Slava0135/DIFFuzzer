/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::{Context, bail};

use crate::command::{CommandInterface, CommandWrapper, ExecError};
use crate::mount::FileSystemMount;
use crate::path::{LocalPath, RemotePath};
use crate::supervisor::Supervisor;

use super::observer::Observer;
use super::outcome::{Completed, Outcome};

pub struct Harness {
    fs_mount: &'static dyn FileSystemMount,
    fs_dir: RemotePath,
    exec_dir: RemotePath,
    outcome_dir: LocalPath,
    timeout: u8,
    observers: Vec<Rc<RefCell<dyn Observer>>>,
}

impl Harness {
    pub fn new(
        fs_mount: &'static dyn FileSystemMount,
        fs_dir: RemotePath,
        exec_dir: RemotePath,
        outcome_dir: LocalPath,
        timeout: u8,
        observers: Vec<Rc<RefCell<dyn Observer>>>,
    ) -> Self {
        Self {
            fs_mount,
            fs_dir,
            exec_dir,
            outcome_dir,
            timeout,
            observers,
        }
    }
    pub fn run(
        &self,
        cmdi: &dyn CommandInterface,
        binary_path: &RemotePath,
        keep_fs: bool,
        supervisor: &mut dyn Supervisor,
    ) -> anyhow::Result<Outcome> {
        supervisor.reset_events()?;

        self.fs_mount.setup(cmdi, &self.fs_dir).with_context(|| {
            format!(
                "failed to setup fs '{}' at '{}'",
                self.fs_mount, self.fs_dir
            )
        })?;

        for observer in &self.observers {
            observer
                .borrow_mut()
                .pre_exec(cmdi, &self.exec_dir)
                .with_context(|| "failed to call observer pre-execution callback")?;
        }

        let mut exec = CommandWrapper::new(binary_path.base.as_ref());
        exec.arg(self.fs_dir.base.as_ref());

        let output = cmdi.exec_in_dir(exec, &self.exec_dir, Some(self.timeout));

        match output {
            Ok(output) => {
                for observer in &self.observers {
                    observer
                        .borrow_mut()
                        .post_exec(cmdi, &self.exec_dir)
                        .with_context(|| "failed to call observer post-execution callback")?;
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

                Ok(Outcome::Completed(Completed::new(
                    stdout,
                    stderr,
                    self.outcome_dir.clone(),
                )))
            }
            Err(ExecError::TimedOut(_)) => {
                for observer in &self.observers {
                    observer.borrow_mut().skip_exec();
                }
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
                for observer in &self.observers {
                    observer.borrow_mut().skip_exec();
                }
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
