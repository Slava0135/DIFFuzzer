use std::fs;
use std::path::Path;

use anyhow::Context;
use clap::Parser;
use regex::RegexSet;
use serde_json::to_string;

use crate::mount::mount::FileSystemMount;
use args::Args;
use hasher::{calc_dir_hash, HasherOptions};

mod args;
mod lib;
mod mount;

fn main() {
    let args = Args::parse();

    let hasher_options = HasherOptions {
        size: args.size,
        nlink: args.nlink,
        mode: args.mode,
    };

    let skip = match args.exclude {
        None => RegexSet::new::<_, &str>([]).unwrap(),
        Some(v) => RegexSet::new(v).unwrap(),
    };
    let (hash, files) = calc_dir_hash(Path::new(&args.target_path), &skip, &hasher_options);
    println!("{}", hash);
    let serialized_file = to_string(&files).unwrap();
    fs::write(Path::new(&args.output_path), serialized_file)
        .with_context(|| format!("failed write diff to file"))
        .unwrap();
}
