/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{fs, path::Path};

use crate::fuzzing::duo_single::DuoSingleFuzzer;
use anyhow::{Context, Ok};
use args::Args;
use clap::Parser;
use config::Config;
use fuzzing::{
    blackbox::fuzzer::BlackBoxFuzzer, fuzzer::Fuzzer, greybox::fuzzer::GreyBoxFuzzer,
    reducer::Reducer, solo_single,
};
use log::{error, info};
use path::LocalPath;

mod abstract_fs;
mod args;
mod command;
mod compile;
mod config;
mod filesystems;
mod fuzzing;
mod markdown;
mod mount;
mod path;
mod reason;
mod save;
mod supervisor;

fn main() {
    let status = run();
    if let Err(ref err) = status {
        error!("{:?}", err);
    }
}

fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    log4rs::init_file("log4rs.yml", Default::default()).with_context(|| "failed to init logger")?;
    info!("init logger");

    info!("read configuration");
    let config = fs::read_to_string(args.config_path)
        .with_context(|| "failed to read configuration file")?;
    let config: Config =
        toml::from_str(&config).with_context(|| "failed to parse configuration")?;

    match args.mode {
        args::Mode::Greybox {
            first_filesystem,
            second_filesystem,
            test_count,
            corpus_path,
        } => {
            info!(
                "start greybox fuzzing ('{}' + '{}')",
                first_filesystem, second_filesystem
            );
            GreyBoxFuzzer::create(
                config,
                first_filesystem.into(),
                second_filesystem.into(),
                LocalPath::new(Path::new("./crashes")),
                corpus_path,
                args.no_qemu,
            )?
            .run(test_count);
        }
        args::Mode::Blackbox {
            first_filesystem,
            second_filesystem,
            test_count,
        } => {
            info!(
                "start blackbox fuzzing ('{}' + '{}')",
                first_filesystem, second_filesystem
            );
            BlackBoxFuzzer::create(
                config,
                first_filesystem.into(),
                second_filesystem.into(),
                LocalPath::new(Path::new("./crashes")),
                args.no_qemu,
            )?
            .run(test_count);
        }
        args::Mode::SoloSingle {
            output_dir,
            path_to_test,
            keep_fs,
            filesystem,
        } => {
            info!("run single test ('{}')", filesystem);
            solo_single::run(
                &LocalPath::new(Path::new(&path_to_test)),
                &LocalPath::new(Path::new(&output_dir)),
                keep_fs,
                filesystem.into(),
                config,
                args.no_qemu,
            )?
        }
        args::Mode::DuoSingle {
            first_filesystem,
            second_filesystem,
            output_dir,
            path_to_test,
            keep_fs,
        } => {
            info!(
                "run single test ('{}' + '{}')",
                first_filesystem, second_filesystem
            );
            DuoSingleFuzzer::create(
                config,
                first_filesystem.into(),
                second_filesystem.into(),
                LocalPath::new(Path::new(&output_dir)),
                LocalPath::new(Path::new(&path_to_test)),
                keep_fs,
                args.no_qemu,
            )?
            .run(Some(1u64));
        }
        args::Mode::Reduce {
            output_dir,
            path_to_test,
            first_filesystem,
            second_filesystem,
        } => {
            info!(
                "reduce test ('{}' + '{}')",
                first_filesystem, second_filesystem
            );
            Reducer::create(
                config,
                first_filesystem.into(),
                second_filesystem.into(),
                LocalPath::new(Path::new(&output_dir)),
                args.no_qemu,
            )?
            .run(
                &LocalPath::new(Path::new(&path_to_test)),
                &LocalPath::new(Path::new(&output_dir)),
            )?;
        }
    }
    Ok(())
}
