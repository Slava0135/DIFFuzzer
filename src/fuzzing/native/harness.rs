use std::{cell::RefCell, rc::Rc};

use anyhow::Context;

use crate::command::{CommandInterface, CommandWrapper};
use crate::fuzzing::native::objective::hash::HashHolder;
use crate::mount::mount::FileSystemMount;
use crate::path::RemotePath;

pub type ConsolePipe = Rc<RefCell<String>>;

pub struct Harness {
    fs_mount: &'static dyn FileSystemMount,
    fs_dir: RemotePath,
    exec_dir: RemotePath,
    stdout: ConsolePipe,
    stderr: ConsolePipe,
}

impl Harness {
    pub fn new(
        fs_mount: &'static dyn FileSystemMount,
        fs_dir: RemotePath,
        exec_dir: RemotePath,
        stdout: ConsolePipe,
        stderr: ConsolePipe,
    ) -> Self {
        Self {
            fs_mount,
            fs_dir,
            exec_dir,
            stdout,
            stderr,
        }
    }
    pub fn run(
        &self,
        cmdi: &dyn CommandInterface,
        binary_path: &RemotePath,
        keep_fs: bool,
        hash_holder: Option<&mut HashHolder>,
    ) -> anyhow::Result<bool> {
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
            self.teardown(cmdi)?;
        }

        self.stdout.replace(
            String::from_utf8(output.stdout)
                .with_context(|| format!("failed to convert stdout to string"))?,
        );
        self.stderr.replace(
            String::from_utf8(output.stderr)
                .with_context(|| format!("failed to convert stderr to string"))?,
        );

        Ok(output.status.success())
    }

    pub fn teardown(&self, cmdi: &dyn CommandInterface) -> anyhow::Result<()> {
        self.fs_mount
            .teardown(cmdi, &self.fs_dir)
            .with_context(|| {
                format!(
                    "failed to teardown fs '{}' at '{}'",
                    self.fs_mount, self.fs_dir
                )
            })?;
        Ok(())
    }
}
