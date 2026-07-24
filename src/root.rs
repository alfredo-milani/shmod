use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::config::CONFIG_FILENAME;

/// Resolve the module tree root, in precedence order:
/// 1. explicit `--root` flag
/// 2. `SHMOD_ROOT` env var (app-specific override)
/// 3. XDG config dir (`$XDG_CONFIG_HOME/shmod`, default `~/.config/shmod`),
///    but only if it contains `shmod.yaml`
/// 4. default `~/.local/shmod`
pub fn resolve(flag: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = flag {
        return canonical(p);
    }
    if let Some(env) = std::env::var_os("SHMOD_ROOT") {
        return canonical(PathBuf::from(env));
    }
    if let Some(dir) = xdg_config_dir()? {
        if dir.join(CONFIG_FILENAME).is_file() {
            return canonical(dir);
        }
    }
    let home = home_dir().context("cannot determine home directory")?;
    canonical(home.join(".local").join("shmod"))
}

fn xdg_config_dir() -> Result<Option<PathBuf>> {
    if let Some(env) = std::env::var_os("XDG_CONFIG_HOME") {
        if !env.is_empty() {
            return Ok(Some(PathBuf::from(env).join("shmod")));
        }
    }
    Ok(home_dir().map(|h| h.join(".config").join("shmod")))
}

fn canonical(p: PathBuf) -> Result<PathBuf> {
    // Fall back to the raw path if canonicalization fails (e.g. not yet created),
    // so callers still get a usable absolute-ish path for error messages.
    Ok(std::fs::canonicalize(&p).unwrap_or(p))
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
