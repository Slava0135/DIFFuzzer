/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::filesystems::filesystems_available;
use clap::{Parser, Subcommand, builder::PossibleValuesParser};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    /// Path to configuration file in TOML format
    #[arg(long, default_value_t = String::from("./config.toml"))]
    pub config_path: String,

    #[clap(subcommand)]
    pub mode: Mode,

    /// Run tests on host instead of QEMU (not recommended)
    #[arg(short, long, default_value_t = false)]
    pub no_qemu: bool,
}

#[derive(Debug, PartialEq, Clone, Subcommand)]
#[clap(rename_all = "kebab_case")]
pub enum Mode {
    /// Run greybox fuzzing
    Greybox {
        /// First filesystem to test
        #[arg(short, long)]
        #[clap(value_parser = PossibleValuesParser::new(filesystems_available()))]
        first_filesystem: String,
        /// Second filesystem to test
        #[arg(short, long)]
        #[clap(value_parser = PossibleValuesParser::new(filesystems_available()))]
        second_filesystem: String,
        /// Test count
        #[arg(short, long)]
        test_count: Option<u64>,
        /// Load corpus from directory
        #[arg(short, long)]
        corpus_path: Option<String>,
    },
    /// Run blackbox fuzzing
    Blackbox {
        /// First filesystem to test
        #[arg(short, long)]
        #[clap(value_parser = PossibleValuesParser::new(filesystems_available()))]
        first_filesystem: String,
        /// Second filesystem to test
        #[arg(short, long)]
        #[clap(value_parser = PossibleValuesParser::new(filesystems_available()))]
        second_filesystem: String,
        /// Test count
        #[arg(short, long)]
        test_count: Option<u64>,
    },
    /// Run single test
    SoloSingle {
        /// Place where results will be saved
        #[arg(short, long)]
        output_dir: String,
        /// Path to testcase in JSON format
        #[arg(short, long)]
        path_to_test: String,
        /// Keep FS after test
        #[arg(short, long, default_value_t = false)]
        keep_fs: bool,
        /// Filesystem to test
        #[arg(short, long)]
        #[clap(value_parser = PossibleValuesParser::new(filesystems_available()))]
        filesystem: String,
    },
    /// Run single test for 2 filesystems
    DuoSingle {
        /// First filesystem to test
        #[arg(short, long)]
        #[clap(value_parser = PossibleValuesParser::new(filesystems_available()))]
        first_filesystem: String,
        /// Second filesystem to test
        #[arg(short, long)]
        #[clap(value_parser = PossibleValuesParser::new(filesystems_available()))]
        second_filesystem: String,
        /// Place where results will be saved
        #[arg(short, long)]
        output_dir: String,
        /// Path to testcase in JSON format
        #[arg(short, long)]
        path_to_test: String,
        /// Keep FS after test
        #[arg(short, long, default_value_t = false)]
        keep_fs: bool,
    },
    /// Reduce testcase
    Reduce {
        /// Place where results will be saved
        #[arg(short, long)]
        output_dir: String,
        /// Path to testcase in JSON format
        #[arg(short, long)]
        path_to_test: String,
        /// First filesystem to test
        #[arg(short, long)]
        #[clap(value_parser = PossibleValuesParser::new(filesystems_available()))]
        first_filesystem: String,
        /// Second filesystem to test
        #[arg(short, long)]
        #[clap(value_parser = PossibleValuesParser::new(filesystems_available()))]
        second_filesystem: String,
        /// Bug variation limit (default 0 - no limit)
        #[arg(short, long, default_value_t = 0)]
        variation_limit: usize,
    },
}
