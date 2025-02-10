use std::{cell::RefCell, fs::read_to_string, path::Path, rc::Rc};

use anyhow::Context;
use log::info;

use crate::{
    abstract_fs::{trace::TRACE_FILENAME, workload::Workload},
    command::{CommandInterface, LocalCommandInterface},
    fuzzing::harness::Harness,
    mount::mount::FileSystemMount,
    path::{LocalPath, RemotePath},
    save::{save_output, save_testcase},
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

    let temp_dir = cmdi
        .setup_remote_dir()
        .with_context(|| "failed to setup temp dir")
        .unwrap();
    let test_dir = temp_dir.clone();

    let exec_dir = temp_dir.join("exec");
    cmdi.remove_dir_all(&exec_dir).unwrap_or(());
    cmdi.create_dir_all(&exec_dir)
        .with_context(|| format!("failed to create executable directory"))
        .unwrap();

    let trace_path = exec_dir.join(TRACE_FILENAME);

    info!("compiling test at '{}'", test_dir);
    let binary_path = input
        .compile(&cmdi, &test_dir)
        .with_context(|| format!("failed to compile test"))
        .unwrap();

    let stdout = Rc::new(RefCell::new("".to_owned()));
    let stderr = Rc::new(RefCell::new("".to_owned()));

    let fs_str = mount.to_string();
    let fs_dir = RemotePath::new(Path::new("/mnt"))
        .join(fs_str.to_lowercase())
        .join(fs_name);
    let harness = Harness::new(mount, fs_dir, exec_dir, stdout.clone(), stderr.clone());

    info!("running harness");
    harness
        .run(&cmdi, &binary_path, keep_fs, None)
        .with_context(|| format!("failed to run harness"))
        .unwrap();

    info!("saving results");
    save_testcase(&cmdi, save_to_dir, &binary_path, &input)
        .with_context(|| format!("failed to save testcase"))
        .unwrap();
    save_output(
        &cmdi,
        save_to_dir,
        &trace_path,
        &fs_str,
        stdout.borrow().clone(),
        stderr.borrow().clone(),
    )
    .with_context(|| format!("failed to save output"))
    .unwrap();
}
