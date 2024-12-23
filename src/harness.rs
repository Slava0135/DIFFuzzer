use std::{path::Path, process::Command};

use log::debug;

use crate::abstract_fs::types::ConsolePipe;
use crate::mount::mount::FileSystemMount;

pub fn harness<T: FileSystemMount>(
    input_path: &Path,
    fs_mount: &T,
    fs_dir: &Path,
    exec_dir: &Path,
    stdout: ConsolePipe,
    stderr: ConsolePipe,
) -> anyhow::Result<bool> {
    let test_exec_copy = exec_dir.join("test.out");
    std::fs::copy(input_path, test_exec_copy.clone());

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
