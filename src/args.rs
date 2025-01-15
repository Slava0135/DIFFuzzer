use clap::{Parser, Subcommand};

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
        first_filesystem: String,
        /// Second filesystem to test
        #[arg(short, long)]
        second_filesystem: String,
    },
    /// Run single test
    Single {
        /// Place where results will be saved
        #[arg(short, long)]
        save_to_dir: String,
        /// Path to testcase in JSON format
        #[arg(short, long)]
        path_to_test: String,
        /// Filesystem to test
        #[arg(short, long)]
        filesystem: String,
    },
}
