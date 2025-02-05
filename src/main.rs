use std::{fs, path::Path};

use args::Args;
use clap::Parser;
use config::Config;
use fuzzing::native::{blackbox::fuzzer::BlackBoxFuzzer, common::Fuzzer, greybox::fuzzer::GreyBoxFuzzer, reducer::Reducer, single};
use log::info;

mod abstract_fs;
mod args;
mod config;
mod filesystems;
mod fuzzing;
mod hasher;
mod mount;
mod save;
mod temp_dir;
mod command;

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
            if args.qemu {
                todo!("QEMU not supported")
            }
            GreyBoxFuzzer::new(
                config,
                first_filesystem.try_into().unwrap(),
                second_filesystem.try_into().unwrap(),
            )
            .run(test_count);
        }
        args::Mode::Blackbox {
            first_filesystem,
            second_filesystem,
            test_count,
        } => {
            if args.qemu {
                todo!("QEMU not supported")
            }
            BlackBoxFuzzer::new(
                config,
                first_filesystem.try_into().unwrap(),
                second_filesystem.try_into().unwrap(),
            )
            .run(test_count);
        }
        args::Mode::Single {
            save_to_dir,
            path_to_test,
            keep_fs,
            filesystem,
        } => {
            if args.qemu {
                todo!("QEMU not supported")
            }
            single::run(
                Path::new(&path_to_test),
                Path::new(&save_to_dir),
                keep_fs,
                filesystem.try_into().unwrap(),
                config.fs_name,
            )
        }
        args::Mode::Reduce {
            output_dir,
            path_to_test,
            first_filesystem,
            second_filesystem,
        } => {
            if args.qemu {
                todo!("QEMU not supported")
            }
            Reducer::new(
                config,
                first_filesystem.try_into().unwrap(),
                second_filesystem.try_into().unwrap(),
            )
            .run(Path::new(&path_to_test), Path::new(&output_dir))
            .unwrap();
        }
    }
}
