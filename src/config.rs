use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use directories::ProjectDirs;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub repository: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Manifest {
    pub open_cmd: Option<String>,
    pub maps: BTreeMap<String, String>,
}

pub fn get_config_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("", "", "linkit").context("获取配置目录失败")?;
    let config_dir = proj_dirs.config_dir();
    if !config_dir.exists() { fs::create_dir_all(config_dir)?; }
    Ok(config_dir.join("config.toml"))
}

pub fn load_config() -> Result<Config> {
    let path = get_config_path()?;
    if !path.exists() { return Ok(Config::default()); }
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = get_config_path()?;
    fs::write(path, toml::to_string_pretty(config)?)?;
    Ok(())
}

pub fn load_manifest(repo_path: &Path) -> Result<Manifest> {
    let path = repo_path.join("manifest.toml");
    if !path.exists() { return Ok(Manifest::default()); }
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

pub fn save_manifest(repo_path: &Path, manifest: &Manifest) -> Result<()> {
    let path = repo_path.join("manifest.toml");
    fs::write(path, toml::to_string_pretty(manifest)?)?;
    Ok(())
}
