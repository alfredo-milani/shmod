# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`shmod` is a context-selective bash configuration loader written in Rust. It organizes shell env vars, aliases, and functions into a tree of `.sh` modules and sources them on demand: a `startup` set every shell gets, plus named **profiles** switched between at runtime.

Because a process cannot mutate its parent shell, the binary **never sources anything itself** — it prints `source '<path>'` lines to stdout, and a bash shim installed via `eval "$(command shmod init bash)"` evals them. This is the direnv/mise hybrid model. Keep this invariant in mind: any command meant to change the live shell must emit shell code, not perform the effect.

## Commands

```bash
cargo build --release        # binary at target/release/shmod
cargo test                   # unit tests (in-module) + integration tests (tests/cli.rs)
cargo test --test cli        # integration tests only
cargo test <name>            # single test by name substring, e.g. cargo test reload_includes_active_profile
cargo clippy --all-targets   # lint
cargo fmt                    # format
```

Integration tests build the binary and invoke it as a subprocess against a temp fixture tree (`tests/cli.rs`), driving root via `--root` and state via `XDG_STATE_HOME` so they never touch real config.

## Architecture

Thin `main.rs` → `lib.rs::run()` dispatches on the parsed `Command`. Each module owns one concern:

- **`cli.rs`** — clap `Cli`/`Command` definitions. `split_paths` handles the legacy `a:b:c` colon-separated multi-path syntax.
- **`root.rs`** — resolves the module tree root in precedence order: `--root` flag → `SHMOD_ROOT` env → XDG config dir (`$XDG_CONFIG_HOME/shmod`, default `~/.config/shmod`). Also holds `expand_path`, used only for `settings.modules_root`: `~`/`~/...` expands against `$HOME`, an absolute path passes through, anything else joins against the config root.
- **`config.rs`** — deserializes `shmod.yaml` (`serde_yaml_ng`). A missing file yields defaults with no profiles, so a bare tree still works. Holds `settings` (extensions, off_extension, startup list, optional `modules_root`) and `profiles` (name → list of paths). `Config::modules_root(root)` resolves where module paths actually live: `settings.modules_root` (expanded via `root::expand_path` for `~`/absolute/relative-to-root) if set, else `root` itself.
- **`discover.rs`** — recursive module-file collection via `walkdir`. Skips hidden entries (except the target root itself), classifies each file as enabled (extension in `settings.extensions`) or disabled (ends in `off_extension`, default `.off`). Sorted for stable output.
- **`emit.rs`** — turns resolved paths into `source` lines and builds the `init bash` output (SHMOD_ROOT export + shim + startup sourcing). **`shell_quote` is security-critical**: it single-quotes every emitted path so metacharacters can't execute when the shim evals the output. The `SHIM` const lists exactly which subcommands get wrapped in `eval` (`init|use|source|reload|reset`) vs. run directly.
- **`state.rs`** — reads/writes the persisted active profile at `${XDG_STATE_HOME:-$HOME/.local/state}/shmod/active-profile`. This is program-managed **state**, deliberately kept separate from user-authored **config** (`shmod.yaml`).
- **`deps.rs`** — parses `# @dep: cmd` markers from module files and reports which declared commands are missing from `PATH` (used by `shmod check`).

### Key behaviors to preserve

- **Profile resolution precedence**: persisted default (`use --save`) wins over the committed `default:` in `shmod.yaml`. `init` and `reload` share this via `state::read()?.or_else(|| config.default.clone())`. `init --profile <name>` overrides both for that shell only (via `profile.or(state::read()?).or_else(...)`) and does not persist. `reset --save` clears the persisted default, falling back to the committed one.
- **Startup vs. profile**: `startup` paths load in every shell in list order; profiles load on demand. `list` deliberately excludes startup paths (they aren't togglable modules).
- **`list` tree rendering** (`lib.rs::TreeNode`): modules render as a `├──`/`└──` tree, each file tagged `[on]`/`[off]`. In the default `--mode all` view, a directory whose entire subtree is disabled collapses to a single `[off]` node instead of expanding — this collapse is intentionally skipped for `--mode disabled`, which wants every disabled file listed.
- **`profiles` rendering** (`lib.rs::profiles`): each profile prints a header line then its module specs as a `├──`/`└──` tree. The resolved profile (same precedence as above) is tagged `● active`; a committed `default:` that a persisted default has overridden is tagged `○ default`.
- **Directories vs. files**: every "path spec" (in `startup` or a profile) may be a file or a dir; dirs expand recursively via discovery.
- **`--force`** (`source --force`) bypasses extension and `.off` filtering to source any regular file.
- **`modules_root` vs. the config root**: `SHMOD_ROOT` (exported by `init bash`) always reflects the *config* root (where `shmod.yaml` is resolved from); `startup`/profile module paths resolve from `modules_root` instead when `settings.modules_root` is set. `lib.rs::run` computes `let modules_root = config.modules_root(&root)` once per command and threads it into `emit`/`discover` calls — `root` itself must never be used for path resolution once `modules_root` exists at a call site.

## Context

This repo lives at `~/.local/shmod` (where the bash predecessor lived); it is the source checkout, not the resolved runtime root (which now defaults to `~/.config/shmod`). `README.md` is the user-facing doc; `TODO.md` tracks unshipped ideas (secrets management, unload/teardown, sync-wave ordering) — treat those as not-yet-designed, not current behavior. `example/` is a runnable module tree for manual testing (`SHMOD_ROOT=example target/release/shmod ...`).
