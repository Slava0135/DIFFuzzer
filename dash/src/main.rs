/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fs;
use std::path::Path;

use anyhow::Context;
use clap::Parser;
use dash::{HasherOptions, calc_dir_hash};
use regex::RegexSet;
use serde_json::to_string;

use args::Args;

mod args;
#[cfg(test)]
mod test;

fn main() {
    let args = Args::parse();

    let hasher_options = HasherOptions {
        size: args.size,
        nlink: args.nlink,
        mode: args.mode,
    };

    let skip = RegexSet::new(args.exclude.unwrap_or(vec![])).unwrap();
    let (hash, files) =
        calc_dir_hash(Path::new(&args.target_path), &skip, &hasher_options).unwrap();
    println!("{}", hash);
    let serialized_file = to_string(&files).unwrap();
    fs::write(Path::new(&args.output_path), serialized_file)
        .with_context(|| "failed write output to file")
        .unwrap();
}
