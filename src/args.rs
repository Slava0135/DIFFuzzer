use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(
        long,
        help = "Path to configuration file in TOML format",
        default_value_t = String::from("./config.toml"),
    )]
    pub config_path: String,
}
