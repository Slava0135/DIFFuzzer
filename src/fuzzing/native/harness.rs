use std::{cell::RefCell, process::Command, rc::Rc};

use anyhow::Context;

use crate::command::CommandInterface;
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
        let binary_copy_path = self.exec_dir.join("test.out");
        todo!("use cmdi");
        std::fs::copy(binary_path.base.as_ref(), binary_copy_path.base.as_ref()).with_context(
            || {
                format!(
                    "failed to copy executable from '{}' to '{}'",
                    binary_path, binary_copy_path
                )
            },
        )?;

        self.fs_mount.setup(cmdi, &self.fs_dir).with_context(|| {
            format!(
                "failed to setup fs '{}' at '{}'",
                self.fs_mount, self.fs_dir
            )
        })?;

        let mut exec = Command::new(binary_copy_path.base.as_ref());
        exec.arg(self.fs_dir.base.as_ref());
        exec.current_dir(&self.exec_dir.base.as_ref());
        let output = exec
            .output()
            .with_context(|| format!("failed to run executable '{:?}'", exec))?;

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
