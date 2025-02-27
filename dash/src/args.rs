/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    /// Path to mount point
    #[arg(short, long)]
    pub target_path: String,

    /// Output file
    #[arg(short, long, default_value = "./fs-state.json")]
    pub output_path: String,

    /// Include total size of files in bytes
    #[arg(short, long, default_value_t = false)]
    pub size: bool,
    /// Include number of hard links pointing to files
    #[arg(short, long, default_value_t = false)]
    pub nlink: bool,
    /// Include rights applied to files
    #[arg(short, long, default_value_t = false)]
    pub mode: bool,
    /// List of regex patterns of dirs and files to skip
    #[arg(short, long)]
    pub exclude: Option<Vec<String>>,
}
