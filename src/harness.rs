use std::{path::Path, process::Command};

use anyhow::Context;
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
    std::fs::copy(input_path, &test_exec_copy).with_context(|| {
        format!(
            "failed to copy executable from '{}' to '{}'",
            input_path.display(),
            test_exec_copy.display()
        )
    })?;

    fs_mount.setup(&fs_dir).with_context(|| {
        format!(
            "failed to setup fs '{}' at '{}'",
            fs_mount,
            fs_dir.display()
        )
    })?;

    let mut exec = Command::new(test_exec_copy);
    exec.arg(fs_dir);
    exec.current_dir(exec_dir);
    debug!("running test executable '{:?}'", exec);
    let output = exec
        .output()
        .with_context(|| format!("failed to run executable"))?;

    fs_mount.teardown(&fs_dir).with_context(|| {
        format!(
            "failed to teardown fs '{}' at '{}'",
            fs_mount,
            fs_dir.display()
        )
    })?;

    stdout.replace(
        String::from_utf8(output.stdout)
            .with_context(|| format!("failed to convert stdout to string"))?,
    );
    stderr.replace(
        String::from_utf8(output.stderr)
            .with_context(|| format!("failed to convert stderr to string"))?,
    );

    Ok(output.status.success())
}
