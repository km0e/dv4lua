use super::dev::*;
use dv_wrap::ops::{DotConfig, DotUtil};
use tokio::sync;

pub struct Dot {
    ctx: Arc<RwLock<Context>>,
    dot: RwLock<DotUtil>,
}
impl Dot {
    pub fn new(ctx: Arc<RwLock<Context>>) -> Self {
        Self {
            ctx,
            dot: RwLock::new(DotUtil::default()),
        }
    }
    async fn lock(
        &self,
    ) -> (
        sync::RwLockReadGuard<'_, Context>,
        sync::RwLockWriteGuard<'_, DotUtil>,
    ) {
        let ctx = self.ctx.read().await;
        let dot = self.dot.write().await;
        (ctx, dot)
    }
}
impl UserData for Dot {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method_mut("confirm", |_, this, confirm: Option<String>| async move {
            this.dot.write().await.copy_action = confirm.unwrap_or_default();
            Ok(())
        });

        methods.add_async_method(
            "add_schema",
            |_, this, (user, path): (String, String)| async move {
                let (ctx, mut dot) = this.lock().await;
                external_error(dot.add_schema(&ctx, &user, &path)).await
            },
        );

        methods.add_async_method_mut(
            "add_source",
            |_, this, (user, path): (String, String)| async move {
                let (ctx, mut dot) = this.lock().await;
                external_error(dot.add_source(&ctx, &user, &path)).await
            },
        );

        methods.add_async_method(
            "sync",
            |_, this, (apps, dst): (Vec<String>, String)| async move {
                let (ctx, dot) = this.lock().await;
                external_error(dot.sync(&ctx, apps.into_iter().map(DotConfig::new).collect(), &dst))
                    .await
            },
        );
        methods.add_async_method(
            "upload",
            |_, this, (apps, dst): (Vec<String>, String)| async move {
                let (ctx, dot) = this.lock().await;
                external_error(dot.upload(
                    &ctx,
                    apps.into_iter().map(DotConfig::new).collect(),
                    &dst,
                ))
                .await
            },
        );
    }
}
