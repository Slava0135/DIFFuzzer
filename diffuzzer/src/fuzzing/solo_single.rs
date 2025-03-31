/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{
    fs::{self, read_to_string},
    path::Path,
};

use anyhow::Context;
use log::info;

use crate::{
    abstract_fs::workload::Workload,
    config::Config,
    fuzzing::{harness::Harness, outcome::Outcome},
    mount::FileSystemMount,
    path::{LocalPath, RemotePath},
    reason::Reason,
    save::{save_completed, save_reason, save_testcase},
    supervisor::launch_cmdi_and_supervisor,
};

pub fn run(
    test_path: &LocalPath,
    output_dir: &LocalPath,
    keep_fs: bool,
    mount: &'static dyn FileSystemMount,
    config: Config,
    no_qemu: bool,
) -> anyhow::Result<()> {
    let local_tmp_dir = LocalPath::new_tmp("solo-single");
    fs::remove_dir(local_tmp_dir.as_ref()).unwrap_or(());
    fs::create_dir_all(local_tmp_dir.as_ref()).with_context(|| {
        format!(
            "failed to create local temporary directory at '{}'",
            local_tmp_dir,
        )
    })?;

    let (cmdi, mut supervisor) = launch_cmdi_and_supervisor(no_qemu, &config, &local_tmp_dir)?;

    info!("read testcase at '{}'", test_path);
    let input = read_to_string(test_path).with_context(|| "failed to read testcase")?;
    let input: Workload = serde_json::from_str(&input).with_context(|| "failed to parse json")?;

    let remote_dir = cmdi
        .setup_remote_dir()
        .with_context(|| "failed to setup remote dir")?;
    let test_dir = remote_dir.clone();

    let exec_dir = remote_dir.join("exec");

    info!("compile test at '{}'", test_dir);
    let binary_path = input
        .compile(cmdi.as_ref(), &test_dir)
        .with_context(|| "failed to compile test")?;

    let fs_str = mount.to_string();
    let fs_dir = RemotePath::new(Path::new("/mnt"))
        .join(fs_str.to_lowercase())
        .join(&config.fs_name);
    let harness = Harness::new(
        mount,
        fs_dir,
        exec_dir,
        local_tmp_dir.join("outcome-single"),
        config.timeout,
        vec![],
    );

    info!("run harness");

    let outcome = harness
        .run(cmdi.as_ref(), &binary_path, keep_fs, supervisor.as_mut())
        .with_context(|| "failed to run harness")?;

    info!("save results");
    fs::create_dir_all(output_dir)?;

    match outcome {
        Outcome::Completed(completed) => {
            save_testcase(cmdi.as_ref(), output_dir, Some(&binary_path), &input)
                .with_context(|| "failed to save testcase")?;
            save_completed(output_dir, &fs_str, &completed)
                .with_context(|| "failed to save outcome")?;
            let mut reason = Reason::new();
            reason
                .md
                .heading(format!("Filesystem '{}' completed workload", fs_str));
            save_reason(output_dir, reason).with_context(|| "failed to save reason")?;
        }
        Outcome::Panicked => {
            save_testcase(cmdi.as_ref(), output_dir, None, &input)
                .with_context(|| "failed to save testcase")?;
            let mut reason = Reason::new();
            reason
                .md
                .heading(format!("Filesystem '{}' panicked", fs_str));
            save_reason(output_dir, reason).with_context(|| "failed to save reason")?;
        }
        Outcome::TimedOut => {
            save_testcase(cmdi.as_ref(), output_dir, None, &input)
                .with_context(|| "failed to save testcase")?;
            let mut reason = Reason::new();
            reason.md.heading(format!(
                "Filesystem '{}' timed out after {}s",
                fs_str, config.timeout
            ));
            save_reason(output_dir, reason).with_context(|| "failed to save reason")?;
        }
    };

    Ok(())
}
