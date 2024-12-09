use std::path::Path;

use abstract_fs::{encode::encode_c, generator::generate_new};
use log::info;
use mount::{ext4::Ext4, mount::FileSystemMount};
use rand::{rngs::StdRng, SeedableRng};

mod abstract_fs;
mod greybox;
mod mount;

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    info!("starting up");

    let mut rng = StdRng::seed_from_u64(123);
    let seq = generate_new(&mut rng, 100);
    println!("{}", encode_c(seq));
    let ext4 = Ext4::new();
    ext4.setup(Path::new("/mnt").join("ext4").join("fstest").as_path())
        .unwrap();
    ext4.teardown(Path::new("/mnt").join("ext4").join("fstest").as_path())
        .unwrap();
}
