use std::{path::Path, process::Command};

use libafl::executors::ExitKind;
use log::{debug, error};

use crate::{abstract_fs::types::Workload, mount::mount::FileSystemMount};

pub fn workload_harness<T: FileSystemMount>(
    fs_mount: T,
    fs_dir: Box<Path>,
    test_dir: Box<Path>,
    exec_dir: Box<Path>,
) -> impl Fn(&Workload) -> ExitKind {
    return move |input: &Workload| match harness(&input, &fs_mount, &fs_dir, &test_dir, &exec_dir) {
        Ok(exit) => exit,
        Err(err) => {
            error!("{err:?}");
            panic!("{err:?}");
        }
    };
}

fn harness<T: FileSystemMount>(
    input: &Workload,
    fs_mount: &T,
    fs_dir: &Path,
    test_dir: &Path,
    exec_dir: &Path,
) -> Result<ExitKind, libafl::Error> {
    debug!("executing harness");
    debug!("compiling test at '{}'", test_dir.display());
    let test_exec = input.compile(&test_dir)?;

    debug!(
        "setting up executable directory at '{}'",
        exec_dir.display()
    );
    std::fs::remove_dir_all(exec_dir).unwrap_or(());
    std::fs::create_dir(exec_dir)?;
    let test_exec_copy = exec_dir.join("test.out");
    std::fs::copy(test_exec, test_exec_copy.clone())?;

    fs_mount.setup(&fs_dir)?;

    let mut exec = Command::new(test_exec_copy);
    exec.arg(fs_dir);
    exec.current_dir(exec_dir);
    debug!("running test executable '{:?}'", exec);
    let output = exec.output()?;

    fs_mount.teardown(&fs_dir)?;

    if output.status.success() {
        Ok(ExitKind::Ok)
    } else {
        Ok(ExitKind::Crash)
    }
}
