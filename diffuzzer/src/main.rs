/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{fs, path::Path};

use crate::fuzzing::duo_single::DuoSingleFuzzer;
use args::Args;
use clap::Parser;
use config::Config;
use fuzzing::{
    blackbox::fuzzer::BlackBoxFuzzer, fuzzer::Fuzzer, greybox::fuzzer::GreyBoxFuzzer,
    reducer::Reducer, solo_single,
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
mod mount;
mod path;
mod save;

fn main() {
    let args = Args::parse();

    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    info!("init logger");

    info!("read configuration");
    let config = fs::read_to_string(args.config_path).expect("failed to read configuration file");
    let config: Config = toml::from_str(&config).expect("bad configuration file");

    match args.mode {
        args::Mode::Greybox {
            first_filesystem,
            second_filesystem,
            test_count,
        } => {
            if args.no_qemu {
                GreyBoxFuzzer::new(
                    config,
                    first_filesystem.into(),
                    second_filesystem.into(),
                    LocalPath::new(Path::new("./crashes")),
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
                    first_filesystem.into(),
                    second_filesystem.into(),
                    LocalPath::new(Path::new("./crashes")),
                )
                .run(test_count);
            } else {
                todo!("QEMU not supported");
            }
        }
        args::Mode::SoloSingle {
            output_dir,
            path_to_test,
            keep_fs,
            filesystem,
        } => {
            if args.no_qemu {
                solo_single::run(
                    &LocalPath::new(Path::new(&path_to_test)),
                    &LocalPath::new(Path::new(&output_dir)),
                    keep_fs,
                    filesystem.into(),
                    config.fs_name,
                )
            } else {
                todo!("QEMU not supported");
            }
        }
        args::Mode::DuoSingle {
            first_filesystem,
            second_filesystem,
            output_dir,
            path_to_test,
            keep_fs,
        } => {
            if args.no_qemu {
                DuoSingleFuzzer::new(
                    config,
                    first_filesystem.into(),
                    second_filesystem.into(),
                    LocalPath::new(Path::new(&output_dir)),
                    LocalPath::new(Path::new(&path_to_test)),
                    keep_fs,
                )
                .run(Some(1u64));
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
                    first_filesystem.into(),
                    second_filesystem.into(),
                    LocalPath::new(Path::new(&output_dir)),
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
