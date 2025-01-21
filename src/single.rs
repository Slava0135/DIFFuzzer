use std::{
    cell::RefCell,
    fs::{self, read_to_string},
    path::Path,
    rc::Rc,
};

use anyhow::Context;
use log::info;

use crate::{
    abstract_fs::{trace::TRACE_FILENAME, workload::Workload},
    harness::Harness,
    mount::mount::FileSystemMount,
    save::{save_output, save_testcase},
    temp_dir::setup_temp_dir,
};

pub fn run(
    test_path: &Path,
    save_to_dir: &Path,
    mount: &'static dyn FileSystemMount,
    fs_name: String,
) {
    info!("running single test");

    info!("reading testcase at '{}'", test_path.display());
    let input = read_to_string(test_path)
        .with_context(|| format!("failed to read testcase"))
        .unwrap();
    let input: Workload = serde_json::from_str(&input)
        .with_context(|| format!("failed to parse json"))
        .unwrap();

    let temp_dir = setup_temp_dir();
    let test_dir = temp_dir.clone();

    let exec_dir = temp_dir.join("exec");
    fs::remove_dir_all(&exec_dir).unwrap_or(());
    fs::create_dir(&exec_dir)
        .with_context(|| format!("failed to create executable directory"))
        .unwrap();
    let trace_path = exec_dir.join(TRACE_FILENAME);

    info!("compiling test at '{}'", test_dir.display());
    let input_path = input
        .compile(test_dir.as_path())
        .with_context(|| format!("failed to compile test"))
        .unwrap();

    let stdout = Rc::new(RefCell::new("".to_owned()));
    let stderr = Rc::new(RefCell::new("".to_owned()));

    let fs_str = mount.to_string();
    let harness = Harness::new(
        mount,
        Path::new("/mnt")
            .join(fs_str.to_lowercase())
            .join(fs_name)
            .into_boxed_path(),
        exec_dir.to_owned().into_boxed_path(),
        stdout.clone(),
        stderr.clone(),
    );

    info!("running harness");
    harness
        .run(&input_path)
        .with_context(|| format!("failed to run harness"))
        .unwrap();

    info!("saving results");
    save_testcase(save_to_dir, &input_path, &input)
        .with_context(|| format!("failed to save testcase"))
        .unwrap();
    save_output(
        save_to_dir,
        &trace_path,
        &fs_str,
        stdout.borrow().clone(),
        stderr.borrow().clone(),
    )
    .with_context(|| format!("failed to save output"))
    .unwrap();
}
