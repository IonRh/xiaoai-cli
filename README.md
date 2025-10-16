# miai

[![Crates.io Version](https://img.shields.io/crates/v/miai)](https://crates.io/crates/miai)
[![docs.rs](https://img.shields.io/docsrs/miai)](https://docs.rs/miai)
[![Crates.io License](https://img.shields.io/crates/l/miai)](/LICENSE)

调用你的小米、小爱音箱，或其他任何支持的小爱设备。

灵感和实现思路源于 [miservice_fork](https://github.com/yihong0618/MiService)，但主要聚焦于小爱音箱这一设备。

## 主要功能

- 播报文本。
- 播放音乐。
- 调整音量。
- 控制播放状态。
- 执行文本（询问小爱）。
- 提供底层接口，或许能帮助你发现更多！

## 如何使用

```rust
use miai::{PlayState, Xiaoai};

#[tokio::main]
async fn main() {
    // 登录你的账号
    let xiaoai = Xiaoai::login("username", "password").await.unwrap();

    // 查询你的设备信息
    let device_info = xiaoai.device_info().await.unwrap();

    for info in device_info {
        // device_id 为请求指明目标设备
        let device_id = info.device_id;

        // 让设备播报文本
        xiaoai.tts(&device_id, "你好！").await.unwrap();

        // 提供一个链接，让设备播放音乐
        xiaoai
            .play_url(&device_id, "http://music-url")
            .await
            .unwrap();

        // 控制小爱的播放状态，比如让它停止
        xiaoai
            .set_play_state(&device_id, PlayState::Stop)
            .await
            .unwrap();

        // 让小爱执行文本，效果就跟口头询问一样
        xiaoai.nlp(&device_id, "查询今天的天气").await.unwrap();

        // 还可以进行低层次的请求，比如 Ubus Call
        let response = xiaoai
            .ubus_call(&device_id, "mibrain", "nlp_result_get", "{}")
            .await
            .unwrap();

        // 通过响应体了解请求的结果
        println!("{}", response.data);
    }
}
```

## 更多示例

参见 [examples](/miai/examples/) 文件夹以获得更多示例。

## 许可证

本项目通过 [MIT license](/LICENSE) 授权。
