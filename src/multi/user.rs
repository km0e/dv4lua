use dv_wrap::{Context, User};
use mlua::{Error as LuaError, Table, UserData, UserDataMethods, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct UserManager {
    ctx: Arc<Mutex<Context>>,
}
impl UserManager {
    pub fn new(ctx: Arc<Mutex<Context>>) -> Self {
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
    }
}
