use crate::config::{self, Config, Manifest};
use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn link(repo: &Path, native_path: &str, stored_path: Option<String>) -> Result<()> {
    let native_path = to_home_path(native_path)?;
    let native_abs = PathBuf::from(shellexpand::tilde(&native_path).into_owned());

    if !native_abs.exists() {
        return Err(anyhow::anyhow!("路径不存在: {}", native_path));
    }

    let entry_name = stored_path.unwrap_or_else(|| {
        let n = native_abs.file_name().unwrap().to_str().unwrap();
        if n.starts_with('.') { n[1..].to_string() } else { n.to_string() }
    });
    let stored_path = repo.join(&entry_name);

    if stored_path.exists() {
        return Err(anyhow::anyhow!("仓库中已存在同名条目: {}", entry_name));
    }

    if let Some(p) = stored_path.parent() { fs::create_dir_all(p)?; }
    fs::rename(&native_abs, &stored_path)?;
    symlink(&stored_path, &native_abs)?;

    let mut manifest = config::load_manifest(repo)?;
    manifest.maps.insert(entry_name.clone(), native_path.clone());
    config::save_manifest(repo, &manifest)?;

    println!("{:>10} {} -> {}", "已链接", entry_name, native_path);
    Ok(())
}

pub fn unlink(repo: &Path, input: &str) -> Result<()> {
    let mut manifest = config::load_manifest(repo)?;
    let entry_name = identify_entry(repo, &manifest, input)?;

    let native_path = manifest.maps.remove(&entry_name).unwrap();
    let native_abs = PathBuf::from(shellexpand::tilde(&native_path).into_owned());
    let stored_path = repo.join(&entry_name);

    if native_abs.is_symlink() { fs::remove_file(&native_abs)?; }
    if let Some(p) = native_abs.parent() { fs::create_dir_all(p)?; }
    fs::rename(stored_path, &native_abs)?;

    config::save_manifest(repo, &manifest)?;
    println!("{:>10} {} (已归位至 {})", "已解除", entry_name, native_path);
    Ok(())
}

pub fn destroy(repo: &Path, input: &str) -> Result<()> {
    let mut manifest = config::load_manifest(repo)?;
    let entry_name = identify_entry(repo, &manifest, input)?;

    let native_path = manifest.maps.remove(&entry_name).unwrap();
    let native_abs = PathBuf::from(shellexpand::tilde(&native_path).into_owned());
    let stored_path = repo.join(&entry_name);

    if native_abs.is_symlink() { fs::remove_file(&native_abs)?; }
    if stored_path.is_dir() { fs::remove_dir_all(&stored_path)?; } else { fs::remove_file(stored_path)?; }

    config::save_manifest(repo, &manifest)?;
    println!("{:>10} {}", "已销毁", entry_name);
    Ok(())
}

pub fn sync(repo: &Path, _force: bool) -> Result<()> {
    let manifest = config::load_manifest(repo)?;
    let mut count = 0;
    for (name, native_path) in &manifest.maps {
        let stored_path = repo.join(name);
        let native_abs = PathBuf::from(shellexpand::tilde(native_path).into_owned());

        if native_abs.is_symlink() && fs::read_link(&native_abs).ok() == Some(stored_path.clone()) {
            continue;
        }

        if native_abs.exists() {
            if native_abs.is_symlink() { fs::remove_file(&native_abs)?; }
            else { continue; }
        }

        if let Some(p) = native_abs.parent() { fs::create_dir_all(p)?; }
        symlink(&stored_path, &native_abs)?;
        count += 1;
        println!("{:>10} {} -> {}", "已同步", name, native_path);
    }
    if count > 0 { println!("{:>10} 处理了 {} 个条目", "完成", count); }
    Ok(())
}

pub fn init(path: &Path, remote: Option<String>, config: &mut Config) -> Result<()> {
    if !path.exists() { fs::create_dir_all(path)?; }
    Command::new("git").arg("init").current_dir(path).status()?;
    if let Some(r) = remote {
        Command::new("git").args(["remote", "add", "origin", &r]).current_dir(path).status()?;
    }
    let default_manifest = "repo_cmd = \"cd {path}\"\n\n[maps]\n";
    fs::write(path.join("manifest.toml"), default_manifest)?;
    config.repository = Some(fs::canonicalize(path)?);
    crate::config::save_config(config)?;
    println!("{:>10} {:?}", "初始化", path);
    Ok(())
}

pub fn clone(remote: &str, locale: Option<PathBuf>, config: &mut Config) -> Result<()> {
    let path = locale.unwrap_or_else(|| PathBuf::from("dotfiles"));
    Command::new("git").args(["clone", remote, path.to_str().unwrap()]).status()?;
    config.repository = Some(fs::canonicalize(&path)?);
    crate::config::save_config(config)?;
    println!("{:>10} {}", "已克隆", remote);
    Ok(())
}

pub fn repo(repo: &Path) -> Result<()> {
    let manifest = config::load_manifest(repo)?;
    let cmd = manifest.repo_cmd.unwrap_or_else(|| "cd {path}".to_string());
    let full_cmd = cmd.replace("{path}", repo.to_str().unwrap());

    println!("{:>10} 执行命令 '{}'", "仓库管理", full_cmd);
    Command::new("sh").arg("-c").arg(&full_cmd).status()?;
    Ok(())
}


/// 将输入路径 (相对/绝对) 转换为 Home 路径 (~/...)
fn to_home_path(input: &str) -> Result<String> {
    let path = PathBuf::from(input);
    let abs_path = if path.is_absolute() { path } else { std::env::current_dir()?.join(path) };

    let final_path = fs::canonicalize(&abs_path).unwrap_or_else(|_| {
        let mut ret = PathBuf::new();
        for component in abs_path.components() {
            match component {
                std::path::Component::CurDir => {},
                std::path::Component::ParentDir => { ret.pop(); },
                c => ret.push(c),
            }
        }
        ret
    });

    let home = directories::BaseDirs::new().context("获取家目录失败")?.home_dir().to_path_buf();
    if final_path.starts_with(&home) {
        Ok(format!("~/{}", final_path.strip_prefix(&home)?.to_str().unwrap()))
    } else {
        Ok(final_path.to_str().unwrap().to_string())
    }
}

/// 通过输入识别 Manifest 中的条目名称
fn identify_entry(repo: &Path, manifest: &Manifest, input: &str) -> Result<String> {
    if manifest.maps.contains_key(input) { return Ok(input.to_string()); }

    let input_path = to_home_path(input)?;
    for (stored_path, native_path) in &manifest.maps {
        if native_path == &input_path { return Ok(stored_path.clone()); }
    }

    let input_abs = if Path::new(input).is_absolute() { PathBuf::from(input) } else { std::env::current_dir()?.join(input) };
    let input_abs = fs::canonicalize(&input_abs).unwrap_or(input_abs);
    if let Ok(rel) = input_abs.strip_prefix(repo) {
        let rel_str = rel.to_str().unwrap_or("");
        if manifest.maps.contains_key(rel_str) { return Ok(rel_str.to_string()); }
    }

    Err(anyhow::anyhow!("未找到目标条目: {}", input_path))
}
