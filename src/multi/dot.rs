#![allow(clippy::await_holding_refcell_ref)]
use std::cell::RefCell;

use super::dev::*;
use dv_wrap::ops::{DotConfig, DotUtil};

pub struct Dot {
    ctx: ContextWrapper,
    dot: RefCell<DotUtil>,
}
impl Dot {
    pub fn new(ctx: ContextWrapper) -> Self {
        Self {
            ctx,
            dot: RefCell::default(),
        }
    }
}
impl UserData for Dot {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method_mut(
            "confirm",
            |_, mut this, confirm: Option<String>| async move {
                this.dot.get_mut().copy_action = confirm.unwrap_or_default();
                Ok(())
            },
        );

        methods.add_async_method(
            "add_schema",
            |_, this, (user, path): (String, String)| async move {
                external_error(
                    this.dot
                        .borrow_mut()
                        .add_schema(&this.ctx.ctx(), &user, &path),
                )
                .await
            },
        );

        methods.add_async_method_mut(
            "add_source",
            |_, this, (user, path): (String, String)| async move {
                external_error(
                    this.dot
                        .borrow_mut()
                        .add_source(&this.ctx.ctx(), &user, &path),
                )
                .await
            },
        );

        methods.add_async_method(
            "sync",
            |_, this, (apps, dst): (Vec<String>, String)| async move {
                external_error(this.dot.borrow_mut().sync(
                    &this.ctx.ctx(),
                    apps.into_iter().map(DotConfig::new).collect(),
                    &dst,
                ))
                .await
            },
        );
        methods.add_async_method(
            "upload",
            |_, this, (apps, dst): (Vec<String>, String)| async move {
                external_error(this.dot.borrow_mut().upload(
                    &this.ctx.ctx(),
                    apps.into_iter().map(DotConfig::new).collect(),
                    &dst,
                ))
                .await
            },
        );
    }
}
