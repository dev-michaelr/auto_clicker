mod app;
use anyhow::{Context, Result};
use app::save;
use relm4::RelmApp;
use std::fs;

fn main() -> Result<()> {
    let path = save::config_path()?;
    let contents = fs::read_to_string(path).context("Failed to open config")?;
    let config: save::Config = toml::from_str(&contents).context("Failed to parse config")?;
    let app = RelmApp::new("io.github.dev-michaelr.autoclicker");
    app.run::<app::AppModel>(config);
    Ok(())
}
