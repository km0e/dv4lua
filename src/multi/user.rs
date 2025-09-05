#![allow(clippy::await_holding_refcell_ref)]

use super::dev::*;
use dv_api::process::ScriptExecutor;
use dv_wrap::User;
use dv_wrap::ops;
use mlua::{FromLua, LuaSerdeExt, Table, Value};
use tracing::debug;

pub struct UserWrapper {
    ctx: ContextWrapper,
    uid: String,
}

impl UserWrapper {
    pub fn new(ctx: ContextWrapper, uid: String) -> Self {
        Self { ctx, uid }
    }
}

#[derive(serde::Deserialize, Default)]
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
impl UserData for UserWrapper {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method(
            "exec",
            |_, this, (commands, opt): (String, Option<ExecOptions>)| async move {
                let opt = opt.unwrap_or_default();
                let ctx = this.ctx.ctx();
                external_error(ops::exec(&ctx, &this.uid, &commands, opt.reply, opt.etor)).await
            },
        );
        methods.add_async_method(
            "write",
            |_, this, (path, content): (String, String)| async move {
                let ctx = this.ctx.ctx();
                external_error(ops::write(&ctx, &this.uid, &path, &content)).await
            },
        );
        methods.add_async_method("read", |_, this, path: String| async move {
            let ctx = this.ctx.ctx();
            external_error(ops::read(&ctx, &this.uid, &path)).await
        });
        methods.add_meta_method(mlua::MetaMethod::Index, |_, this, key: String| {
            let ctx = this.ctx.ctx();
            let user = ctx.get_user(&this.uid).expect("User must exist");
            if let Some(value) = user.vars.get(&key) {
                return Ok(Some(value.clone()));
            }
            Ok(None)
        });
    }
}

pub struct UserManager {
    ctx: ContextWrapper,
}

impl UserManager {
    pub fn new(ctx: ContextWrapper) -> Self {
        Self { ctx }
    }
}

impl std::ops::Deref for UserManager {
    type Target = ContextWrapper;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl std::ops::DerefMut for UserManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
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

        methods.add_async_method_mut(
            "add_cur",
            async move |_, this, obj: Table| -> mlua::Result<bool> {
                let mut cfg = add_user_prepare(obj)?;
                external_error(async {
                    let mut ctx = this.ctx_mut();
                    if ctx.contains_user("cur") {
                        return Ok(false);
                    }
                    cfg.set("hid", "local");
                    ctx.add_user("cur".to_string(), User::local(cfg).await?)
                        .await
                        .map(|_| true)
                })
                .await
            },
        );
        methods.add_async_method_mut(
            "add_ssh",
            async move |_, this, (uid, obj): (String, Table)| -> mlua::Result<bool> {
                let mut cfg = add_user_prepare(obj)?;
                external_error(async {
                    let mut ctx = this.ctx_mut();
                    if ctx.contains_user(&uid) {
                        return Ok(false);
                    }
                    cfg.set("host", &uid);
                    ctx.add_user(uid, User::ssh(cfg).await?).await.map(|_| true)
                })
                .await
            },
        );

        methods.add_meta_method(
            mlua::MetaMethod::Index,
            |_, this, key: String| -> mlua::Result<Option<UserWrapper>> {
                debug!("Accessing user: {}", key);
                if !this.ctx().contains_user(&key) {
                    return Ok(None);
                }
                Ok(Some(UserWrapper::new(this.ctx.clone(), key)))
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::ExecOptions;
    use dv_api::process::ScriptExecutor;
    use mlua::FromLua;

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
