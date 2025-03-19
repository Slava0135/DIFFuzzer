/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fs;

use anyhow::Context;

use crate::command::CommandInterface;
use crate::compile::{TEST_EXE_FILENAME, TEST_SOURCE_FILENAME};
use crate::fuzzing::outcome::Completed;
use crate::path::LocalPath;
use crate::reason::Reason;
use crate::{
    abstract_fs::{trace::TRACE_FILENAME, workload::Workload},
    path::RemotePath,
};

pub const TEST_FILE_NAME: &str = "test.json";

pub fn save_testcase(
    cmdi: &dyn CommandInterface,
    output_dir: &LocalPath,
    binary_path: Option<&RemotePath>,
    input: &Workload,
) -> anyhow::Result<()> {
    let source_path = output_dir.join(TEST_SOURCE_FILENAME);
    fs::write(&source_path, input.clone().encode_c())
        .with_context(|| format!("failed to save source file to '{}'", source_path))?;

    let binary_copy_path = output_dir.join(TEST_EXE_FILENAME);
    if let Some(binary_path) = binary_path {
        cmdi.copy_from_remote(binary_path, &binary_copy_path)
            .with_context(|| {
                format!(
                    "failed to copy binary test executable from '{}' to '{}'",
                    binary_path, binary_copy_path
                )
            })?;
    }

    let json_path = output_dir.join(TEST_FILE_NAME);
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

pub fn save_reason(output_dir: &LocalPath, reason: Reason) -> anyhow::Result<()> {
    let reason_path = output_dir.join("reason.md");
    fs::write(&reason_path, reason.to_string())
        .with_context(|| format!("failed to save source file to '{}'", reason_path))
}
