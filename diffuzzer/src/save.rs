/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fs;
use std::fs::OpenOptions;
use std::io::Write;

use anyhow::Context;
use hasher::FileDiff::{DifferentHash, OneExists};
use hasher::{FileDiff, DIFF_HASH_FILENAME};

use crate::command::CommandInterface;
use crate::compile::{TEST_EXE_FILENAME, TEST_SOURCE_FILENAME};
use crate::fuzzing::outcome::Completed;
use crate::path::LocalPath;
use crate::{
    abstract_fs::{trace::TRACE_FILENAME, workload::Workload},
    path::RemotePath,
};

pub fn save_testcase(
    cmdi: &dyn CommandInterface,
    output_dir: &LocalPath,
    binary_path: &RemotePath,
    input: &Workload,
) -> anyhow::Result<()> {
    let source_path = output_dir.join(TEST_SOURCE_FILENAME);
    fs::write(&source_path, input.clone().encode_c())
        .with_context(|| format!("failed to save source file to '{}'", source_path))?;

    let binary_copy_path = output_dir.join(TEST_EXE_FILENAME);
    cmdi.copy_from_remote(binary_path, &binary_copy_path)
        .with_context(|| {
            format!(
                "failed to copy binary test executable from '{}' to '{}'",
                binary_path, binary_copy_path
            )
        })?;

    let json_path = output_dir.join("test").with_extension("json");
    let json = serde_json::to_string_pretty(&input)
        .with_context(|| format!("failed to copy workload as json at '{}'", json_path))?;
    fs::write(json_path, json)?;
    Ok(())
}

pub fn save_completed(
    output_dir: &LocalPath,
    fs_name: &str,
    outcome: &Completed,
) -> anyhow::Result<()> {
    let fs_name = fs_name.to_lowercase();
    let trace_path = outcome.dir.join(TRACE_FILENAME);
    let trace_copy_path = output_dir.join(format!("{}.{}", fs_name, TRACE_FILENAME));
    fs::copy(trace_path.as_ref(), trace_copy_path.as_ref()).with_context(|| {
        format!(
            "failed to copy trace from '{}' to '{}'",
            trace_path, trace_copy_path
        )
    })?;

    let stdout_path = output_dir.join(format!("{}.stdout.txt", fs_name));
    fs::write(&stdout_path, outcome.stdout.clone())
        .with_context(|| format!("failed to save stdout at '{}'", stdout_path))?;

    let stderr_path = output_dir.join(format!("{}.stderr.txt", fs_name));
    fs::write(&stderr_path, outcome.stderr.clone())
        .with_context(|| format!("failed to save stderr at '{}'", stderr_path))?;

    Ok(())
}

pub fn save_diff(output_dir: &LocalPath, diff_hash: Vec<FileDiff>) -> anyhow::Result<()> {
    let diff_hash_path = output_dir.join(DIFF_HASH_FILENAME);
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&diff_hash_path)
        .with_context(|| format!("failed to save hash difference at '{}'", diff_hash_path))?;

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
