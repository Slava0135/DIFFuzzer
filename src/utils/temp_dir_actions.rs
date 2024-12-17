use log::info;
use rand::prelude::StdRng;
use rand::SeedableRng;
use std::env;
use std::path::{Path, PathBuf};

pub fn get_temp_dir() -> PathBuf {
    let temp_dir = env::temp_dir().join("DIFFuzzer");
    std::fs::remove_dir_all(temp_dir.as_path()).unwrap_or(());
    std::fs::create_dir(temp_dir.as_path()).unwrap();

    info!("copying executor to '{}'", temp_dir.display());
    let executor_dir = Path::new("executor");
    let makefile = "makefile";
    let executor_h = "executor.h";
    let executor_cpp = "executor.cpp";
    std::fs::copy(executor_dir.join(makefile), temp_dir.join(makefile)).unwrap();
    std::fs::copy(executor_dir.join(executor_h), temp_dir.join(executor_h)).unwrap();
    std::fs::copy(executor_dir.join(executor_cpp), temp_dir.join(executor_cpp)).unwrap();
    return temp_dir;
}
