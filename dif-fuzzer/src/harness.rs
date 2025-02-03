use std::{cell::RefCell, path::Path, process::Command, rc::Rc};

use anyhow::Context;

use crate::fuzzing::objective::hash::HashHolder;
use crate::mount::mount::FileSystemMount;

pub type ConsolePipe = Rc<RefCell<String>>;

pub struct Harness {
    fs_mount: &'static dyn FileSystemMount,
    fs_dir: Box<Path>,
    exec_dir: Box<Path>,
    stdout: ConsolePipe,
    stderr: ConsolePipe,
}

impl Harness {
    pub fn new(
        fs_mount: &'static dyn FileSystemMount,
        fs_dir: Box<Path>,
        exec_dir: Box<Path>,
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
        input_path: &Path,
        keep_fs: bool,
        hash_holder: Option<&mut HashHolder>,
    ) -> anyhow::Result<bool> {
        let test_exec_copy = self.exec_dir.join("test.out");
        std::fs::copy(input_path, &test_exec_copy).with_context(|| {
            format!(
                "failed to copy executable from '{}' to '{}'",
                input_path.display(),
                test_exec_copy.display()
            )
        })?;

        self.fs_mount.setup(&self.fs_dir).with_context(|| {
            format!(
                "failed to setup fs '{}' at '{}'",
                self.fs_mount,
                self.fs_dir.display()
            )
        })?;

        let mut exec = Command::new(test_exec_copy);
        exec.arg(self.fs_dir.as_os_str());
        exec.current_dir(&self.exec_dir);
        let output = exec
            .output()
            .with_context(|| format!("failed to run executable '{:?}'", exec))?;

        match hash_holder {
            Some(holder) => { holder.calc_and_save_hash() }
            _ => {}
        }

        if !keep_fs {
            self.teardown()?;
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

    pub fn teardown(&self) -> anyhow::Result<()> {
        self.fs_mount.teardown(&self.fs_dir).with_context(|| {
            format!(
                "failed to teardown fs '{}' at '{}'",
                self.fs_mount,
                self.fs_dir.display()
            )
        })?;
        Ok(())
    }
}
