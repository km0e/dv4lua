#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("dv-wrap error: {0}")]
    DvWrap(#[from] dv_wrap::error::ErrorChain),
    #[error("unknown error: {0}")]
    Unknown(String),
}

impl From<Error> for mlua::Error {
    fn from(err: Error) -> Self {
        mlua::Error::external(err)
    }
}
