use std::{fs, path::Path};

use args::{string_to_fs, Args};
use clap::Parser;
use config::Config;
use greybox::fuzzer::Fuzzer;
use log::info;

mod abstract_fs;
mod args;
mod blackbox;
mod config;
mod greybox;
mod harness;
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
            first_filesystem: fst_filesystem,
            second_filesystem: snd_filesystem,
        } => {
            let mut fuzzer = Fuzzer::new(
                config,
                string_to_fs(fst_filesystem),
                string_to_fs(snd_filesystem),
            );
            fuzzer.fuzz();
        }
        args::Mode::Single {
            save_to_dir,
            path_to_test,
            filesystem,
        } => single::run(
            Path::new(&path_to_test),
            Path::new(&save_to_dir),
            string_to_fs(filesystem),
        ),
    }
}
