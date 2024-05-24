use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const CONFY_APP_NAME: &str = "dmoj-submit";
pub const CONFY_CONFIG_NAME: &str = "config";

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ConfyConfig {
    /// API token
    pub token: Option<String>,
    /// File extension -> language key mapping
    pub ext_key_map: Option<HashMap<String, String>>,
}

pub fn get_config_path() -> Result<std::path::PathBuf> {
    confy::get_configuration_file_path(CONFY_APP_NAME, CONFY_CONFIG_NAME)
        .with_context(|| "could not get the configuration file path")
}

pub fn get_config() -> Result<ConfyConfig> {
    confy::load(CONFY_APP_NAME, CONFY_CONFIG_NAME).with_context(|| "could not load configuration")
}

pub fn set_config(cfg: ConfyConfig) -> Result<()> {
    confy::store(CONFY_APP_NAME, CONFY_CONFIG_NAME, cfg)
        .with_context(|| "could not store configuration")
}
