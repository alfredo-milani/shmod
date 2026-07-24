use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::root::home_dir;

/// Path to the persisted active-profile state file:
/// `${XDG_STATE_HOME:-$HOME/.local/state}/shmod/active-profile`.
pub fn state_file() -> Result<PathBuf> {
    let base = match std::env::var_os("XDG_STATE_HOME") {
        Some(x) if !x.is_empty() => PathBuf::from(x),
        _ => home_dir()
            .context("cannot determine home directory")?
            .join(".local")
            .join("state"),
    };
    Ok(base.join("shmod").join("active-profile"))
}

/// Read the persisted default profile name, if any.
pub fn read() -> Result<Option<String>> {
    let path = state_file()?;
    match std::fs::read_to_string(&path) {
        Ok(s) => {
            let name = s.trim();
            Ok(if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("reading state {}", path.display())),
    }
}

/// Persist `name` as the default profile.
pub fn write(name: &str) -> Result<()> {
    let path = state_file()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating state dir {}", parent.display()))?;
    }
    std::fs::write(&path, format!("{name}\n"))
        .with_context(|| format!("writing state {}", path.display()))?;
    Ok(())
}

/// Clear the persisted default profile.
pub fn clear() -> Result<()> {
    let path = state_file()?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("removing state {}", path.display())),
    }
}
