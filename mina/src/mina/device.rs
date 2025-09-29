use std::{collections::HashMap, sync::Arc};

use log::debug;
use serde_json::json;

use super::{Mina, MinaDeviceInfo, MinaResponse};

/// 提供设备相关的 `Mina` 服务请求。
#[derive(Debug)]
pub struct MinaDevice {
    mina: Arc<Mina>,
    info: MinaDeviceInfo,
}

impl MinaDevice {
    pub(super) fn new(mina: Arc<Mina>, info: MinaDeviceInfo) -> Self {
        Self { mina, info }
    }

    /// 获取设备信息的引用。
    pub fn info(&self) -> &MinaDeviceInfo {
        &self.info
    }

    /// 获取设备信息。
    pub fn into_info(self) -> MinaDeviceInfo {
        self.info
    }

    /// 向设备发送 `ubus` 请求。
    pub async fn ubus(
        &self,
        method: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> crate::Result<MinaResponse> {
        let form = HashMap::from([
            ("deviceId", self.info.device_id.to_string()),
            ("method", method.into()),
            ("path", path.into()),
            ("message", message.into()),
        ]);

        self.mina
            .post("remote/ubus", form)
            .await?
            .error_for_status()?
            .json::<MinaResponse>()
            .await?
            .error_for_code()
    }

    /// 请求设备播放 `url`。
    pub async fn play_url(&self, url: &str) -> crate::Result<MinaResponse> {
        if matches!(
            self.info.hardware.as_str(),
            "LX04" | "L05B" | "L05C" | "L06" | "L06A" | "X08A" | "X10A",
        ) {
            debug!("使用 player_play_music method");
            self.play_music(url).await
        } else {
            debug!("使用 player_play_url method");
            self.ubus(
                "player_play_url",
                "mediaplayer",
                json!({
                    "url": url,
                    "type": 2,
                    "media": "app_ios"
                })
                .to_string(),
            )
            .await
        }
    }

    async fn play_music(&self, url: &str) -> crate::Result<MinaResponse> {
        const AUDIO_ID: &str = "1582971365183456177";
        const ID: &str = "355454500";
        let music = json!({
            "payload": {
                "audio_type": "",
                "audio_items": [
                    {
                        "item_id": {
                            "audio_id": AUDIO_ID,
                            "cp": {
                                "album_id": "-1",
                                "episode_index": 0,
                                "id": ID,
                                "name": "xiaowei",
                            },
                        },
                        "stream": {"url": url},
                    }
                ],
                "list_params": {
                    "listId": "-1",
                    "loadmore_offset": 0,
                    "origin": "xiaowei",
                    "type": "MUSIC",
                },
            },
            "play_behavior": "REPLACE_ALL",
        });

        self.ubus(
            "player_play_music",
            "mediaplayer",
            json!({
                "startaudioid": AUDIO_ID,
                "music": music
            })
            .to_string(),
        )
        .await
    }
}
