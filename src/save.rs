use std::fs;
use std::fs::OpenOptions;
use std::io::Write;

use anyhow::Context;

use crate::hasher::hasher::FileDiff::{DifferentHash, OneExists};
use crate::hasher::hasher::{FileDiff, DIFF_HASH_FILENAME};
use crate::path::LocalPath;
use crate::{
    abstract_fs::{
        compile::{TEST_EXE_FILENAME, TEST_SOURCE_FILENAME},
        trace::TRACE_FILENAME,
        workload::Workload,
    },
    path::RemotePath,
};

pub fn save_testcase(
    dir: &LocalPath,
    binary_path: &RemotePath,
    input: &Workload,
) -> anyhow::Result<()> {
    let source_path = dir.join(TEST_SOURCE_FILENAME);
    fs::write(&source_path, input.clone().encode_c())
        .with_context(|| format!("failed to save source file to '{}'", source_path))?;

    let binary_copy_path = dir.join(TEST_EXE_FILENAME);
    todo!("use cmdi");
    fs::copy(&binary_path.base, &binary_copy_path).with_context(|| {
        format!(
            "failed to copy executable from '{}' to '{}'",
            binary_path, binary_copy_path
        )
    })?;

    let json_path = dir.join("test").with_extension("json");
    let json = serde_json::to_string_pretty(&input)
        .with_context(|| format!("failed to copy workload as json at '{}'", json_path))?;
    fs::write(json_path, json)?;
    Ok(())
}

pub fn save_output(
    dir: &LocalPath,
    trace_path: &RemotePath,
    fs_name: &str,
    stdout: String,
    stderr: String,
) -> anyhow::Result<()> {
    let copy_trace_path = dir.join(format!("{}.{}", fs_name, TRACE_FILENAME));
    todo!("use cmdi");
    fs::copy(trace_path.base.as_ref(), &copy_trace_path).with_context(|| {
        format!(
            "failed to copy trace from '{}' to '{}'",
            trace_path, copy_trace_path
        )
    })?;

    let stdout_path = dir.join(format!("{}.stdout.txt", fs_name));
    fs::write(&stdout_path, stdout)
        .with_context(|| format!("failed to save stdout at '{}'", stdout_path))?;

    let stderr_path = dir.join(format!("{}.stderr.txt", fs_name));
    fs::write(&stderr_path, stderr)
        .with_context(|| format!("failed to save stderr at '{}'", stderr_path))?;

    Ok(())
}

pub fn save_diff(dir: &LocalPath, diff_hash: Vec<FileDiff>) -> anyhow::Result<()> {
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
        file.write(txt.as_bytes())
            .with_context(|| format!("failed to save source file to '{}'", diff_hash_path))?;
    }
    Ok(())
}
