use std::path::Path;

use crate::{
    abstract_fs::types::{ConsolePipe, Workload},
    harness::{harness, HarnessError},
    mount::mount::FileSystemMount,
};

pub struct Harness<T: FileSystemMount> {
    fs_mount: T,
    fs_dir: Box<Path>,
    test_dir: Box<Path>,
    exec_dir: Box<Path>,
    stdout: ConsolePipe,
    stderr: ConsolePipe,
}

impl<T: FileSystemMount> Harness<T> {
    pub fn new(
        fs_mount: T,
        fs_dir: Box<Path>,
        test_dir: Box<Path>,
        exec_dir: Box<Path>,
        stdout: ConsolePipe,
        stderr: ConsolePipe,
    ) -> Self {
        Self {
            fs_mount,
            fs_dir,
            test_dir,
            exec_dir,
            stdout,
            stderr,
        }
    }
    pub fn run(&self, input: &Workload) -> Result<bool, HarnessError> {
        harness(
            input,
            &self.fs_mount,
            &self.fs_dir,
            &self.test_dir,
            &self.exec_dir,
            self.stdout.clone(),
            self.stderr.clone(),
        )
    }
}
