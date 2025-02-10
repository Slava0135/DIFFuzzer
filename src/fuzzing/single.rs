use std::{fs::read_to_string, path::Path};

use anyhow::Context;
use log::info;

use crate::{
    abstract_fs::workload::Workload,
    command::{CommandInterface, LocalCommandInterface},
    fuzzing::{harness::Harness, runner::setup_dir},
    mount::mount::FileSystemMount,
    path::{LocalPath, RemotePath},
    save::{save_outcome, save_testcase},
};

pub fn run(
    test_path: &LocalPath,
    save_to_dir: &LocalPath,
    keep_fs: bool,
    mount: &'static dyn FileSystemMount,
    fs_name: String,
) {
    info!("running single test");

    info!("reading testcase at '{}'", test_path);
    let input = read_to_string(test_path)
        .with_context(|| format!("failed to read testcase"))
        .unwrap();
    let input: Workload = serde_json::from_str(&input)
        .with_context(|| format!("failed to parse json"))
        .unwrap();

    let cmdi = LocalCommandInterface::new();

    let remote_dir = cmdi
        .setup_remote_dir()
        .with_context(|| "failed to setup remote dir")
        .unwrap();
    let test_dir = remote_dir.clone();

    let exec_dir = remote_dir.join("exec");
    setup_dir(&cmdi, &exec_dir).unwrap();

    info!("compiling test at '{}'", test_dir);
    let binary_path = input
        .compile(&cmdi, &test_dir)
        .with_context(|| format!("failed to compile test"))
        .unwrap();

    let fs_str = mount.to_string();
    let fs_dir = RemotePath::new(Path::new("/mnt"))
        .join(fs_str.to_lowercase())
        .join(&fs_name);
    let harness = Harness::new(
        mount,
        fs_dir,
        exec_dir,
        LocalPath::new(&Path::new("/tmp").join("diffuzzer-outcome-single")),
    );

    info!("running harness");
    let outcome = harness
        .run(&cmdi, &binary_path, keep_fs, None)
        .with_context(|| format!("failed to run harness"))
        .unwrap();

    info!("saving results");
    save_testcase(&cmdi, save_to_dir, &binary_path, &input)
        .with_context(|| format!("failed to save testcase"))
        .unwrap();
    save_outcome(save_to_dir, &fs_name, &outcome)
        .with_context(|| format!("failed to save outcome"))
        .unwrap();
}
