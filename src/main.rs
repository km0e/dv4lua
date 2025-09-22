#![allow(clippy::await_holding_refcell_ref)]
static DIR: std::sync::LazyLock<Option<directories::ProjectDirs>> =
    std::sync::LazyLock::new(|| directories::ProjectDirs::from("dev", "dv", "dv4lua"));

use dv_wrap::{Context, MultiDB, TermInteractor};
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

    let arg::Args {
        config,
        cache_dir,
        dry_run,
        entry,
        dbpath,
        rargs,
    } = arg::cli();

    tracing::debug!(?config, ?cache_dir, ?dry_run, ?entry, ?dbpath, ?rargs);

    let mut cache = MultiDB::default();
    cache.add_sqlite(dbpath);
    let ctx = Context::new(
        dry_run,
        cache,
        cache_dir,
        TermInteractor::new().expect("Failed to create interactor"),
    );

    let ctx = multi::register(ctx)?;

    let mut content = std::fs::read_to_string(config).expect("cannot read config file");

    let call = format!(
        "\n{}({})\n",
        entry,
        rargs
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
