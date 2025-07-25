use std::{collections::HashMap, path::Path};

use dv_api::process::Interactor;
use dv_wrap::{
    Context, DeviceInfo, SqliteCache, TermInteractor, User,
    ops::{DotConfig, DotUtil, Pm},
};
use mlua::Error as LuaError;

use mlua::{Table, UserData, UserDataMethods, Value};
use tokio::sync::Mutex;

#[derive(Debug)]
struct Device {
    pub info: DeviceInfo,
    system: Option<String>,
    users: Vec<String>,
}

impl Device {
    pub fn new(info: DeviceInfo) -> Self {
        Self {
            info,
            system: None,
            users: Vec::new(),
        }
    }
}

use crate::{multi::Packages, util::conversion_error};

use super::*;

pub struct Dv {
    dry_run: bool,
    devices: HashMap<String, Device>,
    users: HashMap<String, User>,
    cache: SqliteCache,
    interactor: TermInteractor,
    dotutils: Mutex<DotUtil>,
}

impl UserData for Dv {
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
                    let ctx = dv_wrap::ops::CopyContext::new(
                        this.context(),
                        &src,
                        &dst,
                        confirm.as_deref(),
                    )?;
                    let mut res = false;
                    for (src_path, dst_path) in pairs {
                        res |= ctx.copy(src_path, dst_path).await?;
                    }
                    Ok::<_, error::Error>(res)
                }
                .await
                .map_err(LuaError::external)
            },
        );

        methods.add_async_method(
            "exec",
            |_, this, (uid, shell, commands): (String, Option<String>, String)| async move {
                dv_wrap::ops::exec(&this.context(), &uid, shell.as_deref(), &commands)
                    .await
                    .map_err(LuaError::external)
            },
        );

        methods.add_async_method(
            "once",
            |_, this, (id, key, f): (String, String, Function)| async move {
                let ret: Result<_, error::Error> = async {
                    let once = dv_wrap::ops::Once::new(this.context(), &id, &key);
                    if !once.test().await? {
                        return Ok::<_, error::Error>(Ok(false));
                    }
                    let res = f.call_async::<bool>(()).await;
                    once.set().await?;
                    Ok::<_, error::Error>(res)
                }
                .await;
                ret?
            },
        );

        methods.add_async_method(
            "refresh",
            |_, this, (id, key): (String, String)| async move {
                dv_wrap::ops::refresh(this.context(), &id, &key)
                    .await
                    .map_err(LuaError::external)
            },
        );

        methods.add_async_method_mut(
            "add_user",
            |_, mut this, (id, obj): (String, Table)| async move {
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

                async {
                    let uid = id.clone();
                    if this.users.contains_key(&uid) {
                        return Err(error::Error::Unknown(format!("user {uid} already exists",)));
                    }
                    let hid = cfg.get("hid").cloned();
                    let u = User::new(cfg).await?;
                    if let Some(hid) = hid {
                        let hid = hid.to_string();
                        let dev = match this.devices.get_mut(&hid) {
                            Some(dev) => dev,
                            None => {
                                let dev = Device::new(DeviceInfo::detect(&u, u.os()).await?);
                                this.devices.insert(hid.clone(), dev);
                                this.devices.get_mut(&hid).unwrap()
                            }
                        };
                        if u.is_system {
                            dev.system = Some(uid);
                        } else {
                            dev.users.push(uid);
                        }
                    };
                    this.interactor
                        .log(format!("user: {:<10}, os: {:<8}", id.as_str(), u.os()))
                        .await;
                    this.users.insert(id, u);
                    Ok(())
                }
                .await
                .map_err(LuaError::external)
            },
        );

        methods.add_async_method_mut("dot", |_, this, confirm: Option<String>| async move {
            *this.dotutils.lock().await = DotUtil::new(confirm);
            Ok(())
        });

        methods.add_async_method(
            "dot_add_schema",
            |_, this, (user, path): (String, String)| async move {
                this.dotutils
                    .lock()
                    .await
                    .add_schema(this.context(), &user, &path)
                    .await
                    .map_err(LuaError::external)
            },
        );

        methods.add_async_method_mut(
            "dot_add_source",
            |_, this, (user, path): (String, String)| async move {
                this.dotutils
                    .lock()
                    .await
                    .add_source(this.context(), &user, &path)
                    .await
                    .map_err(LuaError::external)
            },
        );

        methods.add_async_method(
            "dot_sync",
            |_, this, (apps, dst): (Vec<String>, String)| async move {
                this.dotutils
                    .lock()
                    .await
                    .sync(
                        this.context(),
                        apps.into_iter().map(DotConfig::new).collect(),
                        &dst,
                    )
                    .await
                    .map_err(LuaError::external)
            },
        );

        methods.add_async_method(
            "pm",
            |_, this, (uid, packages): (String, Packages)| async move {
                async {
                    let user = this.context().get_user(&uid)?;
                    let pm = match this.devices.get(&uid).map(|dev| dev.info.pm) {
                        Some(pm) => pm,
                        None => Pm::detect(user, &user.os()).await?,
                    };
                    let res = packages
                        .as_package()
                        .install(this.context(), &uid, &pm)
                        .await?;
                    Ok::<_, error::Error>(res)
                }
                .await
                .map_err(LuaError::external)
            },
        );
    }
}

impl Dv {
    pub fn new(path: impl AsRef<Path>, dry_run: bool) -> Self {
        Self {
            dry_run,
            devices: HashMap::new(),
            users: HashMap::new(),
            cache: SqliteCache::new(path),
            interactor: TermInteractor::new().unwrap(),
            dotutils: Mutex::new(DotUtil::new(None)),
        }
    }
    pub fn context(&self) -> Context<'_> {
        Context::new(self.dry_run, &self.cache, &self.interactor, &self.users)
    }
}
