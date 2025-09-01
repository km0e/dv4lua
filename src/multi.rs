use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use dv_wrap::Context;
use mlua::Lua;

mod dot;
mod op;
mod pm;
mod user;

#[derive(Clone)]
pub struct ContextWrapper {
    ctx: Rc<RefCell<Context>>,
    lua: Rc<RefCell<Lua>>,
}

impl ContextWrapper {
    pub fn lua(&'_ self) -> Ref<'_, Lua> {
        self.lua.borrow()
    }
    fn ctx(&'_ self) -> Ref<'_, Context> {
        self.ctx.borrow()
    }
    fn ctx_mut(&'_ self) -> std::cell::RefMut<'_, Context> {
        self.ctx.borrow_mut()
    }
}

pub fn register(ctx: dv_wrap::Context) -> mlua::Result<ContextWrapper> {
    let ctx = ContextWrapper {
        lua: Rc::new(RefCell::new(Lua::new())),
        ctx: Rc::new(RefCell::new(ctx)),
    };
    let lua = ctx.lua();
    let dv = lua.create_table()?;
    let op = lua.create_userdata(op::Op::new(ctx.clone()))?;
    dv.set("op", op)?;
    let dot = lua.create_userdata(dot::Dot::new(ctx.clone()))?;
    dv.set("dot", dot)?;
    let um = lua.create_userdata(user::UserManager::new(ctx.clone()))?;
    dv.set("um", um)?;
    let dev = lua.create_userdata(pm::Pm::new(ctx.clone()))?;
    dv.set("pm", dev)?;
    lua.globals().set("dv", dv)?;
    drop(lua);
    Ok(ctx)
}

mod dev {
    pub use super::ContextWrapper;
    pub use crate::util::external_error;
    pub use mlua::{UserData, UserDataMethods};
}
