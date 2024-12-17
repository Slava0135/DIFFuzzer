use std::io;
use std::string::FromUtf8Error;
use std::{path::Path, process::Command};

use log::debug;

use crate::abstract_fs::types::ConsolePipe;
use crate::{abstract_fs::types::Workload, mount::mount::FileSystemMount};

#[derive(Debug)]
pub enum HarnessError {
    IOError(io::Error),
    FromUtf8Error(FromUtf8Error),
}

impl From<io::Error> for HarnessError {
    fn from(value: io::Error) -> Self {
        HarnessError::IOError(value)
    }
}

impl From<FromUtf8Error> for HarnessError {
    fn from(value: FromUtf8Error) -> Self {
        HarnessError::FromUtf8Error(value)
    }
}

pub fn harness<T: FileSystemMount>(
    input: &Workload,
    fs_mount: &T,
    fs_dir: &Path,
    test_dir: &Path,
    exec_dir: &Path,
    stdout: ConsolePipe,
    stderr: ConsolePipe,
) -> Result<bool, HarnessError> {
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

    stdout.replace(String::from_utf8(output.stdout)?);
    stderr.replace(String::from_utf8(output.stderr)?);

    Ok(output.status.success())
}
