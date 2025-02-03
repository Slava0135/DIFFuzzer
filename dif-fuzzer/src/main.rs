use std::{fs, path::Path};

use args::Args;
use clap::Parser;
use config::Config;
use fuzzing::{
    blackbox::fuzzer::BlackBoxFuzzer, fuzzer::Fuzzer, greybox::fuzzer::GreyBoxFuzzer,
    reducer::Reducer, single,
};
use log::info;
use path::LocalPath;

mod abstract_fs;
mod args;
mod command;
mod compile;
mod config;
mod filesystems;
mod fuzzing;
mod hasher;
mod mount;
mod path;
mod save;

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
            if args.no_qemu {
                GreyBoxFuzzer::new(
                    config,
                    first_filesystem.try_into().unwrap(),
                    second_filesystem.try_into().unwrap(),
                )
                .run(test_count);
            } else {
                todo!("QEMU not supported");
            }
        }
        args::Mode::Blackbox {
            first_filesystem,
            second_filesystem,
            test_count,
        } => {
            if args.no_qemu {
                BlackBoxFuzzer::new(
                    config,
                    first_filesystem.try_into().unwrap(),
                    second_filesystem.try_into().unwrap(),
                )
                .run(test_count);
            } else {
                todo!("QEMU not supported");
            }
        }
        args::Mode::Single {
            output_dir: save_to_dir,
            path_to_test,
            keep_fs,
            filesystem,
        } => {
            if args.no_qemu {
                single::run(
                    &LocalPath::new(Path::new(&path_to_test)),
                    &LocalPath::new(Path::new(&save_to_dir)),
                    keep_fs,
                    filesystem.try_into().unwrap(),
                    config.fs_name,
                )
            } else {
                todo!("QEMU not supported");
            }
        }
        args::Mode::Reduce {
            output_dir,
            path_to_test,
            first_filesystem,
            second_filesystem,
        } => {
            if args.no_qemu {
                Reducer::new(
                    config,
                    first_filesystem.try_into().unwrap(),
                    second_filesystem.try_into().unwrap(),
                )
                .run(
                    &LocalPath::new(Path::new(&path_to_test)),
                    &LocalPath::new(Path::new(&output_dir)),
                )
                .unwrap();
            } else {
                todo!("QEMU not supported");
            }
        }
    }
}
