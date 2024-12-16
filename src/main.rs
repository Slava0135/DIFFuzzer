use std::fs;

use config::Config;
use greybox::fuzzer::fuzz;
use log::info;

mod abstract_fs;
mod config;
mod greybox;
mod mount;

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    info!("logger initialized");
    info!("reading configuration");
    let config = fs::read_to_string("config.toml")
        .expect("expected configuration file in working directory");
    let config: Config = toml::from_str(&config).expect("bad configuration");
    fuzz(config);
}
