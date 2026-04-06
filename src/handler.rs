use crate::config::{self, Config, Manifest};
use anyhow::{Context, Result};
use std::fs;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::process::Command;
use console::{style};


// ============ 操作命令::配置文件 ====================================

pub fn link(repo: &Path, system_path: &str, stored_key: Option<String>) -> Result<()> {
    let system_path = normalize_path(system_path)?;
    let system_abs = PathBuf::from(shellexpand::tilde(&system_path).into_owned());
    if !system_abs.exists() {
        return Err(anyhow::anyhow!("{}", format_message("path does not exist", style("[x]").red(), &[&style(&system_path).strikethrough()])));
    }

    let stored_key = stored_key.unwrap_or_else(|| {
        let name = system_abs.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
        name.strip_prefix('.').unwrap_or(&name).to_string()
    });
    let stored_path = repo.join(&stored_key);
    if stored_path.exists() {
        return Err(anyhow::anyhow!("{}", format_message("entry already exists", style("[x]").red(), &[&stored_key, &"<--", &system_path])));
    }

    if let Some(p) = stored_path.parent() { fs::create_dir_all(p)?; }
    move_item(&system_abs, &stored_path).context(format_message("move failed", style("[x]").red(), &[&stored_key, &"<--", &system_path]))?;
    symlink::symlink_auto(&stored_path, &system_abs).context(format_message("symlink failed", style("[x]").red(), &[&stored_key, &"<--", &system_path]))?;

    let mut manifest = config::load_manifest(repo)?;
    manifest.maps.insert(stored_key.clone(), system_path.clone());
    config::save_manifest(repo, &manifest)?;

    println!("{}", format_message("success", style("[√]").green(), &[&stored_key, &"<--", &system_path]));
    Ok(())
}

pub fn unlink(repo: &Path, input: &str) -> Result<()> {
    let mut manifest = config::load_manifest(repo)?;
    let stored_key = identify_entry(repo, &manifest, input)?;

    let system_path = manifest.maps.remove(&stored_key).context(format_message("entry not found", style("[x]").red(), &[&style(&stored_key).strikethrough()]))?;
    let system_abs = PathBuf::from(shellexpand::tilde(&system_path).into_owned());
    let stored_path = repo.join(&stored_key);

    if system_abs.exists() || system_abs.is_symlink() {
        let _ = symlink::remove_symlink_auto(&system_abs)
            .or_else(|_| fs::remove_dir_all(&system_abs))
            .or_else(|_| fs::remove_file(&system_abs));
    }

    if let Some(p) = system_abs.parent() { fs::create_dir_all(p)?; }
    move_item(&stored_path, &system_abs).context(format_message("file move failed", style("[x]").red(), &[&style(&stored_key).strikethrough(), &"-->", &system_path]))?;

    config::save_manifest(repo, &manifest)?;
    println!("{}", format_message("success", style("[√]").green(), &[&style(&stored_key).strikethrough(), &"-->", &system_path]));
    Ok(())
}

pub fn remove(repo: &Path, input: &str) -> Result<()> {
    let mut manifest = config::load_manifest(repo)?;
    let stored_key = identify_entry(repo, &manifest, input)?;

    let system_path = manifest.maps.remove(&stored_key).context(format_message("entry not found", style("[x]").red(), &[&style(&stored_key).strikethrough()]))?;
    let system_abs = PathBuf::from(shellexpand::tilde(&system_path).into_owned());
    let stored_path = repo.join(&stored_key);
        if system_abs.exists() || system_abs.is_symlink() {
        let _ = symlink::remove_symlink_auto(&system_abs)
            .or_else(|_| fs::remove_dir_all(&system_abs))
            .or_else(|_| fs::remove_file(&system_abs));
    }
    if stored_path.is_dir() { fs::remove_dir_all(&stored_path)?; } else { fs::remove_file(&stored_path)?; }

    config::save_manifest(repo, &manifest)?;
    println!("{}", format_message("success", style("[√]").green(), &[&style(&stored_key).strikethrough(), &"<x>", &style(&system_path).strikethrough()]));
    Ok(())
}

pub fn deploy(repo: &Path, force: bool) -> Result<()> {
    let manifest = config::load_manifest(repo)?;
    for (stored_key, system_path) in &manifest.maps {
        let stored_path = repo.join(stored_key);
        let system_abs = PathBuf::from(shellexpand::tilde(system_path).into_owned());

        let stored_abs = dunce::canonicalize(&stored_path).unwrap_or(stored_path.clone());
        let system_link_abs = fs::read_link(&system_abs).ok().map(|p| dunce::canonicalize(&p).unwrap_or(p));
        if system_link_abs.as_ref() == Some(&stored_abs) {
            println!("{}", format_message("skipped", style("[~]").green(), &[&stored_key, &">>>", &system_path]));
            continue;
        }

        if system_abs.exists() || system_abs.is_symlink() {
            if !force {
                println!("{}", format_message("conflict", style("[x]").red(), &[&stored_key, &">>>", &system_path]));
                continue;
            }
            let _ = symlink::remove_symlink_auto(&system_abs)
                .or_else(|_| fs::remove_dir_all(&system_abs))
                .or_else(|_| fs::remove_file(&system_abs));
        }

        if let Some(p) = system_abs.parent() { fs::create_dir_all(p)?; }
        symlink::symlink_auto(&stored_path, &system_abs).context("同步链接失败")?;
        println!("{}", format_message("success", style("[√]").green(), &[&stored_key, &">>>", &system_path]));
    }
    Ok(())
}


// ============ 操作命令::仓库 ====================================

pub fn init(path: &Path, remote: Option<String>, config: &mut Config) -> Result<()> {
    if !path.exists() { fs::create_dir_all(path)?; }
    Command::new("git").arg("init").current_dir(path).status()?;
    if let Some(r) = remote {
        Command::new("git").args(["remote", "add", "origin", &r]).current_dir(path).status()?;
    }
    let default_manifest = "open_cmd = \"nvim {path}\"\n\n[maps]\n";
    fs::write(path.join("manifest.toml"), default_manifest)?;
    config.repository = Some(dunce::canonicalize(path)?);
    crate::config::save_config(config)?;
    println!("{}", format_message("initialized repository", style("[√]").green(), &[&path.to_str().unwrap()]));
    Ok(())
}

pub fn clone(remote: &str, locale: Option<PathBuf>, config: &mut Config) -> Result<()> {
    let path = locale.unwrap_or_else(|| PathBuf::from("dotfiles"));
    Command::new("git").args(["clone", remote, path.to_str().unwrap()]).status()?;
    config.repository = Some(dunce::canonicalize(&path)?);
    crate::config::save_config(config)?;
    println!("{}", format_message("cloned repository", style("[√]").green(), &[&remote]));
    Ok(())
}

pub fn locate(locale: &Path, config: &mut Config) -> Result<()> {
    if !locale.exists() {
        return Err(anyhow::anyhow!("{}", format_message("specified path does not exist", style("[x]").red(), &[&locale.to_str().unwrap()])));
    }
    config.repository = Some(dunce::canonicalize(locale)?);
    crate::config::save_config(config)?;
    println!("{}", format_message("located repository", style("[√]").green(), &[&locale.to_str().unwrap()]));
    Ok(())
}

pub fn open(repo: &Path) -> Result<()> {
    let manifest = config::load_manifest(repo)?;
    let cmd = manifest.open_cmd.as_ref().ok_or_else(|| anyhow::anyhow!("'open_cmd' is not set in manifest.toml"))?;
    let full_cmd = cmd.replace("{path}", repo.to_str().unwrap());

    if cfg!(windows) {
        Command::new("cmd").arg("/C").arg(&full_cmd).status()?;
    } else {
        Command::new("sh").arg("-c").arg(&full_cmd).status()?;
    }
    Ok(())
}


// ============ 工具函数 ====================================

fn normalize_path(input: &str) -> Result<String> {
    let path = PathBuf::from(input);
    let abs_path = if path.is_absolute() { path } else { std::env::current_dir()?.join(path) };

    let final_path = dunce::canonicalize(&abs_path).unwrap_or(abs_path);
    let home = directories::BaseDirs::new().context(format_message("failed to get home directory", style("[x]").red(), &[]))?.home_dir().to_path_buf();
    if final_path.starts_with(&home) {
        Ok(format!("~/{}", final_path.strip_prefix(&home)?.to_str().unwrap().replace('\\', "/")))
    } else {
        Ok(final_path.to_str().unwrap().replace('\\', "/"))
    }
}

fn identify_entry(repo: &Path, manifest: &Manifest, input: &str) -> Result<String> {
    if manifest.maps.contains_key(input) { return Ok(input.to_string()); }

    let input_path = normalize_path(input)?;
    for (stored_path, system_path) in &manifest.maps {
        if system_path == &input_path { return Ok(stored_path.clone()); }
    }

    let input_abs = if Path::new(input).is_absolute() { PathBuf::from(input) } else { std::env::current_dir()?.join(input) };
    let input_abs = fs::canonicalize(&input_abs).unwrap_or(input_abs);
    if let Ok(rel) = input_abs.strip_prefix(repo) {
        let rel_str = rel.to_str().unwrap_or("");
        if manifest.maps.contains_key(rel_str) { return Ok(rel_str.to_string()); }
    }

    Err(anyhow::anyhow!("{}", format_message("entry not found", style("[x]").red(), &[&input])))
}

fn move_item(from: &Path, to: &Path) -> Result<()> {
    if from.is_dir() {
        let mut options = fs_extra::dir::CopyOptions::new();
        options.copy_inside = true;
        options.overwrite = true;
        fs_extra::dir::move_dir(from, to, &options)?;
    } else {
        let mut options = fs_extra::file::CopyOptions::new();
        options.overwrite = true;
        fs_extra::file::move_file(from, to, &options)?;
    }
    Ok(())
}

fn format_message(
    note: &str,
    status: impl Display,
    args: &[&dyn Display], 
) -> String {
    let mut middle = String::new();
    for arg in args {
        middle.push_str(&format!("{} ", arg));
    }

    format!("{:>10} {} ({})", status, middle.trim(), style(note).dim().italic())
}
