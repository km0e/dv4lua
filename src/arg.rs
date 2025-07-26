use clap::Parser;
use std::path::PathBuf;

fn default_config() -> PathBuf {
    home::home_dir()
        .expect("can't find home directory")
        .join(".config/dv/")
}

#[derive(Parser, Debug)]
#[command(version = env!("CARGO_PKG_VERSION"), about = "Simple CLI to use dv-api with lua")]
pub struct Cli {
    #[arg(short = 'b', long, help = "cache database path")]
    pub dbpath: Option<PathBuf>,
    #[arg(short, long, help = "The config file to use")]
    pub config: Option<PathBuf>,
    #[arg(short, long, help = "The directory to use for the config and cache",
        default_value_os_t = default_config())]
    pub directory: PathBuf,
    #[arg(
        short = 'n',
        long,
        default_value = "false",
        help = "Do not actually modify anything"
    )]
    pub dry_run: bool,
    #[arg(default_value = "Main", help = "The entry point of the script")]
    pub entry: String,
    #[arg(trailing_var_arg = true, help = "Arguments to pass to the entry point")]
    pub rargs: Vec<String>,
}
