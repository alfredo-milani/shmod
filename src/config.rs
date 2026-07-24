use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

pub const CONFIG_FILENAME: &str = "shmod.yaml";

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub settings: Settings,
    /// Committed default profile, sourced by new shells unless a user has
    /// persisted their own default via `use --save`.
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub profiles: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,
    #[serde(default = "default_off_extension")]
    pub off_extension: String,
    /// Ordered list of paths (files or dirs) sourced at startup, before any
    /// profile. Order is list position; dirs expand via discovery.
    #[serde(default = "default_startup")]
    pub startup: Vec<String>,
}

fn default_extensions() -> Vec<String> {
    vec!["sh".to_string()]
}

fn default_off_extension() -> String {
    "off".to_string()
}

fn default_startup() -> Vec<String> {
    vec![
        ".core/environment.sh".to_string(),
        ".core/alias.sh".to_string(),
        ".core/function.sh".to_string(),
    ]
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            extensions: default_extensions(),
            off_extension: default_off_extension(),
            startup: default_startup(),
        }
    }
}

impl Config {
    /// Load configuration from `<root>/shmod.yaml`. A missing file yields defaults
    /// with no profiles, so a bare module tree still works.
    pub fn load(root: &Path) -> Result<Config> {
        let path = root.join(CONFIG_FILENAME);
        if !path.exists() {
            return Ok(Config {
                settings: Settings::default(),
                default: None,
                profiles: BTreeMap::new(),
            });
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading config {}", path.display()))?;
        let config: Config = serde_yaml_ng::from_str(&text)
            .with_context(|| format!("parsing config {}", path.display()))?;
        Ok(config)
    }

    pub fn profile(&self, name: &str) -> Result<&[String]> {
        self.profiles
            .get(name)
            .map(|v| v.as_slice())
            .with_context(|| format!("unknown profile \"{name}\""))
    }
}
