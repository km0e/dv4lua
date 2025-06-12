mod pm;
pub use pm::Packages;
mod user;

pub fn register(lua: &mlua::Lua, globals: &mlua::Table) -> mlua::Result<()> {
    let table = lua.create_table()?;
    user::register(lua, &table)?;
    pm::register(lua, &table)?;
    globals.set("user", table)?;
    Ok(())
}
