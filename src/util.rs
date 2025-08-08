use mlua::Error;

/// constructs a FromLuaConversionError
pub fn conversion_error(
    from: &'static str,
    to: impl Into<String>,
    message: Option<impl Into<String>>,
) -> Error {
    Error::FromLuaConversionError {
        from,
        to: to.into(),
        message: message.map(|m| m.into()),
    }
}
pub async fn external_error<F, T>(f: F) -> Result<T, Error>
where
    F: std::future::Future<Output = Result<T, dv_wrap::error::Error>>,
{
    match f.await {
        Ok(v) => Ok(v),
        Err(e) => Err(Error::external(e)),
    }
}
