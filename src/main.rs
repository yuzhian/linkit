mod config;
mod handler;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "linkit", version = "0.1.0", about = "Dotfiles Manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 建立链接 :: 将文件移入仓库并建立链接 [l]
    #[command(alias = "l")]
    Link {
        /// 要链接的原始路径 (如 . 或 ~/.zshrc)
        #[arg(value_name = "原始路径")]
        native_path: String,
        /// 在仓库中存储的自定义名称 (可选)
        #[arg(value_name = "仓库路径")]
        stored_path: Option<String>
    },

    /// 解除链接 :: 解除链接并将文件移回原位 [u/un]
    #[command(alias = "un", alias = "u")]
    Unlink {
        /// 条目名称、逻辑路径或仓库内路径
        #[arg(value_name = "标识")]
        input: String
    },

    /// 彻底销毁 :: 物理删除仓库原件及链接 [d]
    #[command(alias = "d")]
    Destroy {
        /// 条目名称、逻辑路径或仓库内路径
        #[arg(value_name = "标识")]
        input: String
    },

    /// 同步清单 :: 同步所有清单条目 [s]
    #[command(alias = "s")]
    Sync {
        /// 是否强制覆盖已存在的非链接文件
        #[arg(short, long)]
        force: bool
    },

    /// 创建仓库 :: 创建仓库并注册仓库路径
    Init {
        /// 远程 Git 仓库地址 (可选)
        #[arg(value_name = "远程地址")]
        remote: Option<String>,
        /// 本地仓库创建位置 (默认为当前目录)
        #[arg(value_name = "位置")]
        locale: Option<PathBuf> 
    },

    /// 克隆仓库 :: 克隆仓库并注册仓库路径
    Clone {
        /// 远程 Git 仓库地址
        #[arg(value_name = "远程地址")]
        remote: String,
        /// 本地克隆位置 (默认为 ./dotfiles)
        #[arg(value_name = "位置")]
        locale: Option<PathBuf>
    },

    /// 关联仓库 :: 关联一个已存在的仓库
    Locate {
        /// 本地仓库位置
        #[arg(value_name = "位置")]
        locale: PathBuf
    },

    /// 管理仓库 :: 执行预定义指令 [rp]
    #[command(alias = "rp")]
    Repo,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = config::load_config()?;

    if let Commands::Init { remote, locale } = cli.command {
        let path = locale.unwrap_or(std::env::current_dir()?);
        return handler::init(&path, remote, &mut config);
    }
    if let Commands::Clone { remote, locale } = cli.command {
        return handler::clone(&remote, locale, &mut config);
    }
    if let Commands::Locate { locale } = cli.command {
        return handler::locate(&locale, &mut config);
    }

    let repo = config.repository.as_ref()
        .context("未关联仓库。请先执行 'linkit init' 或 'linkit clone'。")?;

    match cli.command {
        Commands::Link { native_path, stored_path } => handler::link(repo, &native_path, stored_path),
        Commands::Unlink { input } => handler::unlink(repo, &input),
        Commands::Destroy { input } => handler::destroy(repo, &input),
        Commands::Sync { force } => handler::sync(repo, force),
        Commands::Repo => handler::repo(repo),
        _ => unreachable!(),
    }
}
