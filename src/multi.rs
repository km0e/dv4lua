use dv_wrap::ops;
use futures::{StreamExt, TryStreamExt, stream};
use std::{cell::RefCell, rc::Rc, time::Duration};

use dv_wrap::Context;
use mlua::{FromLua, Function, Lua, LuaSerdeExt, UserData, UserDataMethods, Value};

use crate::util::{conversion_error, external_error};

mod dot;
mod pm;
mod user;

#[derive(Clone)]
pub struct ContextWrapper {
    ctx: Rc<RefCell<Context>>,
    lua: Rc<RefCell<Lua>>,
}

impl ContextWrapper {
    fn new(ctx: Context) -> Self {
        Self {
            ctx: Rc::new(RefCell::new(ctx)),
            lua: Rc::new(RefCell::new(Lua::new())),
        }
    }
    fn ctx(&self) -> std::cell::Ref<'_, Context> {
        self.ctx.borrow()
    }
    fn ctx_mut(&self) -> std::cell::RefMut<'_, Context> {
        self.ctx.borrow_mut()
    }
    pub fn lua(&self) -> std::cell::Ref<'_, Lua> {
        self.lua.borrow()
    }
    async fn sync(
        &self,
        src: impl AsRef<str>,
        dst: impl AsRef<str>,
        pairs: &[(String, String)],
        confirm: Option<&str>,
    ) -> Result<bool, dv_wrap::error::Error> {
        let ctx = self.ctx();
        let sync_ctx = ops::SyncContext::new(&ctx, src.as_ref(), dst.as_ref(), confirm)?;
        let res = stream::iter(pairs)
            .map(|(src_path, dst_path)| sync_ctx.sync(src_path, dst_path))
            .buffered(4)
            .try_fold(false, |res, copy_res| async move { Ok(res | copy_res) })
            .await?;
        Ok(res)
    }
    async fn once(
        &self,
        id: impl AsRef<str>,
        key: impl AsRef<str>,
        f: Function,
    ) -> Result<bool, mlua::Error> {
        let ctx = self.ctx();
        let once = ops::Once::new(&ctx, id.as_ref(), key.as_ref());
        if !external_error(once.test()).await? {
            return Ok(false);
        }
        let res = f.call_async::<bool>(()).await;
        external_error(once.set()).await?;
        res
    }
    async fn refresh(
        &self,
        id: impl AsRef<str>,
        key: impl AsRef<str>,
    ) -> Result<(), dv_wrap::error::Error> {
        ops::refresh(&self.ctx(), id.as_ref(), key.as_ref()).await
    }

    async fn dl(&self, url: impl AsRef<str>, expire: Option<Value>) -> Result<String, mlua::Error> {
        let expire: Option<humantime_serde::Serde<Duration>> = expire
            .map(|expire| self.lua().from_value(expire))
            .transpose()?;
        let expire = expire.map(|e| e.as_secs());
        external_error(ops::dl(&self.ctx(), url.as_ref(), expire)).await
    }
}

#[derive(serde::Deserialize)]
enum SyncPath {
    Single(String),
    Multiple(Vec<String>),
}

impl FromLua for SyncPath {
    fn from_lua(value: Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        if let Some(s) = value.as_string() {
            return Ok(SyncPath::Single(s.to_str()?.to_string()));
        }
        let vec: Vec<String> = lua.from_value(value)?;
        Ok(SyncPath::Multiple(vec))
    }
}

impl UserData for ContextWrapper {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method(
            "sync",
            |_,
             this,
             (src, src_path, dst, dst_path, confirm): (
                String,
                SyncPath,
                String,
                SyncPath,
                Option<String>,
            )| async move {
                let pairs: Vec<(String, String)> = match (src_path, dst_path) {
                    (SyncPath::Single(s), SyncPath::Single(d)) => vec![(s, d)],
                    (SyncPath::Single(s), SyncPath::Multiple(d)) => {
                        d.into_iter().map(|dp| (s.clone(), dp)).collect()
                    }
                    _ => Err(conversion_error(
                        "Value",
                        "SyncPath",
                        Some("Single dst_path required"),
                    ))?,
                };
                external_error(this.sync(&src, &dst, &pairs, confirm.as_deref())).await
            },
        );

        methods.add_async_method(
            "once",
            |_, this, (id, key, f): (String, String, Function)| async move {
                this.once(id, key, f).await
            },
        );

        methods.add_async_method(
            "refresh",
            |_, this, (id, key): (String, String)| async move {
                external_error(this.refresh(id, key)).await
            },
        );

        methods.add_async_method(
            "dl",
            |_, this, (url, expire): (String, Option<Value>)| async move { this.dl(url, expire).await },
        );

        methods.add_async_method("json", |_, this, (value,): (Value,)| async move {
            let lua = this.lua();
            if let Some(s) = value.as_string() {
                let val: serde_json::Value = serde_json::from_str(&s.to_str()?)
                    .map_err(|e| conversion_error("JSON(string)", "value", Some(e)))?;
                lua.to_value(&val)
            } else {
                lua.to_value(
                    &serde_json::to_string(&value)
                        .map_err(|e| conversion_error("value", "JSON(string)", Some(e)))?,
                )
            }
        });

        methods.add_method("dot", |_, this, ()| Ok(dot::Dot::new(this.clone())));
        methods.add_method("um", |_, this, ()| Ok(user::UserManager::new(this.clone())));
        methods.add_method("pm", |_, this, ()| Ok(pm::Pm::new(this.clone())));
    }
}

pub fn register(ctx: dv_wrap::Context) -> mlua::Result<ContextWrapper> {
    let ctx = ContextWrapper::new(ctx);
    ctx.lua().globals().set("dv", ctx.clone())?;
    Ok(ctx)
}

mod dev {
    pub use super::ContextWrapper;
    pub use crate::util::external_error;
    pub use mlua::{UserData, UserDataMethods};
}

#[cfg(test)]
mod tests {
    use mlua::FromLua;

    fn sync_path_des_suc_f(s: &str) -> Result<super::SyncPath, mlua::Error> {
        let lua = mlua::Lua::new();
        let val = lua.load(s).eval::<mlua::Value>().expect("Failed to load");
        super::SyncPath::from_lua(val, &lua)
    }

    #[test]
    fn sync_path_serde() {
        let sp = sync_path_des_suc_f("\"/path/to/single\"").expect("Failed to deserialize");
        match sp {
            super::SyncPath::Single(s) => assert_eq!(s, "/path/to/single"),
            _ => panic!("Expected Single variant"),
        }
        let sp =
            sync_path_des_suc_f("{\"/path/one\", \"/path/two\"}").expect("Failed to deserialize");
        match sp {
            super::SyncPath::Multiple(v) => {
                assert_eq!(v.len(), 2);
                assert_eq!(v[0], "/path/one");
                assert_eq!(v[1], "/path/two");
            }
            _ => panic!("Expected Multiple variant"),
        }
    }
}
