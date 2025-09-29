mod device;
mod mina;

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

pub use device::*;
pub use mina::*;
use utoipa::ToSchema;

/// `Mina` 服务请求的响应。
#[derive(Deserialize, Debug)]
pub struct MinaResponse {
    pub code: Number,
    pub message: String,
    pub data: Value,
}

impl MinaResponse {
    /// 验证响应的 `code`，如果不对，此函数将报错。
    pub fn error_for_code(self) -> crate::Result<Self> {
        if self.code == Number::from_i128(0).unwrap() {
            Ok(self)
        } else {
            Err(crate::Error::Server {
                code: self.code,
                message: self.message,
            })
        }
    }
}

/// `Mina` 设备信息。
#[derive(Clone, Serialize, Deserialize, Debug, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MinaDeviceInfo {
    #[serde(rename(deserialize = "deviceID"))]
    pub device_id: String,

    pub name: String,
    pub hardware: String,
}
