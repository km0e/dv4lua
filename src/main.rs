use clap::Parser;

use dv::Dv;
use mlua::{Function, Lua};
mod error;

mod arg;
mod multi;

mod dv;

mod util;

#[tokio::main]
async fn main() -> Result<(), mlua::Error> {
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
    content.push_str(&format!("\n{}({})\n", args.entry, args.rargs.join(", ")));

    lua.load(content).exec_async().await?;
    Ok(())
}
