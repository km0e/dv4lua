use dv_wrap::{
    Context,
    ops::{DotConfig, DotUtil},
};
use mlua::{Error as LuaError, UserData, UserDataMethods};
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

pub struct Dot {
    ctx: Arc<Mutex<Context>>,
    dot: Mutex<DotUtil>,
}
impl Dot {
    pub fn new(ctx: Arc<Mutex<Context>>) -> Self {
        Self {
            ctx,
            dot: Mutex::new(DotUtil::default()),
        }
    }
    async fn lock(&self) -> (MutexGuard<Context>, MutexGuard<DotUtil>) {
        let ctx = self.ctx.lock().await;
        let dot = self.dot.lock().await;
        (ctx, dot)
    }
}
impl UserData for Dot {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method_mut("confirm", |_, this, confirm: Option<String>| async move {
            *this.dot.lock().await = DotUtil::new(confirm);
            Ok(())
        });

        methods.add_async_method(
            "add_schema",
            |_, this, (user, path): (String, String)| async move {
                let (ctx, mut dot) = this.lock().await;
                dot.add_schema(&ctx, &user, &path)
                    .await
                    .map_err(LuaError::external)
            },
        );

        methods.add_async_method_mut(
            "add_source",
            |_, this, (user, path): (String, String)| async move {
                let (ctx, mut dot) = this.lock().await;
                dot.add_source(&ctx, &user, &path)
                    .await
                    .map_err(LuaError::external)
            },
        );

        methods.add_async_method(
            "sync",
            |_, this, (apps, dst): (Vec<String>, String)| async move {
                let (ctx, dot) = this.lock().await;
                dot.sync(&ctx, apps.into_iter().map(DotConfig::new).collect(), &dst)
                    .await
                    .map_err(LuaError::external)
            },
        );
    }
}
