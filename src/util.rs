use anyhow::bail;
use dv_wrap::ops::SyncOpt;
use mlua::Error;

pub fn sync_opts(s: &str) -> dv_wrap::Result<Vec<SyncOpt>> {
    let mut opts = Vec::new();
    for c in s.chars() {
        if !c.is_ascii_digit() {
            bail!("Invalid confirm option: {}", c);
        }
        opts.push(SyncOpt::from_bits_retain((1 << (c as u8 - b'0')) >> 1));
    }
    Ok(opts)
}

/// constructs a FromLuaConversionError
pub fn conversion_error(
    from: &'static str,
    to: impl Into<String>,
    message: Option<impl ToString>,
) -> Error {
    Error::FromLuaConversionError {
        from,
        to: to.into(),
        message: message.map(|m| m.to_string()),
    }
}
