/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::{fs, path::Path};

use crate::fuzzing::duo_single::DuoSingleFuzzer;
use anyhow::{Context, Ok};
use args::Args;
use clap::Parser;
use command::{CommandInterface, LocalCommandInterface, RemoteCommandInterface};
use config::Config;
use fuzzing::{
    blackbox::fuzzer::BlackBoxFuzzer, fuzzer::Fuzzer, greybox::fuzzer::GreyBoxFuzzer,
    reducer::Reducer, solo_single,
};
use log::{error, info};
use path::LocalPath;
use supervisor::{NativeSupervisor, QemuSupervisor, Supervisor};

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
mod supervisor;
mod markdown;

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

    let supervisor: Box<dyn Supervisor> = if args.no_qemu {
        Box::new(NativeSupervisor::new())
    } else {
        Box::new(QemuSupervisor::launch(&config.qemu).unwrap())
    };

    let cmdi: Box<dyn CommandInterface> = if args.no_qemu {
        Box::new(LocalCommandInterface::new())
    } else {
        Box::new(RemoteCommandInterface::new(&config.qemu))
    };

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
                cmdi,
                supervisor,
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
                cmdi,
                supervisor,
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
                cmdi,
                supervisor,
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
                cmdi,
                supervisor,
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
                cmdi,
                supervisor,
            )?
            .run(
                &LocalPath::new(Path::new(&path_to_test)),
                &LocalPath::new(Path::new(&output_dir)),
            )?;
        }
    }
    Ok(())
}
