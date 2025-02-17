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
    command::{CommandInterface, LocalCommandInterface},
    fuzzing::{harness::Harness, runner::setup_dir},
    mount::FileSystemMount,
    path::{LocalPath, RemotePath},
    save::{save_outcome, save_testcase},
};

pub fn run(
    test_path: &LocalPath,
    output_dir: &LocalPath,
    keep_fs: bool,
    mount: &'static dyn FileSystemMount,
    fs_name: String,
) -> anyhow::Result<()> {
    info!("read testcase at '{}'", test_path);
    let input = read_to_string(test_path).with_context(|| "failed to read testcase")?;
    let input: Workload = serde_json::from_str(&input).with_context(|| "failed to parse json")?;

    let cmdi = LocalCommandInterface::new();

    let remote_dir = cmdi
        .setup_remote_dir()
        .with_context(|| "failed to setup remote dir")?;
    let test_dir = remote_dir.clone();

    let exec_dir = remote_dir.join("exec");
    setup_dir(&cmdi, &exec_dir)?;

    info!("compile test at '{}'", test_dir);
    let binary_path = input
        .compile(&cmdi, &test_dir)
        .with_context(|| "failed to compile test")?;

    let fs_str = mount.to_string();
    let fs_dir = RemotePath::new(Path::new("/mnt"))
        .join(fs_str.to_lowercase())
        .join(&fs_name);
    let harness = Harness::new(
        mount,
        fs_dir,
        exec_dir,
        LocalPath::new_tmp("outcome-single"),
    );

    info!("run harness");
    let outcome = harness
        .run(&cmdi, &binary_path, keep_fs, None)
        .with_context(|| "failed to run harness")?;

    info!("save results");
    fs::create_dir_all(output_dir)?;
    save_testcase(&cmdi, output_dir, &binary_path, &input)
        .with_context(|| "failed to save testcase")?;
    save_outcome(output_dir, &fs_str, &outcome).with_context(|| "failed to save outcome")?;

    Ok(())
}
