use anyhow::Context;
use log::info;
use std::path::Path;

use crate::{
    command::{CommandInterface, CommandWrapper},
    path::{LocalPath, RemotePath},
};

const EXECUTOR_SOURCE_DIR: &str = "./executor";
const MAKEFILE_NAME: &str = "makefile";
const EXECUTOR_H_NAME: &str = "executor.h";
const EXECUTOR_CPP_NAME: &str = "executor.cpp";
const TEST_NAME: &str = "test.c";

pub fn setup_temp_dir(cmdi: &dyn CommandInterface) -> anyhow::Result<RemotePath> {
    let temp_dir = RemotePath::new(&Path::new("/tmp").join("DIFFuzzer"));

    info!(
        "setting up remote temporary directory at '{}'",
        temp_dir.base.display()
    );
    cmdi.remove_dir_all(&temp_dir).unwrap_or(());
    cmdi.create_dir_all(&temp_dir).with_context(|| {
        format!(
            "failed to create remote temporary directory at '{}'",
            temp_dir.base.display()
        )
    })?;

    info!("copying executor to '{}'", temp_dir.base.display());
    let executor_dir = LocalPath::new(&Path::new(EXECUTOR_SOURCE_DIR));
    cmdi.copy_to_remote(
        &executor_dir.join(MAKEFILE_NAME),
        &temp_dir.join(MAKEFILE_NAME),
    )?;
    cmdi.copy_to_remote(
        &executor_dir.join(EXECUTOR_H_NAME),
        &temp_dir.join(EXECUTOR_H_NAME),
    )?;
    cmdi.copy_to_remote(
        &executor_dir.join(EXECUTOR_CPP_NAME),
        &temp_dir.join(EXECUTOR_CPP_NAME),
    )?;
    cmdi.copy_to_remote(
        &executor_dir.join(EXECUTOR_CPP_NAME),
        &temp_dir.join(EXECUTOR_CPP_NAME),
    )?;
    cmdi.copy_to_remote(&executor_dir.join(TEST_NAME), &temp_dir.join(TEST_NAME))?;

    let mut make = CommandWrapper::new("make");
    make.arg("-C").arg(executor_dir.as_ref());
    cmdi.exec(make)
        .with_context(|| "failed to make test binary")?;

    Ok(temp_dir)
}
