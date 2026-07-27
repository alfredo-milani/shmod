use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Resolve the module tree root, in precedence order:
/// 1. explicit `--root` flag
/// 2. `SHMOD_ROOT` env var (app-specific override)
/// 3. XDG config dir (`$XDG_CONFIG_HOME/shmod`, default `~/.config/shmod`)
pub fn resolve(flag: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = flag {
        return canonical(p);
    }
    if let Some(env) = std::env::var_os("SHMOD_ROOT") {
        return canonical(PathBuf::from(env));
    }
    xdg_config_dir()?
        .context("cannot determine XDG config directory")
        .and_then(canonical)
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

/// Expand a config-supplied path: `~` or `~/...` resolves against `$HOME`,
/// an absolute path is used as-is, and anything else is joined against
/// `base` (the config root). Used for `settings.modules_root`.
pub fn expand_path(raw: &str, base: &Path) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(rest);
        }
    } else if raw == "~" {
        if let Some(home) = home_dir() {
            return home;
        }
    }
    let p = PathBuf::from(raw);
    if p.is_absolute() {
        p
    } else {
        base.join(p)
    }
}
