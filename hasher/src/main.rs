use std::fs;
use std::path::Path;

use anyhow::Context;
use clap::Parser;
use serde_json::to_string;

use args::Args;
use hasher::{calc_dir_hash, HasherOptions};
use crate::mount::mount::FileSystemMount;

mod args;
mod filesystems;
mod lib;
mod mount;

fn main() {
    let args = Args::parse();

    let hasher_options = HasherOptions {
        size: args.size,
        nlink: args.nlink,
        mode: args.mode,
    };


    let skip = <String as TryInto<&'static dyn FileSystemMount>>::try_into(args.filesystem).unwrap().get_internal_dirs();
    let (hash, files) = calc_dir_hash(
        Path::new(&args.target_path),
        &skip,
        &hasher_options,
    );
    println!("{}", hash);
    let serialized_file = to_string(&files).unwrap();
    fs::write(Path::new(&args.output_path), serialized_file)
        .with_context(|| format!("failed write diff to file"))
        .unwrap();
}
