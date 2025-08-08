use super::dev::*;
use dv_wrap::ops;
use futures::{StreamExt, TryStreamExt, stream};
use mlua::{Error as LuaError, Function, Value};

use crate::util::conversion_error;

pub struct Op {
    ctx: Arc<RwLock<Context>>,
}

impl Op {
    pub fn new(ctx: Arc<RwLock<Context>>) -> Self {
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
                external_error(async {
                    let ctx = this.ctx.read().await;
                    let ctx = ops::CopyContext::new(&ctx, &src, &dst, confirm.as_deref())?;
                    let res = stream::iter(pairs)
                        .map(|(src_path, dst_path)| ctx.copy(src_path, dst_path))
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
            |_, this, (uid, commands, tty): (String, String, Option<bool>)| async move {
                let ctx = this.ctx.read().await;
                external_error(ops::exec(&ctx, &uid, &commands, tty.unwrap_or(true))).await
            },
        );

        methods.add_async_method(
            "once",
            |_, this, (id, key, f): (String, String, Function)| async move {
                external_error(async {
                    let ctx = this.ctx.read().await;
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
                let ctx = this.ctx.read().await;
                external_error(ops::refresh(&ctx, &id, &key)).await
            },
        );

        methods.add_async_method("os", |_, this, uid: String| async move {
            let ctx = this.ctx.read().await;
            let user = ctx.get_user(&uid).map_err(LuaError::external)?;
            Ok::<_, LuaError>(user.os().to_string())
        });
    }
}
