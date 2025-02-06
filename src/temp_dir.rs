use anyhow::Context;
use log::info;
use std::path::{Path, PathBuf};

use crate::command::{CommandInterface, CommandWrapper};

const EXECUTOR_SOURCE_DIR: &str = "./executor";
const MAKEFILE_NAME: &str = "makefile";
const EXECUTOR_H_NAME: &str = "executor.h";
const EXECUTOR_CPP_NAME: &str = "executor.cpp";
const TEST_NAME: &str = "test.c";

pub fn setup_temp_dir(cmdi: &dyn CommandInterface) -> anyhow::Result<PathBuf> {
    let temp_dir = Path::new("/tmp").join("DIFFuzzer");

    info!("setting up temporary directory at '{}'", temp_dir.display());
    cmdi.remove_dir_all(&temp_dir).unwrap_or(());
    cmdi.create_dir_all(&temp_dir).with_context(|| {
        format!(
            "failed to create temporary directory at '{}'",
            temp_dir.display()
        )
    })?;

    info!("copying executor to '{}'", temp_dir.display());
    let executor_dir = Path::new(EXECUTOR_SOURCE_DIR);
    cmdi.copy_to_guest(
        &executor_dir.join(MAKEFILE_NAME),
        &temp_dir.join(MAKEFILE_NAME),
    )?;
    cmdi.copy_to_guest(
        &executor_dir.join(EXECUTOR_H_NAME),
        &temp_dir.join(EXECUTOR_H_NAME),
    )?;
    cmdi.copy_to_guest(
        &executor_dir.join(EXECUTOR_CPP_NAME),
        &temp_dir.join(EXECUTOR_CPP_NAME),
    )?;
    cmdi.copy_to_guest(
        &executor_dir.join(EXECUTOR_CPP_NAME),
        &temp_dir.join(EXECUTOR_CPP_NAME),
    )?;
    cmdi.copy_to_guest(&executor_dir.join(TEST_NAME), &temp_dir.join(TEST_NAME))?;

    let mut make = CommandWrapper::new("make");
    make.arg("-C").arg(executor_dir);
    cmdi.exec(make)
        .with_context(|| "failed to make test binary")?;

    Ok(temp_dir)
}
