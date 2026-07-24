use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::discover;

/// Single-quote a string for safe use in POSIX shell, escaping embedded quotes.
/// Fixes the space/quote bugs of the old `ls`/`column` pipeline.
pub fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn source_line(path: &Path) -> String {
    format!("source {}", shell_quote(&path.to_string_lossy()))
}

/// Emit `source` lines for a list of resolved files.
pub fn source_lines(files: &[PathBuf]) -> String {
    let mut out = String::new();
    for f in files {
        out.push_str(&source_line(f));
        out.push('\n');
    }
    out
}

/// Resolve a list of module path specs (relative to root) into sourceable files,
/// expanding directories recursively and filtering by extension / `.off`.
pub fn resolve_specs(root: &Path, config: &Config, specs: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for spec in specs {
        let target = root.join(spec);
        out.extend(discover::collect_enabled(&target, &config.settings));
    }
    out
}

/// Resolve specs for `source --force`: include every regular file (any
/// extension, including `.off`), expanding directories recursively. Replaces
/// the old `modl -f` / `_modl_force_source`.
pub fn resolve_specs_force(root: &Path, specs: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for spec in specs {
        let target = root.join(spec);
        if target.is_file() {
            out.push(target);
        } else if target.is_dir() {
            for entry in walkdir::WalkDir::new(&target)
                .sort_by_file_name()
                .into_iter()
                .filter_entry(|e| match e.file_name().to_str() {
                    Some(name) => !name.starts_with('.') || e.depth() == 0,
                    None => false,
                })
                .flatten()
            {
                if entry.file_type().is_file() {
                    out.push(entry.path().to_path_buf());
                }
            }
        }
    }
    out
}

/// Emit source lines for the full active environment: the `startup` paths
/// followed by the active profile's modules. Shared by `init bash` and `reload`.
pub fn startup_lines(root: &Path, config: &Config, active_profile: Option<&str>) -> String {
    let mut out = String::new();
    // startup paths (files + dirs), in list order
    out.push_str(&source_lines(&resolve_specs(
        root,
        config,
        &config.settings.startup,
    )));
    // active profile
    if let Some(name) = active_profile {
        if let Ok(specs) = config.profile(name) {
            out.push_str(&source_lines(&resolve_specs(root, config, specs)));
        }
    }
    out
}

/// Emit the `init bash` output: SHMOD_ROOT export, the shim function, and the
/// startup sourcing of the `startup` paths + persisted profile modules.
pub fn init_bash(root: &Path, config: &Config, active_profile: Option<&str>) -> String {
    let mut out = String::new();
    let root_str = root.to_string_lossy();

    out.push_str(&format!("export SHMOD_ROOT={}\n", shell_quote(&root_str)));
    out.push_str(SHIM);
    out.push('\n');
    out.push_str(&startup_lines(root, config, active_profile));

    out
}

/// The shim function `.profile` installs via `eval "$(command shmod init bash)"`.
/// Subcommands that emit shell code are wrapped in `eval`; all others run directly.
const SHIM: &str = r#"shmod() {
  case "$1" in
    init|use|source|reload|reset) eval "$(command shmod "$@")" ;;
    *)                     command shmod "$@" ;;
  esac
}"#;

#[cfg(test)]
mod tests {
    use super::shell_quote;

    #[test]
    fn wraps_plain_string_in_single_quotes() {
        assert_eq!(shell_quote("abc"), "'abc'");
    }

    #[test]
    fn preserves_spaces_inside_quotes() {
        assert_eq!(shell_quote("a b c"), "'a b c'");
    }

    #[test]
    fn escapes_embedded_single_quotes() {
        // The classic POSIX idiom: close, escaped-quote, reopen.
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn neutralizes_shell_metacharacters() {
        // Inside single quotes these are all literal — no expansion in `eval`.
        assert_eq!(shell_quote("$(rm -rf /)"), "'$(rm -rf /)'");
        assert_eq!(shell_quote("$HOME"), "'$HOME'");
        assert_eq!(shell_quote("a;b|c&d"), "'a;b|c&d'");
    }

    #[test]
    fn handles_adjacent_and_multiple_quotes() {
        assert_eq!(shell_quote("''"), r"''\'''\'''");
    }
}
