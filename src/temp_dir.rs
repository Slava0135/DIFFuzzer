use log::info;
use std::path::{Path, PathBuf};
use std::{env, fs};

pub fn setup_temp_dir() -> PathBuf {
    info!("setting up temporary directory");
    let temp_dir = env::temp_dir().join("DIFFuzzer");
    fs::remove_dir_all(temp_dir.as_path()).unwrap_or(());
    fs::create_dir(temp_dir.as_path()).unwrap();

    info!("copying executor to '{}'", temp_dir.display());
    let executor_dir = Path::new("executor");
    let makefile = "makefile";
    let executor_h = "executor.h";
    let executor_cpp = "executor.cpp";
    fs::copy(executor_dir.join(makefile), temp_dir.join(makefile)).unwrap();
    fs::copy(executor_dir.join(executor_h), temp_dir.join(executor_h)).unwrap();
    fs::copy(executor_dir.join(executor_cpp), temp_dir.join(executor_cpp)).unwrap();
    temp_dir
}
