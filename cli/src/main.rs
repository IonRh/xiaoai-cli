use std::{fs::File, io::BufReader, path::PathBuf};

use clap::{Parser, Subcommand};
use inquire::{Confirm, Password, PasswordDisplayMode, Text};
use miai::Xiaoai;

const DEFAULT_AUTH_FILE: &str = "xiaoai-auth.json";

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 登录小爱服务以获得认证
    Login {
        /// 另存为认证文件
        #[arg(short, long, default_value = DEFAULT_AUTH_FILE)]
        save: PathBuf,
    },
    /// 列出小爱设备
    Device {
        /// 加载认证文件
        #[arg(long, default_value = DEFAULT_AUTH_FILE)]
        auth: PathBuf,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Login { save } => {
            let username = Text::new("账号:").prompt()?;
            let password = Password::new("密码:")
                .with_display_toggle_enabled()
                .with_display_mode(PasswordDisplayMode::Masked)
                .without_confirmation()
                .with_help_message("CTRL + R 显示/隐藏密码")
                .prompt()?;
            let xiaoai = Xiaoai::login(&username, &password).await?;

            let can_save = if save.exists() {
                Confirm::new(&format!("{} 已存在，是否覆盖?", save.display())).prompt()?
            } else {
                true
            };

            if can_save {
                let mut file = File::create(save)?;
                xiaoai.save(&mut file).map_err(anyhow::Error::from_boxed)?;
            }
        }
        Commands::Device { auth } => {
            let file = File::open(auth)?;
            let xiaoai = Xiaoai::load(BufReader::new(file)).map_err(anyhow::Error::from_boxed)?;
            let devices = xiaoai.device_info().await?;
            for device in devices {
                println!("名称: {}", device.name);
                println!("设备 ID: {}", device.device_id);
                println!("机型: {}", device.hardware);
            }
        }
    }

    Ok(())
}
