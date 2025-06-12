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
