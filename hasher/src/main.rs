mod lib;
mod args;

use regex::RegexSet;
use args::Args;
use hasher::{calc_dir_hash, HasherOptions};

fn main() {
    let args = Args::parse();

    let hasher_options = HasherOptions { size: args.size, nlink: args.nlink, mode: args.mode };

    let r_set = RegexSet::new::<_, &str>([]).unwrap();
    let hash = calc_dir_hash(args.fs_path, &r_set, &hasher_options);
    println!("{}", hash)
}
