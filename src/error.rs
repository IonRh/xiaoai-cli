use serde_json::Number;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("请求失败")]
    Request(#[from] reqwest::Error),

    #[error("json 解析失败")]
    Parse(#[from] serde_json::Error),

    #[error("Cookie 出现问题")]
    Cookie(#[from] cookie_store::CookieError),

    #[error("服务端返回 {code}: {message:?}")]
    Server { code: Number, message: String },
}
