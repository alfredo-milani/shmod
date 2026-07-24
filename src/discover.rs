use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::config::Settings;

/// Whether a filename is hidden (leading dot).
fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

/// Final extension of a path, lowercased.
fn extension_of(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
}

/// A module file with a flag for whether it is disabled (`.off`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub path: PathBuf,
    pub disabled: bool,
}

/// Recursively collect module files under `target`, skipping hidden entries.
///
/// `enabled` files have an extension in `settings.extensions`; `disabled` files
/// end in `settings.off_extension`. All other files are ignored. Results are
/// sorted for stable output. This is the non-`eval` replacement for the old
/// `recevalf` + `_modl_source` traversal.
pub fn collect(target: &Path, settings: &Settings) -> Vec<Module> {
    let mut out = Vec::new();
    if !target.exists() {
        return out;
    }
    for entry in WalkDir::new(target)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| match e.file_name().to_str() {
            Some(name) => !is_hidden(name) || e.depth() == 0,
            None => false,
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = match extension_of(path) {
            Some(e) => e,
            None => continue,
        };
        if ext == settings.off_extension.to_ascii_lowercase() {
            out.push(Module {
                path: path.to_path_buf(),
                disabled: true,
            });
        } else if settings
            .extensions
            .iter()
            .any(|e| e.to_ascii_lowercase() == ext)
        {
            out.push(Module {
                path: path.to_path_buf(),
                disabled: false,
            });
        }
    }
    out
}

/// Collect only sourceable (enabled) module files under `target`.
pub fn collect_enabled(target: &Path, settings: &Settings) -> Vec<PathBuf> {
    collect(target, settings)
        .into_iter()
        .filter(|m| !m.disabled)
        .map(|m| m.path)
        .collect()
}
