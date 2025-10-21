#![allow(clippy::await_holding_refcell_ref)]
static DIR: std::sync::LazyLock<Option<directories::ProjectDirs>> =
    std::sync::LazyLock::new(|| directories::ProjectDirs::from("dev", "dv", "dv4lua"));

use dv_wrap::{Context, MultiDB, TermInteractor};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod arg;
mod multi;
mod util;

fn lua_string_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

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
    cache.add_sqlite(dbpath).map_err(mlua::Error::external)?;
    let interactor = TermInteractor::new().map_err(mlua::Error::external)?;
    let ctx = Context::new(cache, cache_dir, interactor);

    let ctx = multi::register(ctx, dry_run)?;

    let mut content = std::fs::read_to_string(&config).unwrap_or_else(|_| {
        tracing::error!("Failed to read config file: {}", config.display());
        std::process::exit(1);
    });

    let call = format!(
        "\n{}({})\n",
        entry,
        rargs
            .iter()
            .map(|s| format!("\"{}\"", lua_string_escape(s)))
            .collect::<Vec<_>>()
            .join(", ")
    );
    tracing::info!("Executing entry point: {}", call.trim());
    content.push_str(&call);

    ctx.lua().load(content).exec_async().await?;
    Ok(())
}
