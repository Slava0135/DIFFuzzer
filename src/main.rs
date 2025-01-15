use std::{fs, path::Path};

use args::Args;
use clap::Parser;
use config::Config;
use greybox::fuzzer::Fuzzer;
use log::info;
use mount::{btrfs::Btrfs, ext4::Ext4};
use rand::random;
use crate::blackbox::fuzzer::BlackBoxFuzzer;

mod abstract_fs;
mod abstract_fuzzer;
mod args;
mod blackbox;
mod config;
mod greybox;
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
            count,
            trace_len,
        } => {
            //todo: create fs instance from input data (without match if possible)
            BlackBoxFuzzer::new(
                Ext4::new(),
                Btrfs::new()
            ).fuzz(count, trace_len, random(), config);
        }
        args::Mode::Single {
            save_to_dir,
            path_to_test,
            filesystem,
        } => single::run(
            Path::new(&path_to_test),
            Path::new(&save_to_dir),
            filesystem.try_into().unwrap(),
        ),
    }
}
