use std::path::Path;

use abstract_fs::{encode::encode_c, generator::generate_new};
use fs_wrap::{setup, teardown};
use log::info;
use rand::{rngs::StdRng, SeedableRng};

mod abstract_fs;
mod fs_wrap;
mod input;
mod observer;

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    info!("starting up");

    let mut rng = StdRng::seed_from_u64(123);
    let seq = generate_new(&mut rng, 100);
    println!("{}", encode_c(seq));
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
