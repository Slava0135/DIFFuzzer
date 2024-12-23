use std::path::Path;

use crate::{abstract_fs::types::ConsolePipe, harness::harness, mount::mount::FileSystemMount};

pub struct Harness<T: FileSystemMount> {
    fs_mount: T,
    fs_dir: Box<Path>,
    exec_dir: Box<Path>,
    stdout: ConsolePipe,
    stderr: ConsolePipe,
}

impl<T: FileSystemMount> Harness<T> {
    pub fn new(
        fs_mount: T,
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
    pub fn run(&self, input_path: &Path) -> anyhow::Result<bool> {
        harness(
            input_path,
            &self.fs_mount,
            &self.fs_dir,
            &self.exec_dir,
            self.stdout.clone(),
            self.stderr.clone(),
        )
    }
}
