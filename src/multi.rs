use dv_wrap::Context;

mod dot;
mod op;
mod pm;
mod user;

pub fn register(lua: &mlua::Lua, ctx: Context) -> mlua::Result<()> {
    let ctx = std::sync::Arc::new(tokio::sync::Mutex::new(ctx));
    let dv = lua.create_table()?;
    let op = lua.create_userdata(op::Op::new(ctx.clone()))?;
    dv.set("op", op)?;
    let dot = lua.create_userdata(dot::Dot::new(ctx.clone()))?;
    dv.set("dot", dot)?;
    let um = lua.create_userdata(user::UserManager::new(ctx.clone()))?;
    dv.set("um", um)?;
    let dev = lua.create_userdata(pm::Pm::new(ctx.clone()))?;
    dv.set("pm", dev)?;
    lua.globals().set("dv", dv)
}
