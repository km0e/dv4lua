mod dev {
    pub use super::ContextWrapper;
    pub use anyhow::Result;
    pub use mlua::{UserData, UserDataMethods};
}
use anyhow::bail;
use dev::*;

use dv_wrap::ops::{self, SyncEntry, SyncOpt};
use futures::{StreamExt, TryStreamExt, stream};
use std::{cell::RefCell, rc::Rc, time::Duration};

use dv_wrap::Context;
use mlua::{FromLua, Function, Lua, LuaSerdeExt, Value};

use crate::util::{conversion_error, sync_opts};

mod dot;
mod pm;
mod user;

#[derive(Clone)]
pub struct ContextWrapper {
    ctx: Rc<RefCell<Context>>,
    lua: Rc<RefCell<Lua>>,
    dry_run: bool,
}

impl dv_wrap::AsRefContext for ContextWrapper {
    fn as_ref(&self) -> impl std::ops::Deref<Target = Context> + '_ {
        self.ctx()
    }
}

impl ContextWrapper {
    fn new(ctx: dv_wrap::Context, dry_run: bool) -> Self {
        Self {
            ctx: Rc::new(RefCell::new(ctx)),
            lua: Rc::new(RefCell::new(Lua::new())),
            dry_run,
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
    ) -> Result<bool> {
        let opts = sync_opts(confirm.unwrap_or_default())?;
        let ctx = self.ctx();
        let sync_ctx = ops::SyncContext::new(&ctx, src.as_ref(), dst.as_ref(), &opts);
        let res = stream::iter(pairs)
            .map(|(src_path, dst_path)| sync_ctx.scan(src_path, dst_path))
            .buffered(4)
            .try_fold(Vec::new(), |mut res, copy_res| async move {
                res.extend(copy_res);
                Ok(res)
            })
            .await?;
        self.sync_impl(src, dst, &res).await
    }
    async fn sync_impl(
        &self,
        src: impl AsRef<str>,
        dst: impl AsRef<str>,
        entries: &[SyncEntry],
    ) -> Result<bool> {
        let ctx = self.ctx();
        for e in entries {
            match e.opt {
                SyncOpt::OVERWRITE => {
                    ctx.interactor
                        .log(format!("Overwrite: {} -> {}", e.src, e.dst))
                        .await;
                }
                SyncOpt::UPDATE => {
                    ctx.interactor
                        .log(format!("Update: {} -> {}", e.src, e.dst))
                        .await;
                }
                SyncOpt::UPLOAD => {
                    ctx.interactor
                        .log(format!("Upload: {} -> {}", e.src, e.dst))
                        .await;
                }
                SyncOpt::DOWNLOAD => {
                    ctx.interactor
                        .log(format!("Download: {} -> {}", e.src, e.dst))
                        .await;
                }
                SyncOpt::DELETESRC => {
                    ctx.interactor.log(format!("Delete local: {}", e.src)).await;
                }
                SyncOpt::DELETEDST => {
                    ctx.interactor
                        .log(format!("Delete remote: {}", e.src))
                        .await;
                }
                _ => {
                    bail!("Unknown operation for sync: {:?}", e.opt);
                }
            }
        }
        if self.dry_run {
            return Ok(true);
        }
        let sync_ctx = ops::SyncContext::new(&ctx, src.as_ref(), dst.as_ref(), &[]);
        sync_ctx.execute(entries).await
    }
    async fn once(
        &self,
        id: impl AsRef<str>,
        key: impl AsRef<str>,
        f: Function,
    ) -> Result<bool, mlua::Error> {
        let once = ops::Once::new(self.clone(), id.as_ref(), key.as_ref());
        if !once.test().await? {
            return Ok(false);
        }
        self.ctx()
            .interactor
            .log(format!("Once executing: {}:{}", id.as_ref(), key.as_ref()))
            .await;
        if self.dry_run {
            return Ok(true);
        }
        let res = f.call_async::<bool>(()).await;
        if res.is_ok() {
            once.execute().await?;
        }
        res
    }
    async fn refresh(&self, id: impl AsRef<str>, key: impl AsRef<str>) -> Result<()> {
        self.ctx()
            .interactor
            .log(format!("Refresh: {}:{}", id.as_ref(), key.as_ref()))
            .await;
        if self.dry_run {
            return Ok(());
        }
        let ctx = self.ctx();
        ops::refresh(&ctx, id.as_ref(), key.as_ref()).await
    }

    async fn dl(
        &self,
        url: impl AsRef<str>,
        expire: Option<humantime_serde::Serde<Duration>>,
    ) -> Result<String, mlua::Error> {
        let expire = expire.map(|e| e.as_secs());
        let (path, dl) = ops::Dl::new(self.clone(), url.as_ref(), expire).await?;
        let Some(dl) = dl else {
            return Ok(path);
        };
        self.ctx()
            .interactor
            .log(format!("Download: {} -> {}", url.as_ref(), path))
            .await;
        if self.dry_run {
            return Ok(path);
        }
        dl.execute(&path).await?;
        Ok(path)
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
                Ok(this.sync(&src, &dst, &pairs, confirm.as_deref()).await?)
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
            |_, this, (id, key): (String, String)| async move { Ok(this.refresh(id, key).await?) },
        );

        methods.add_async_method(
            "dl",
            |_, this, (url, expire): (String, Option<Value>)| async move {
                this.dl(
                    url,
                    expire
                        .map(|expire| this.lua().from_value(expire))
                        .transpose()?,
                )
                .await
            },
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

pub fn register(ctx: dv_wrap::Context, dry_run: bool) -> mlua::Result<ContextWrapper> {
    let ctx = ContextWrapper::new(ctx, dry_run);
    ctx.lua().globals().set("dv", ctx.clone())?;
    Ok(ctx)
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
