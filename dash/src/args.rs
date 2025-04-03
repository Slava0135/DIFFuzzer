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

    /// Include total size of files in bytes when calculating hash
    #[arg(short, long, default_value_t = false)]
    pub size: bool,
    /// Include number of hard links pointing to files when calculating hash (for files)
    #[arg(short, long, default_value_t = false)]
    pub file_nlink: bool,
    /// Include number of hard links pointing to files when calculating hash (for dirs)
    #[arg(short, long, default_value_t = false)]
    pub dir_nlink: bool,
    /// Include rights applied to files when calculating hash
    #[arg(short, long, default_value_t = false)]
    pub mode: bool,
    /// Regex patterns of directories and files to exclude from state and hash
    /// Note: patterns are applied to full paths, relative to mount point
    /// Example: -e "output.log" -e "\w*.rs"
    #[arg(short, long, verbatim_doc_comment)]
    pub exclude: Option<Vec<String>>,
}
