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
        system_path: String,
        /// 在仓库中存储的自定义名称 (可选)
        #[arg(value_name = "仓库路径")]
        stored_key: Option<String>
    },

    /// 解除链接 :: 解除链接并将文件移回原位 [u/un]
    #[command(alias = "un", alias = "u")]
    Unlink {
        /// 条目名称、逻辑路径或仓库内路径
        #[arg(value_name = "标识")]
        input: String
    },

    /// 彻底销毁 :: 物理删除仓库原件及链接 [rm]
    #[command(alias = "rm")]
    Remove {
        /// 条目名称、逻辑路径或仓库内路径
        #[arg(value_name = "标识")]
        input: String
    },

    /// 同步清单 :: 同步所有清单条目 [d]
    #[command(alias = "d")]
    Deploy {
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

    /// 打开仓库 :: 执行预定义指令
    Open,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let mut config = config::load_config()?;

    match cli.command {
        Commands::Init { remote, locale } => {
            let path = locale.unwrap_or(std::env::current_dir()?);
            handler::init(&path, remote, &mut config)
        }
        Commands::Clone { remote, locale } => handler::clone(&remote, locale, &mut config),
        Commands::Locate { locale } => handler::locate(&locale, &mut config),

        cmd => {
            let repo = config.repository.as_ref().context("No repository found. Please run 'linkit init' or 'linkit clone' or 'linkit locate' first.")?;

            match cmd {
                Commands::Link { system_path, stored_key } => handler::link(repo, &system_path, stored_key),
                Commands::Unlink { input } => handler::unlink(repo, &input),
                Commands::Remove { input } => handler::remove(repo, &input),
                Commands::Deploy { force } => handler::deploy(repo, force),
                Commands::Open => handler::open(repo),
                _ => unreachable!(),
            }
        }
    }
}
