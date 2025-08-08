use super::dev::*;
use dv_wrap::ops::Pm as OpPm;
use std::ops::Deref;

use dv_api::whatever;

pub struct Pm {
    ctx: Arc<RwLock<Context>>,
}
impl Pm {
    pub fn new(ctx: Arc<RwLock<Context>>) -> Self {
        Self { ctx }
    }
}

async fn with_pm<'a: 'b, 'b, F, Fut, R>(
    ctx: &'a dv_wrap::Context,
    device: &str,
    f: F,
) -> Result<R, mlua::Error>
where
    F: FnOnce(&'a OpPm, &'a str, &'a dv_wrap::Context) -> Fut,
    Fut: std::future::Future<Output = Result<R, dv_wrap::error::Error>> + 'b,
{
    external_error(async {
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
    })
    .await
}

impl UserData for Pm {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method_mut(
            "install",
            |_, this, (device, packages): (String, String)| async move {
                with_pm(this.ctx.read().await.deref(), &device, |pm, target, ctx| {
                    pm.install(ctx, target, &packages, true)
                })
                .await
            },
        );
        methods.add_async_method_mut("update", |_, this, device: String| async move {
            with_pm(this.ctx.read().await.deref(), &device, |pm, target, ctx| {
                pm.update(ctx, target, true)
            })
            .await
        });
        methods.add_async_method_mut(
            "upgrade",
            |_, this, (device, packages): (String, String)| async move {
                with_pm(this.ctx.read().await.deref(), &device, |pm, target, ctx| {
                    pm.upgrade(ctx, target, &packages, true)
                })
                .await
            },
        );
    }
}
