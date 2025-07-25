use mlua::{Lua, Table};

pub fn register(lua: &Lua, tb: &Table) -> mlua::Result<()> {
    let cur = lua.create_function(|lua, _args: ()| {
        let cfg = lua.create_table()?;
        cfg.set("mount", "~/.local/share/dv")?;
        cfg.set("os", "linux")?;
        Ok(cfg)
    })?;
    let ssh = lua.create_function(|lua, _args: ()| {
        let cfg = lua.create_table()?;
        cfg.set("mount", "~/.local/share/dv")?;
        cfg.set("os", "linux")?;
        Ok(cfg)
    })?;
    tb.set("cur", cur)?;
    tb.set("ssh", ssh)?;
    Ok(())
}
