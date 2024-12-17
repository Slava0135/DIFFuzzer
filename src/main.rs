use std::fs;

use args::Args;
use clap::Parser;
use config::Config;
use greybox::fuzzer::fuzz;
use log::info;

mod abstract_fs;
mod args;
mod blackbox;
mod config;
mod greybox;
mod mount;
mod harness;
mod temp_dir;

fn main() {
    let args = Args::parse();

    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    info!("logger initialized");
    info!("reading configuration");
    let config = fs::read_to_string(args.config_path).expect("failed to read configuration file");
    let config: Config = toml::from_str(&config).expect("bad configuration");
    fuzz(config);
}
