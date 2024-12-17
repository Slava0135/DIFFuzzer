use std::path::Path;

use libafl::executors::ExitKind;
use log::error;

use crate::{
    abstract_fs::types::{ConsolePipe, Workload},
    harness::harness,
    mount::mount::FileSystemMount,
};

pub fn workload_harness<T: FileSystemMount>(
    fs_mount: T,
    fs_dir: Box<Path>,
    test_dir: Box<Path>,
    exec_dir: Box<Path>,
    stdout: ConsolePipe,
    stderr: ConsolePipe,
) -> impl Fn(&Workload) -> ExitKind {
    return move |input: &Workload| match harness(
        &input,
        &fs_mount,
        &fs_dir,
        &test_dir,
        &exec_dir,
        stdout.clone(),
        stderr.clone(),
    ) {
        Ok(ok) => {
            if ok {
                ExitKind::Ok
            } else {
                ExitKind::Crash
            }
        }
        Err(err) => {
            error!("{err:?}");
            panic!("{err:?}");
        }
    };
}
