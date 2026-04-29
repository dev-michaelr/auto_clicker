use super::AppModel;
use anyhow::{Context, Result};
use evdev::KeyCode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

pub const MIN_DURATION: Duration = Duration::from_millis(1);

#[derive(Serialize, Deserialize)]
pub struct Devices {
    pub mouse: PathBuf,
    pub keyboard: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(with = "humantime_serde", default = "default_interval")]
    pub interval: Duration,

    #[serde(default = "default_hotkey")]
    pub hotkey: KeyCode,

    #[serde(default)]
    pub toggle: bool,

    pub devices: Devices,
}

fn default_hotkey() -> KeyCode {
    KeyCode::BTN_EXTRA
}

fn default_interval() -> Duration {
    Duration::from_millis(15)
}

pub fn config_path() -> Result<std::path::PathBuf> {
    let path = std::path::PathBuf::from(
        std::env::var("HOME").context("Couldn't get environment var $HOME")?,
    )
    .join(".config/auto_clicker/config.toml");
    Ok(path)
}

impl AppModel {
    pub fn save_config(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let path = config_path()?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        let contents =
            toml::to_string(&self.config).context("Failed to parse app's stored configuration.")?;
        std::fs::write(path, contents).context("Failed to write to config")?;
        self.dirty = false;
        Ok(())
    }
}
