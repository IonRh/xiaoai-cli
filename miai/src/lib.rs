mod error;
mod xiaoai;

use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;

pub use error::*;
pub use xiaoai::*;

/// 小爱服务请求的响应。
#[derive(Clone, Deserialize, Debug)]
pub struct XiaoaiResponse<T = Value> {
    /// 错误码。
    /// 
    /// 非 0 的错误码表示当前请求出错了。
    pub code: i64,

    /// 一条简短的消息。
    /// 
    /// 常用于定位错误，当请求成功时，用处不大。
    pub message: String,

    /// 返回的实际数据。
    /// 
    /// 当请求发生错误时，无法保证返回的数据。
    /// 建议在解析数据前，先使用 [`XiaoaiResponse::error_for_code`] 校验错误码。
    pub data: T,
}

impl XiaoaiResponse {
    /// 验证响应的 `code`，如果不对，此函数将报错。
    /// 
    /// # Errors
    /// 
    /// `code` 不对时，将返回 [`Error::Api`]。
    /// 
    /// # Examples
    /// 
    /// ```no_run
    /// # use miai::XiaoaiResponse;
    /// fn on_response(res: XiaoaiResponse) {
    ///     match res.error_for_code() {
    ///         Ok(res) => assert_eq!(res.code, 0),
    ///         Err(_err) => ()
    ///     }
    /// }
    /// ```
    pub fn error_for_code(self) -> crate::Result<Self> {
        if self.code == 0 {
            Ok(self)
        } else {
            Err(crate::Error::Api(self))
        }
    }

    /// 提取响应的 `data` 并反序列化。
    /// 
    /// # Errors
    /// 
    /// 当 `data` 不能反序列化为 `T` 时报错，详见 [`serde_json::from_value`]。
    pub fn extract_data<T: DeserializeOwned>(self) -> crate::Result<T> {
        Ok(serde_json::from_value(self.data)?)
    }
}
