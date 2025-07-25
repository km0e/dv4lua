use clap::Parser;

use dv::Dv;
use mlua::{Function, Lua};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
mod error;

mod arg;
mod multi;

mod dv;

mod util;

#[tokio::main]
async fn main() -> Result<(), mlua::Error> {
    tracing_subscriber::Registry::default()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().with_thread_ids(true))
        // .with(tracing_subscriber::fmt::layer().pretty())
        .init();

    let args = arg::Cli::parse();

    let dbpath = args.dbpath.unwrap_or_else(|| args.directory.join(".cache"));
    let lua = Lua::new();

    let dv = lua.create_table()?;
    let op = lua
        .create_userdata(Dv::new(&dbpath, args.dry_run))
        .expect("cannot create dv userdata");
    dv.set("op", op).expect("cannot set dv userdata");
    multi::register(&lua, &dv).expect("cannot register dv module");
    lua.globals().set("dv", dv).expect("cannot set dv globals");
    let mut content = std::fs::read_to_string(
        args.config
            .unwrap_or_else(|| args.directory.join("config.lua")),
    )
    .expect("cannot read config file");

    let call = format!(
        "\n{}({})\n",
        args.entry,
        args.rargs
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    tracing::info!("Executing entry point: {}", call.trim());
    content.push_str(&call);

    lua.load(content).exec_async().await?;
    Ok(())
}
