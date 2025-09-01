#![allow(clippy::await_holding_refcell_ref)]

use super::dev::*;
use dv_api::process::ScriptExecutor;
use dv_wrap::ops;
use futures::{StreamExt, TryStreamExt, stream};
use mlua::{Error as LuaError, FromLua, Function, LuaSerdeExt, Value};

use crate::util::conversion_error;

pub struct Op {
    ctx: ContextWrapper,
}

impl Op {
    pub fn new(ctx: ContextWrapper) -> Self {
        Self { ctx }
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

#[derive(serde::Deserialize)]
struct ExecOptions {
    reply: bool,
    etor: Option<ScriptExecutor>,
}

impl FromLua for ExecOptions {
    fn from_lua(value: Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        if let Some(b) = value.as_boolean() {
            return Ok(ExecOptions {
                reply: b,
                etor: None,
            });
        }
        lua.from_value(value)
    }
}

impl UserData for Op {
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
                external_error(async {
                    let ctx = this.ctx.ctx();
                    let sync_ctx = ops::SyncContext::new(&ctx, &src, &dst, confirm.as_deref())?;
                    let res = stream::iter(pairs)
                        .map(|(src_path, dst_path)| sync_ctx.sync(src_path, dst_path))
                        .buffered(4)
                        .try_fold(false, |res, copy_res| async move { Ok(res | copy_res) })
                        .await?;
                    Ok(res)
                })
                .await
            },
        );

        methods.add_async_method(
            "exec",
            |_, this, (uid, commands, opt): (String, String, ExecOptions)| async move {
                let ctx = this.ctx.ctx();
                external_error(ops::exec(&ctx, &uid, &commands, opt.reply, opt.etor)).await
            },
        );

        methods.add_async_method(
            "once",
            |_, this, (id, key, f): (String, String, Function)| async move {
                external_error(async {
                    let ctx = this.ctx.ctx();
                    let once = ops::Once::new(&ctx, &id, &key);
                    if !once.test().await? {
                        return Ok(Ok(false));
                    }
                    let res = f.call_async::<bool>(()).await;
                    once.set().await?;
                    Ok(res)
                })
                .await
            },
        );

        methods.add_async_method(
            "refresh",
            |_, this, (id, key): (String, String)| async move {
                external_error(ops::refresh(&this.ctx.ctx(), &id, &key)).await
            },
        );

        methods.add_async_method("os", |_, this, uid: String| async move {
            let ctx = this.ctx.ctx();
            let user = ctx.get_user(&uid).map_err(LuaError::external)?;
            Ok::<_, LuaError>(user.os().to_string())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::ExecOptions;
    use dv_api::process::ScriptExecutor;
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

    fn exec_options_des_suc_f(s: &str) -> ExecOptions {
        let lua = mlua::Lua::new();
        let val = lua.load(s).eval::<mlua::Value>().expect("Failed to load");
        ExecOptions::from_lua(val, &lua).expect("Failed to deserialize")
    }
    #[test]
    fn exec_options_serde() {
        let opt = exec_options_des_suc_f("true");
        assert!(opt.reply);
        assert!(opt.etor.is_none());

        let opt = exec_options_des_suc_f("{reply = false}");
        assert!(!opt.reply);
        assert!(opt.etor.is_none());

        let opt = exec_options_des_suc_f("{reply = true, etor = 'sh'}");
        assert!(opt.reply);
        assert_eq!(opt.etor, Some(ScriptExecutor::Sh));

        let opt = exec_options_des_suc_f("{reply = false, etor = 'bash'}");
        assert!(!opt.reply);
        assert_eq!(opt.etor, Some(ScriptExecutor::Bash));
    }
}
