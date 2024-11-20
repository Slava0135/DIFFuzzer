use std::path::Path;

use fs_wrap::{setup, teardown};
use log::info;
use rand::{rngs::StdRng, SeedableRng};

mod abstract_fs;
mod encode;
mod fs_wrap;
mod input;
mod mutator;

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    info!("starting up");

    let mut rng = StdRng::seed_from_u64(123);
    let seq = mutator::generate_new(&mut rng, 100);
    println!("{}", encode::encode_c(seq));
    setup(
        Path::new("/mnt").join("ext4").join("fstest").as_path(),
        fs_wrap::FileSystemType::EXT4,
    )
    .unwrap();
    teardown(
        Path::new("/mnt").join("ext4").join("fstest").as_path(),
        fs_wrap::FileSystemType::EXT4,
    )
    .unwrap();
}
