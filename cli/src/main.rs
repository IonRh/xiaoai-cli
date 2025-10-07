use std::{fs::File, path::PathBuf};

use clap::{Parser, Subcommand};
use inquire::{Confirm, Password, PasswordDisplayMode, Text};
use miai::Xiaoai;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 登录小爱服务以获得认证
    Login {
        /// 另存为认证文件
        #[arg(short, long, default_value = "xiaoai-auth.json")]
        save: PathBuf,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
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
        }
    }

    Ok(())
}
