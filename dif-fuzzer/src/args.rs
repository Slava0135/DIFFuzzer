use crate::filesystems::filesystems_available;
use clap::{builder::PossibleValuesParser, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    /// Path to configuration file in TOML format
    #[arg(long,default_value_t = String::from("./config.toml"))]
    pub config_path: String,

    #[clap(subcommand)]
    pub mode: Mode,
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
    Single {
        /// Place where results will be saved
        #[arg(short, long)]
        save_to_dir: String,
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
    },
}
