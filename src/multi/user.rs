#![allow(clippy::await_holding_refcell_ref)]
use super::dev::*;
use dv_wrap::User;
use mlua::{Table, Value};

pub struct UserManager {
    ctx: ContextWrapper,
}
impl UserManager {
    pub fn new(ctx: ContextWrapper) -> Self {
        Self { ctx }
    }
}

impl UserData for UserManager {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
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
            external_error(async {
                let mut ctx = this.ctx.ctx_mut();
                if ctx.contains_user("cur") {
                    dv_api::whatever!("user cur already exists");
                }
                cfg.set("hid", "local");
                ctx.add_user("cur".to_string(), User::local(cfg).await?)
                    .await
            })
            .await
        });
        methods.add_async_method_mut(
            "add_ssh",
            |_, this, (uid, obj): (String, Table)| async move {
                let mut cfg = add_user_prepare(obj)?;
                external_error(async {
                    let mut ctx = this.ctx.ctx_mut();
                    if ctx.contains_user(&uid) {
                        dv_api::whatever!("user {uid} already exists");
                    }
                    cfg.set("host", &uid);
                    ctx.add_user(uid, User::ssh(cfg).await?).await
                })
                .await
            },
        );
    }
}
