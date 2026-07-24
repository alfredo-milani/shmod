pub mod cli;
pub mod config;
pub mod deps;
pub mod discover;
pub mod emit;
pub mod root;
pub mod state;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use cli::{Cli, Command, ListMode};
use config::Config;

pub fn run(args: Cli) -> Result<()> {
    let root = root::resolve(args.root.clone())?;

    match args.command {
        Command::Init { .. } => {
            let config = Config::load(&root)?;
            // Persisted user default wins; otherwise fall back to the
            // committed default profile from shmod.yaml.
            let active = state::read()?.or_else(|| config.default.clone());
            print!("{}", emit::init_bash(&root, &config, active.as_deref()));
        }

        Command::Use { profile, save } => {
            let config = Config::load(&root)?;
            let specs = config.profile(&profile)?.to_vec();
            let files = emit::resolve_specs(&root, &config, &specs);
            print!("{}", emit::source_lines(&files));
            if save {
                state::write(&profile)?;
                println!("echo 'shmod: default profile set to {profile}'");
            }
        }

        Command::Source { paths, force } => {
            let config = Config::load(&root)?;
            let specs = cli::split_paths(&paths);
            let files = if force {
                emit::resolve_specs_force(&root, &specs)
            } else {
                emit::resolve_specs(&root, &config, &specs)
            };
            print!("{}", emit::source_lines(&files));
        }

        Command::Reload => {
            let config = Config::load(&root)?;
            // Same resolution as init: persisted user default wins, else committed default.
            let active = state::read()?.or_else(|| config.default.clone());
            print!("{}", emit::startup_lines(&root, &config, active.as_deref()));
        }

        Command::Reset { save } => {
            if save {
                state::clear()?;
                println!("echo 'shmod: default profile cleared'");
            } else {
                println!("echo 'shmod: reset only affects new shells; open a new shell'");
            }
        }

        Command::Profiles => {
            let config = Config::load(&root)?;
            profiles(&config)?;
        }

        Command::Active => {
            let config = Config::load(&root)?;
            match state::read()? {
                Some(name) => println!("{name}"),
                None => match config.default.as_deref() {
                    Some(name) => println!("{name} (default)"),
                    None => println!("(none)"),
                },
            }
        }

        Command::List { mode } => list(&root, mode)?,

        Command::Check { paths } => check(&root, &paths)?,
    }

    Ok(())
}

/// Render defined profiles, one block per profile, with each profile's module
/// specs listed as a tree. The resolved active profile (persisted default wins
/// over the committed `default:`) is tagged so users can see it at a glance.
fn profiles(config: &Config) -> Result<()> {
    if config.profiles.is_empty() {
        println!("(no profiles defined)");
        return Ok(());
    }

    let persisted = state::read()?;
    // Persisted default wins over the committed `default:`; whichever resolves
    // is the profile new shells load.
    let active = persisted.as_deref().or(config.default.as_deref());

    let last = config.profiles.len().saturating_sub(1);
    for (i, (name, specs)) in config.profiles.iter().enumerate() {
        let n = name.as_str();
        let tag = if active == Some(n) {
            "  ● active"
        } else if config.default.as_deref() == Some(n) {
            // Committed default that a persisted default has overridden.
            "  ○ default"
        } else {
            ""
        };
        println!("{name}{tag}");
        let spec_last = specs.len().saturating_sub(1);
        for (j, spec) in specs.iter().enumerate() {
            let connector = if j == spec_last {
                "└── "
            } else {
                "├── "
            };
            println!("{connector}{spec}");
        }
        if i != last {
            println!();
        }
    }
    Ok(())
}

fn list(root: &Path, mode: ListMode) -> Result<()> {
    let config = Config::load(root)?;
    let startup: Vec<PathBuf> = config
        .settings
        .startup
        .iter()
        .map(|c| root.join(c))
        .collect();
    let mut tree = TreeNode::default();
    for m in discover::collect(root, &config.settings) {
        // Skip startup paths — they always load and aren't togglable modules.
        if startup
            .iter()
            .any(|c| m.path == *c || m.path.starts_with(c))
        {
            continue;
        }
        let show = match mode {
            ListMode::All => true,
            ListMode::Enabled => !m.disabled,
            ListMode::Disabled => m.disabled,
        };
        if !show {
            continue;
        }
        let rel = m.path.strip_prefix(root).unwrap_or(&m.path);
        let components: Vec<String> = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        tree.insert(&components, m.disabled);
    }
    // Collapsing fully-disabled directories only makes sense for the full view;
    // `--mode disabled` deliberately wants those files shown.
    tree.render(mode == ListMode::All);
    Ok(())
}

/// A node in the module listing tree. Directories hold `children`; leaf module
/// files carry a `disabled` flag (rendered as `on`/`off`).
#[derive(Default)]
struct TreeNode {
    children: BTreeMap<String, TreeNode>,
    disabled: Option<bool>,
}

impl TreeNode {
    fn insert(&mut self, components: &[String], disabled: bool) {
        match components {
            [] => {}
            [leaf] => {
                self.children.entry(leaf.clone()).or_default().disabled = Some(disabled);
            }
            [head, rest @ ..] => {
                self.children
                    .entry(head.clone())
                    .or_default()
                    .insert(rest, disabled);
            }
        }
    }

    fn render(&self, collapse_disabled: bool) {
        self.render_children("", collapse_disabled);
    }

    /// True when every module file in this subtree is disabled. A leaf reports
    /// its own state; a directory folds over its children.
    fn all_disabled(&self) -> bool {
        match self.disabled {
            Some(state) => state,
            None => self.children.values().all(TreeNode::all_disabled),
        }
    }

    fn render_children(&self, prefix: &str, collapse_disabled: bool) {
        let last = self.children.len().saturating_sub(1);
        for (i, (name, node)) in self.children.iter().enumerate() {
            let is_last = i == last;
            let connector = if is_last { "└── " } else { "├── " };
            let is_dir = node.disabled.is_none();
            // Collapse a directory whose whole subtree is disabled: mark it
            // `[off]` and don't expand its children.
            let collapse = collapse_disabled && is_dir && node.all_disabled();
            let label = match node.disabled {
                Some(true) => format!("{name}  [off]"),
                Some(false) => format!("{name}  [on]"),
                None if collapse => format!("{name}  [off]"),
                None => name.clone(),
            };
            println!("{prefix}{connector}{label}");
            if collapse {
                continue;
            }
            let child_prefix = format!("{prefix}{}", if is_last { "    " } else { "│   " });
            node.render_children(&child_prefix, collapse_disabled);
        }
    }
}

fn check(root: &Path, paths: &[String]) -> Result<()> {
    let config = Config::load(root)?;
    let specs = cli::split_paths(paths);
    let mut any_missing = false;
    for spec in specs {
        let target = root.join(&spec);
        // Check both enabled and disabled files (mirror old behavior).
        for m in discover::collect(&target, &config.settings) {
            let declared = deps::declared(&m.path)?;
            let declared_refs: Vec<&str> = declared.iter().map(String::as_str).collect();
            let missing = deps::missing(declared_refs);
            if !missing.is_empty() {
                any_missing = true;
                let rel = m.path.strip_prefix(root).unwrap_or(&m.path);
                println!("{} requires: {}", rel.to_string_lossy(), missing.join(" "));
            }
        }
    }
    if !any_missing {
        println!("all dependencies satisfied");
    }
    Ok(())
}
