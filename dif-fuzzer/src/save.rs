use std::fs::OpenOptions;
use std::io::Write;
use std::{fs, path::Path};

use anyhow::Context;

use crate::abstract_fs::{
    compile::{TEST_EXE_FILENAME, TEST_SOURCE_FILENAME},
    trace::TRACE_FILENAME,
    workload::Workload,
};
use crate::hasher::hasher::FileDiff::{DifferentHash, OneExists};
use crate::hasher::hasher::{FileDiff, DIFF_HASH_FILENAME};

pub fn save_testcase(dir: &Path, input_path: &Path, input: &Workload) -> anyhow::Result<()> {
    let source_path = dir.join(TEST_SOURCE_FILENAME);
    fs::write(&source_path, input.clone().encode_c())
        .with_context(|| format!("failed to save source file to '{}'", source_path.display()))?;

    let exe_path = dir.join(TEST_EXE_FILENAME);
    fs::copy(&input_path, &exe_path).with_context(|| {
        format!(
            "failed to copy executable from '{}' to '{}'",
            input_path.display(),
            exe_path.display()
        )
    })?;

    let json_path = dir.join("test").with_extension("json");
    let json = serde_json::to_string_pretty(&input).with_context(|| {
        format!(
            "failed to copy workload as json at '{}'",
            json_path.display()
        )
    })?;
    fs::write(json_path, json)?;
    Ok(())
}

pub fn save_output(
    dir: &Path,
    trace_path: &Path,
    fs_name: &str,
    stdout: String,
    stderr: String,
) -> anyhow::Result<()> {
    let copy_trace_path = dir.join(format!("{}.{}", fs_name, TRACE_FILENAME));
    fs::copy(trace_path, &copy_trace_path).with_context(|| {
        format!(
            "failed to copy trace from '{}' to '{}'",
            trace_path.display(),
            copy_trace_path.display()
        )
    })?;

    let stdout_path = dir.join(format!("{}.stdout.txt", fs_name));
    fs::write(&stdout_path, stdout)
        .with_context(|| format!("failed to save stdout at '{}'", stdout_path.display()))?;

    let stderr_path = dir.join(format!("{}.stderr.txt", fs_name));
    fs::write(&stderr_path, stderr)
        .with_context(|| format!("failed to save stderr at '{}'", stderr_path.display()))?;

    Ok(())
}

pub fn save_diff(dir: &Path, diff_hash: Vec<FileDiff>) -> anyhow::Result<()> {
    let diff_hash_path = dir.join(DIFF_HASH_FILENAME);
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&diff_hash_path)
        .unwrap();

    for diff in diff_hash {
        let txt = match diff {
            DifferentHash { fst, snd } => {
                format!("File with different hash:\n {}\n\n {}\n\n", fst, snd)
            }
            OneExists(f) => format!("File exists only in one FS:\n {}\n\n", f),
        };
        file.write(txt.as_bytes()).with_context(|| {
            format!(
                "failed to save source file to '{}'",
                diff_hash_path.display()
            )
        })?;
    }
    Ok(())
}
