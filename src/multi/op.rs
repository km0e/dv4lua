use dv_wrap::{Context, User, ops};
use mlua::{Error as LuaError, Function, Table, UserData, UserDataMethods, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::util::conversion_error;

pub struct Op {
    ctx: Arc<Mutex<Context>>,
}

impl Op {
    pub fn new(ctx: Arc<Mutex<Context>>) -> Self {
        Self { ctx }
    }
}

impl UserData for Op {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method(
            "copy",
            |_,
             this,
             (src, src_path, dst, dst_path, confirm): (
                String,
                Value,
                String,
                Value,
                Option<String>,
            )| async move {
                let pairs = match (src_path, dst_path) {
                    (Value::String(src_path), Value::String(dst_path)) => {
                        Ok(vec![(src_path, dst_path)])
                    }
                    (Value::Table(src_paths), Value::Table(dst_paths)) => src_paths
                        .sequence_values::<mlua::String>()
                        .zip(dst_paths.sequence_values::<mlua::String>())
                        .map(|(src, dst)| Ok((src?, dst?)))
                        .collect::<Result<Vec<_>, _>>(),
                    _ => Err(conversion_error(
                        "Value",
                        "String or Table",
                        Some("src_path and dst_path must be String or Table"),
                    )),
                }?;
                let pairs = pairs
                    .iter()
                    .map(|(src, dst)| Ok::<_, LuaError>((src.to_str()?, dst.to_str()?)))
                    .collect::<Result<Vec<_>, _>>()?;
                async {
                    let ctx = this.ctx.lock().await;
                    let ctx = ops::CopyContext::new(&ctx, &src, &dst, confirm.as_deref())?;
                    let mut res = false;
                    for (src_path, dst_path) in pairs {
                        res |= ctx.copy(src_path, dst_path).await?;
                    }
                    Ok::<_, dv_wrap::error::Error>(res)
                }
                .await
                .map_err(LuaError::external)
            },
        );

        methods.add_async_method(
            "exec",
            |_, this, (uid, commands, shell): (String, String, Option<String>)| async move {
                let ctx = this.ctx.lock().await;
                ops::exec(&ctx, &uid, shell.as_deref(), &commands)
                    .await
                    .map_err(LuaError::external)
            },
        );

        methods.add_async_method(
            "once",
            |_, this, (id, key, f): (String, String, Function)| async move {
                let ret: Result<_, dv_wrap::error::Error> = async {
                    let ctx = this.ctx.lock().await;
                    let once = ops::Once::new(&ctx, &id, &key);
                    if !once.test().await? {
                        return Ok(Ok(false));
                    }
                    let res = f.call_async::<bool>(()).await;
                    once.set().await?;
                    Ok(res)
                }
                .await;
                ret.map_err(LuaError::external)?
            },
        );

        methods.add_async_method(
            "refresh",
            |_, this, (id, key): (String, String)| async move {
                let ctx = this.ctx.lock().await;
                ops::refresh(&ctx, &id, &key)
                    .await
                    .map_err(LuaError::external)
            },
        );

        fn add_user_prepare(obj: Table) -> mlua::Result<dv_api::multi::Config> {
            let mut cfg = dv_api::multi::Config::default();
            for v in obj.pairs::<String, Value>() {
                let (name, value) = v?;
                if name == "is_system" && value.is_boolean() {
                    cfg.is_system = value.as_boolean();
                    continue;
                }
                let Some(value) = value.as_string() else {
                    continue;
                };
                cfg.set(name, value.to_str()?.to_string());
            }
            Ok(cfg)
        }

        methods.add_async_method_mut("add_cur", |_, this, obj: Table| async move {
            let mut cfg = add_user_prepare(obj)?;
            async {
                let mut ctx = this.ctx.lock().await;
                if ctx.contains_user("cur") {
                    return Err(dv_wrap::error::Error::unknown("user cur already exists"));
                }
                cfg.set("hid", "local");
                ctx.add_user("cur".to_string(), User::local(cfg).await?)
                    .await
            }
            .await
            .map_err(LuaError::external)
        });
        methods.add_async_method_mut(
            "add_ssh",
            |_, this, (uid, obj): (String, Table)| async move {
                let mut cfg = add_user_prepare(obj)?;
                async {
                    let mut ctx = this.ctx.lock().await;
                    if ctx.contains_user(&uid) {
                        return Err(dv_wrap::error::Error::unknown(format!(
                            "user {uid} already exists",
                        )));
                    }
                    cfg.set("hid", "local");
                    cfg.set("host", &uid);
                    ctx.add_user(uid, User::ssh(cfg).await?).await
                }
                .await
                .map_err(LuaError::external)
            },
        );

        methods.add_async_method("os", |_, this, uid: String| async move {
            let ctx = this.ctx.lock().await;
            let user = ctx.get_user(&uid).map_err(LuaError::external)?;
            Ok::<_, LuaError>(user.os().to_string())
        });
    }
}
