use crate::filesystems::filesystems_available;
use clap::{builder::PossibleValuesParser, Parser};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    /// Path to mount
    #[arg(short, long)]
    pub target_path: String,

    /// Output file
    #[arg(short, long, default_value = "./files.json")]
    pub output_path: String,

    #[arg(short, long, default_value_t = false)]
    pub size: bool,
    #[arg(short, long, default_value_t = false)]
    pub nlink: bool,
    #[arg(short, long, default_value_t = false)]
    pub mode: bool,
    /// Filesystem to test
    #[arg(short, long)]
    #[clap(value_parser = PossibleValuesParser::new(filesystems_available()))]
    pub filesystem: String,
}
