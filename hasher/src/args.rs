use clap::{builder::PossibleValuesParser, Parser};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    /// Path to mount
    #[arg(long)]
    pub fs_path: String,

    /// Output file
    #[arg(long, default_value = "./files.json")]
    pub target_path: String,

    #[arg(short, long, default_value_t = false)]
    pub size: bool,
    #[arg(short, long, default_value_t = false)]
    pub nlink: bool,
    #[arg(short, long, default_value_t = false)]
    pub mode: bool,
}
