mod dot;
mod op;
mod pm;
mod user;

pub fn register(lua: &mlua::Lua, ctx: dv_wrap::Context) -> mlua::Result<()> {
    let ctx = std::sync::Arc::new(tokio::sync::RwLock::new(ctx));
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

mod dev {
    pub use crate::util::external_error;
    pub use dv_wrap::Context;
    pub use mlua::{UserData, UserDataMethods};
    pub use std::sync::Arc;
    pub use tokio::sync::RwLock;
}
