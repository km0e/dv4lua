use dv_api::whatever;
use dv_wrap::{Context, ops::Pm as OpPm};
use mlua::{Error as LuaError, UserData, UserDataMethods};
use std::{ops::Deref, sync::Arc};
use tokio::sync::Mutex;

pub struct Pm {
    ctx: Arc<Mutex<Context>>,
}
impl Pm {
    pub fn new(ctx: Arc<Mutex<Context>>) -> Self {
        Self { ctx }
    }
}

async fn with_pm<'a: 'b, 'b, F, Fut, R>(
    ctx: &'a dv_wrap::Context,
    device: &str,
    f: F,
) -> Result<R, dv_wrap::error::Error>
where
    F: FnOnce(&'a OpPm, &'a str, &'a dv_wrap::Context) -> Fut,
    Fut: std::future::Future<Output = Result<R, dv_wrap::error::Error>> + 'b,
{
    let Some(dev) = ctx.devices.get(device) else {
        whatever!("Device {device} not found in context")
    };

    if let Some(sys) = &dev.system {
        f(&dev.info.pm, sys, ctx).await
    } else if let Some(user) = dev.users.first() {
        f(&dev.info.pm, user, ctx).await
    } else {
        whatever!("Device {device} has no system or user defined")
    }
}

impl UserData for Pm {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method_mut(
            "install",
            |_, this, (device, packages): (String, String)| async move {
                with_pm(this.ctx.lock().await.deref(), &device, |pm, target, ctx| {
                    pm.install(ctx, target, &packages, true)
                })
                .await
                .map_err(LuaError::external)
            },
        );
        methods.add_async_method_mut("update", |_, this, device: String| async move {
            with_pm(this.ctx.lock().await.deref(), &device, |pm, target, ctx| {
                pm.update(ctx, target, true)
            })
            .await
            .map_err(LuaError::external)
        });
        methods.add_async_method_mut(
            "upgrade",
            |_, this, (device, packages): (String, String)| async move {
                with_pm(this.ctx.lock().await.deref(), &device, |pm, target, ctx| {
                    pm.upgrade(ctx, target, &packages, true)
                })
                .await
                .map_err(LuaError::external)
            },
        );
    }
}
