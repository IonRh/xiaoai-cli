use std::{borrow::Cow, fmt::Display, fs::File, io::BufReader, path::PathBuf};

use anyhow::ensure;
use clap::{Parser, Subcommand};
use inquire::{Confirm, Password, PasswordDisplayMode, Select, Text};
use miai::{DeviceInfo, Xiaoai};
use url::Url;

const DEFAULT_AUTH_FILE: &str = "xiaoai-auth.json";

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// 指定认证文件
    #[arg(long, default_value = DEFAULT_AUTH_FILE)]
    auth_file: PathBuf,

    /// 指定设备 ID
    #[arg(short, long)]
    device_id: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// 登录以获得认证
    Login,
    /// 列出设备
    Device,
    /// 播报文本
    Say {
        text: String,
    },
    Play {
        url: Url,
    },
}

impl Cli {
    fn xiaoai(&self) -> anyhow::Result<Xiaoai> {
        let file = File::open(&self.auth_file)?;

        Xiaoai::load(BufReader::new(file)).map_err(anyhow::Error::from_boxed)
    }

    /// 获取用户指定的设备 ID。
    ///
    /// 如果用户没有在命令行指定，则会向服务器请求设备列表。
    /// 如果请求结果只有一个设备，会自动选择这个唯一的设备。
    /// 如果请求结果存在多个设备，则会让用户自行选择。
    async fn device_id(&'_ self, xiaoai: &Xiaoai) -> anyhow::Result<Cow<'_, str>> {
        if let Some(device_id) = &self.device_id {
            return Ok(device_id.into());
        }

        let info = xiaoai.device_info().await?;
        ensure!(!info.is_empty(), "无可用设备，需要在小米音箱 APP 中绑定");
        if info.len() == 1 {
            return Ok(info[0].device_id.clone().into());
        }

        let options = info.into_iter().map(DisplayDeviceInfo).collect();
        let ans = Select::new("目标设备?", options).prompt()?;

        Ok(ans.0.device_id.into())
    }
}

struct DisplayDeviceInfo(DeviceInfo);

impl Display for DisplayDeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "名称: {}", self.0.name)?;
        writeln!(f, "设备 ID: {}", self.0.device_id)?;
        writeln!(f, "机型: {}", self.0.hardware)
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Commands::Login = cli.command {
        let username = Text::new("账号:").prompt()?;
        let password = Password::new("密码:")
            .with_display_toggle_enabled()
            .with_display_mode(PasswordDisplayMode::Masked)
            .without_confirmation()
            .with_help_message("CTRL + R 显示/隐藏密码")
            .prompt()?;
        let xiaoai = Xiaoai::login(&username, &password).await?;

        let can_save = if cli.auth_file.exists() {
            Confirm::new(&format!("{} 已存在，是否覆盖?", cli.auth_file.display())).prompt()?
        } else {
            true
        };

        if can_save {
            let mut file = File::create(cli.auth_file)?;
            return xiaoai.save(&mut file).map_err(anyhow::Error::from_boxed);
        }
    }

    // 以下命令需要登录
    let xiaoai = cli.xiaoai()?;
    match &cli.command {
        Commands::Login => (),
        Commands::Device => {
            let device_info = xiaoai.device_info().await?;
            for info in device_info {
                println!("{}", DisplayDeviceInfo(info));
            }
        }
        Commands::Say { text } => {
            let device_id = cli.device_id(&xiaoai).await?;
            let response = xiaoai.text_to_speech(&device_id, text).await?;
            println!("{}", response.message);
        }
        Commands::Play { url } => {
            let device_id = cli.device_id(&xiaoai).await?;
            let response = xiaoai.player_play_url(&device_id, url.as_str()).await?;
            println!("{}", response.message);
        }
    }

    Ok(())
}
