use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result};

const DEP_MARKER: &str = "# @dep:";

/// Parse `# @dep:` declarations from a file, returning the deduped, sorted set
/// of declared command names. Mirrors the old `_dep_finder`.
pub fn declared(file: &Path) -> Result<BTreeSet<String>> {
    let text =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(DEP_MARKER) {
            for tok in rest.split_whitespace() {
                set.insert(tok.to_string());
            }
        }
    }
    Ok(set)
}

/// Return the subset of `commands` that are NOT found as executables on `PATH`.
/// The Rust equivalent of iterating `command -v` (mirrors `_dep_missing`).
pub fn missing<'a, I>(commands: I) -> Vec<String>
where
    I: IntoIterator<Item = &'a str>,
{
    commands
        .into_iter()
        .filter(|cmd| !on_path(cmd))
        .map(|s| s.to_string())
        .collect()
}

fn on_path(cmd: &str) -> bool {
    // An absolute/relative path with a slash is checked directly.
    if cmd.contains('/') {
        return is_executable(Path::new(cmd));
    }
    let path = match std::env::var_os("PATH") {
        Some(p) => p,
        None => return false,
    };
    std::env::split_paths(&path).any(|dir| is_executable(&dir.join(cmd)))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(m) => m.is_file() && (m.permissions().mode() & 0o111 != 0),
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}
