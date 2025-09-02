#![allow(clippy::await_holding_refcell_ref)]
use clap::Parser;

use dv_wrap::{Context, MultiCache, TermInteractor};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod arg;
mod multi;
mod util;

#[tokio::main]
async fn main() -> Result<(), mlua::Error> {
    tracing_subscriber::Registry::default()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().with_thread_ids(true))
        .init();

    let args = arg::Cli::parse();

    let dbpath = args.dbpath.unwrap_or_else(|| args.directory.join(".cache"));

    let mut cache = MultiCache::default();
    cache.add_sqlite(dbpath);
    let ctx = Context::new(
        args.dry_run,
        cache,
        TermInteractor::new().expect("Failed to create interactor"),
    );

    let ctx = multi::register(ctx)?;

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

    ctx.lua().load(content).exec_async().await?;
    Ok(())
}
