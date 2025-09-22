use clap::{Arg, Command};
use std::path::PathBuf;

pub struct Args {
    pub cache_dir: Option<PathBuf>,
    pub config: PathBuf,
    pub dbpath: PathBuf,
    pub dry_run: bool,
    pub entry: String,
    pub rargs: Vec<String>,
}

pub fn cli() -> Args {
    let matches =
        Command::new("dv4lua")
            .version(env!("CARGO_PKG_VERSION"))
            .about("Simple CLI to use dv-api with lua")
            .arg(
                Arg::new("dbpath")
                    .short('b')
                    .long("dbpath")
                    .help("database path, default [$directory/.cache] -> [project cache dir]"),
            )
            .arg(Arg::new("cache_dir").short('a').long("cache-dir").help(
                "The cache directory to use, default [project cache dir] -> [$directory/cache]",
            ))
            .arg(Arg::new("config").short('c').long("config").help(
                "The config file to use, default [$directory/config.lua] -> [project config dir]",
            ))
            .arg(
                Arg::new("directory")
                    .short('d')
                    .long("directory")
                    .help("The directory to use for the config and cache"),
            )
            .arg(
                Arg::new("dry_run")
                    .short('n')
                    .long("dry-run")
                    .action(clap::ArgAction::SetTrue)
                    .default_value("false")
                    .help("Do not actually modify anything"),
            )
            .arg(
                Arg::new("entry")
                    .help("The entry point of the script")
                    .default_value("Main"),
            )
            .arg(
                Arg::new("rargs")
                    .num_args(0..)
                    .trailing_var_arg(true)
                    .help("Arguments to pass to the entry point"),
            )
            .get_matches();

    let directory = matches.get_one::<PathBuf>("directory");
    let cache_dir = matches
        .get_one::<PathBuf>("cache_dir")
        .cloned()
        .or_else(|| crate::DIR.as_ref().map(|d| d.cache_dir().to_path_buf()))
        .or_else(|| directory.as_ref().map(|d| d.join("cache")));
    let dbpath = matches
        .get_one::<PathBuf>("dbpath")
        .cloned()
        .or_else(|| directory.as_ref().map(|d| d.join(".cache")))
        .or_else(|| {
            crate::DIR
                .as_ref()
                .map(|d| d.data_local_dir().join(".cache"))
        })
        .expect("dbpath must be calculated");
    let config = matches
        .get_one::<PathBuf>("config")
        .cloned()
        .or_else(|| directory.map(|d| d.join("config.lua")))
        .or_else(|| {
            crate::DIR
                .as_ref()
                .map(|d| d.config_local_dir().join("config.lua"))
        })
        .expect("config must be calculated");
    let dry_run = matches
        .get_one::<bool>("dry_run")
        .expect("defaulted by clap");
    let entry = matches
        .get_one::<String>("entry")
        .cloned()
        .expect("defaulted by clap");
    let rargs: Vec<String> = matches
        .get_many::<String>("rargs")
        .unwrap_or_default()
        .cloned()
        .collect();
    Args {
        dbpath,
        cache_dir,
        config,
        dry_run: *dry_run,
        entry,
        rargs,
    }
}
