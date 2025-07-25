use std::collections::HashMap;

use dv_wrap::ops::{Package, Pm};
use mlua::{FromLua, Lua, MetaMethod, Table, UserData};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LuaPm(pub Pm);

impl FromLua for LuaPm {
    fn from_lua(lua_value: mlua::Value, _lua: &Lua) -> mlua::Result<Self> {
        let Some(Ok(s)) = lua_value.as_string().map(|s| s.to_str()) else {
            return Err(mlua::Error::FromLuaConversionError {
                from: lua_value.type_name(),
                to: "Pm".to_string(),
                message: Some("expected a string".to_string()),
            });
        };
        let pm = s.parse().map_err(|_| mlua::Error::FromLuaConversionError {
            from: "String",
            to: "Pm".to_string(),
            message: Some(format!("invalid package manager: {s}")),
        })?;
        Ok(LuaPm(pm))
    }
}

#[derive(Debug, Default, Clone)]
pub struct Packages {
    pm: HashMap<LuaPm, String>,
}

impl std::fmt::Display for Packages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.pm.is_empty() {
            write!(f, "empty")
        } else {
            for (pm, package) in &self.pm {
                write!(f, "{}:{} ", pm.0, package)?;
            }
            Ok(())
        }
    }
}

impl Packages {
    pub fn as_package(&self) -> Package {
        Package {
            pm: self.pm.iter().map(|(k, v)| (k.0, v.as_str())).collect(),
        }
    }
}

impl FromLua for Packages {
    fn from_lua(lua_value: mlua::Value, _lua: &Lua) -> mlua::Result<Self> {
        let Some(table) = lua_value.as_table() else {
            return Err(mlua::Error::FromLuaConversionError {
                from: lua_value.type_name(),
                to: "Packages".to_string(),
                message: Some("expected a table".to_string()),
            });
        };
        let mut pm = HashMap::new();
        for res in table.pairs::<LuaPm, String>() {
            let (key, value) = res?;
            pm.insert(key, value);
        }
        Ok(Packages { pm })
    }
}

impl UserData for Packages {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Add, |_lua, this, other: Packages| {
            let mut new = this.clone();
            for (k, v) in &other.pm {
                let entry = new.pm.entry(*k).or_default();
                if !entry.is_empty() {
                    entry.push(' ');
                }
                entry.push_str(v);
            }
            Ok(new)
        });
        methods.add_meta_method_mut(
            MetaMethod::NewIndex,
            |_lua, this, (key, value): (LuaPm, String)| {
                this.pm.insert(key, value);
                Ok(())
            },
        );
    }
}

pub fn register(lua: &Lua, _tb: &Table) -> mlua::Result<()> {
    lua.register_userdata_type(Packages::register)?;
    Ok(())
}
