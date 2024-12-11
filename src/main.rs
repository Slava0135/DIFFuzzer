use greybox::fuzzer::fuzz;
use log::info;

mod abstract_fs;
mod greybox;
mod mount;
mod blackbox;

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    info!("starting up");
    fuzz();
}
