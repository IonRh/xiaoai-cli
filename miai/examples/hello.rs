//! 让小爱打个招呼！
//!
//! 该示例演示了 [`Xiaoai`] 的基本用法，可用于快速上手。

use std::env;

use miai::Xiaoai;

// 进行网络请求需要 `tokio` 运行时：`cargo add tokio --features macros`
// 这里使用单线程运行时，以方便测试。
#[tokio::main(flavor = "current_thread")]
async fn main() {
    // 初始化日志，主要用于测试目的，不需要可以去掉。
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // 从 `.env` 加载环境变量，主要用于测试目的，不需要可以去掉。
    let _ = dotenvy::dotenv();

    // 从环境变量加载账号密码
    let username = env::var("MI_USER").expect("env::var");
    let password = env::var("MI_PASS").expect("env::var");
    let xiaoai = Xiaoai::login(&username, &password).await.expect("login");
    println!("登录成功！");

    let devices = xiaoai.device_info().await.expect("device_info");
    if devices.is_empty() {
        println!("未发现小爱设备，请确保设备已在小米音箱 APP 中绑定！");
    } else {
        for device in devices {
            println!("发现小爱设备 {}，让它打个招呼。", device.name);
            let text = format!("你好，{username}！我是 {}。", device.name);
            let response = xiaoai.tts(&device.device_id, &text).await.expect("tts");
            println!("{} 回复: {}", device.name, response.message);
        }
    }
}
