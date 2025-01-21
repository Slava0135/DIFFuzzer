use std::{fs, path::Path};

use crate::fuzzing::blackbox::fuzzer::BlackBoxFuzzer;
use crate::fuzzing::greybox::fuzzer::Fuzzer;
use args::Args;
use clap::Parser;
use config::Config;
use log::info;
use rand::random;

mod abstract_fs;
mod args;
mod config;
mod filesystems;
mod fuzzing;
mod harness;
mod hasher;
mod mount;
mod save;
mod single;
mod temp_dir;

fn main() {
    let args = Args::parse();

    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    info!("logger initialized");
    info!("reading configuration");
    let config = fs::read_to_string(args.config_path).expect("failed to read configuration file");
    let config: Config = toml::from_str(&config).expect("bad configuration");

    match args.mode {
        args::Mode::Greybox {
            first_filesystem,
            second_filesystem,
            test_count,
        } => {
            let mut fuzzer = Fuzzer::new(
                config,
                first_filesystem.try_into().unwrap(),
                second_filesystem.try_into().unwrap(),
            );
            fuzzer.fuzz();
        }
        args::Mode::Blackbox {
            first_filesystem,
            second_filesystem,
            test_count,
        } => {
            BlackBoxFuzzer::new(
                first_filesystem.try_into().unwrap(),
                second_filesystem.try_into().unwrap(),
                config.fs_name.clone(),
            )
            .fuzz(test_count, config);
        }
        args::Mode::Single {
            save_to_dir,
            path_to_test,
            filesystem,
        } => single::run(
            Path::new(&path_to_test),
            Path::new(&save_to_dir),
            filesystem.try_into().unwrap(),
            config.fs_name,
        ),
    }
}
