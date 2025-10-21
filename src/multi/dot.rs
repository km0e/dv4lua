use crate::util::sync_opts;

use super::dev::*;
use dv_wrap::ops::{DotConfig, DotUtil};

pub struct Dot {
    dot: DotUtil<ContextWrapper>,
}
impl Dot {
    pub fn new(ctx: ContextWrapper) -> Self {
        Self {
            dot: DotUtil::new(ctx, Vec::new()),
        }
    }
}

impl UserData for Dot {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method_mut(
            "confirm",
            |_, mut this, confirm: Option<String>| async move {
                this.dot.copy_action = sync_opts(&confirm.unwrap_or_default())?;
                Ok(())
            },
        );

        methods.add_async_method_mut(
            "add_schema",
            |_, mut this, (user, path): (String, String)| async move {
                Ok(this.dot.add_schema(&user, &path).await?)
            },
        );

        methods.add_async_method_mut(
            "add_source",
            |_, mut this, (user, path): (String, String)| async move {
                Ok(this.dot.add_source(&user, &path).await)
            },
        );

        methods.add_async_method(
            "sync",
            |_, this, (apps, dst): (Vec<String>, String)| async move {
                let entries = this
                    .dot
                    .sync(apps.into_iter().map(DotConfig::new).collect(), &dst)
                    .await?;
                let mut res = false;
                for e in &entries {
                    res |= this.dot.ctx.sync_impl(&e.src, &e.dst, &e.entries).await?;
                }
                Ok(res)
            },
        );
        methods.add_async_method(
            "upload",
            |_, this, (apps, dst): (Vec<String>, String)| async move {
                let entries = this
                    .dot
                    .upload(apps.into_iter().map(DotConfig::new).collect(), &dst)
                    .await?;
                let mut res = false;
                for e in &entries {
                    res |= this.dot.ctx.sync_impl(&e.src, &e.dst, &e.entries).await?;
                }
                Ok(res)
            },
        );
    }
}
