# shmod

A context-selective bash configuration loader, written in Rust. `shmod`
organizes your environment variables, aliases, and functions into a tree of
`.sh` modules and lets you load them on demand — a startup set that every shell
gets, plus named **profiles** you switch between (`k8s`, `infra`, `git`, ...).

Because a program can't mutate its parent shell, `shmod` emits `source` lines to
stdout and a shell shim `eval`s them.

## How it works

- **Module tree** — a directory of `.sh` files grouped into subdirectories.
- **`shmod.yaml`** — at the tree root; defines `settings` and `profiles`.
- **`modules_root`** — optional; points module discovery at a different
  directory than the one holding `shmod.yaml` (see below).
- **Startup list** — paths in `settings.startup` sourced by every new shell,
  in order (files and dirs both allowed; dirs expand via discovery).
- **Profiles** — named lists of module paths you load on demand (each path is a
  file or a dir, same as the startup list; dirs expand via discovery). A
  committed `default:` in `shmod.yaml` is loaded by new shells; a user can
  override it by persisting their own with `use --save`.
- **`.off` marker** — a file ending in `.off` is disabled and skipped by
  discovery (source it anyway with `source --force`).
- **Markers in files** — `# @dep: cmd` declares a required command (checked by
  `shmod check`).

The tree root is resolved in this order: `--root` flag → `SHMOD_ROOT` → XDG
config dir (`$XDG_CONFIG_HOME/shmod`, default `~/.config/shmod`).

## Install

```bash
cargo build --release   # binary at target/release/shmod
```

## Setup

Add to `~/.profile` (with `shmod` on your `PATH`):

```bash
export SHMOD_ROOT="/path/to/your/module/tree"
eval "$(command shmod init bash)"
```

This exports `SHMOD_ROOT`, installs the `shmod()` shim, and sources the startup
list plus the active default profile — the user's last `use --save`, or the
committed `default:` from `shmod.yaml` if none is saved.

Pass `--profile <name>` to load a specific profile for this shell instead of the
default (e.g. `eval "$(command shmod init bash --profile k8s)"`). It overrides
both the persisted and committed defaults and does not persist.

## Commands

| Command | Description |
| --- | --- |
| `shmod init bash` | Emit shell integration (shim + startup sourcing). |
| `shmod init bash --profile <name>` | Same, but load `<name>` for this shell instead of the default (overrides persisted/committed; not persisted). |
| `shmod use <profile>` | Source a profile's modules into the current shell. |
| `shmod use <profile> --save` | Same, and persist it as the default for new shells. |
| `shmod source <paths>` | Source specific files/dirs (`:`-separated allowed). |
| `shmod source <path> --force` | Source regardless of extension or `.off` marker. |
| `shmod reload` | Re-source the startup list + active profile into the current shell. |
| `shmod reset [--save]` | Clear the active profile; `--save` clears the persisted default. |
| `shmod profiles` | List profiles from `shmod.yaml`, each as a module tree; the resolved profile is tagged `● active`, an overridden committed default `○ default`. |
| `shmod active` | Show the active default profile (persisted, or committed `default:`). |
| `shmod list [--mode all\|enabled\|disabled]` | Show discovered modules as a tree, tagged `[on]`/`[off]`. |
| `shmod check <paths>` | Report declared `@dep` commands missing from `PATH`. |

Subcommands that emit shell code (`init`, `use`, `source`, `reload`, `reset`)
are wrapped in `eval` by the shim; the rest run directly.

## Configuration (`shmod.yaml`)

```yaml
settings:
  extensions: ["sh"]      # sourceable file extensions
  off_extension: "off"    # marker that disables a file
  startup:                # sourced by every new shell, in order
    - "core/environment.sh"
    - "core/alias.sh"
    - "core/function.sh"
  # modules_root: "~/dotfiles/shmod"   # optional, see below

# Committed default profile loaded by new shells (a user's `use --save`
# overrides it). Omit for no default.
default: personal

profiles:
  # Context profiles: a shared base/ module + a per-context overlay, listed
  # last so its exports/aliases win over the shared base.
  work:     ["base/git", "contexts/work"]
  personal: ["base/git", "contexts/personal"]
```

This mirrors the runnable [example/](example/) tree.

### Separate module tree location (`modules_root`)

By default module files live alongside `shmod.yaml`: `startup` and profile
paths are resolved relative to the tree root. Set `settings.modules_root` to
load modules from elsewhere instead — e.g. a dotfiles repo checked out
somewhere else, or a directory shared across machines by sync software:

```yaml
settings:
  modules_root: "~/dotfiles/shmod"
```

`modules_root` accepts:

- `~` or `~/...` — expanded against `$HOME`.
- an absolute path — used as-is.
- a relative path — joined against the tree root (the directory holding
  `shmod.yaml`).

`shmod.yaml` itself, `default`, and `profiles` stay where the tree root
resolves them (`--root` → `SHMOD_ROOT` → XDG config dir); only the
`startup`/profile module *paths* resolve under `modules_root` instead.
`SHMOD_ROOT` still exports the config root, not `modules_root`.

### Contexts (work / personal)

To drive one module tree from different environments (work laptop, personal
machine, ...), model each context as a profile — not a git branch. Branches only
let one context exist at a time and force shared modules to diverge and be
merged; profiles compose instead. Split shared modules into `base/` and
per-context differences into `contexts/`, then have each context profile layer
the base and its overlay, listed last so it wins:

```yaml
profiles:
  work:     ["base/git", "contexts/work"]
  personal: ["base/git", "contexts/personal"]
```

Shared config lives once in the base; each context holds only its differences
(identity, remotes, host-specific helpers). Select a context once per machine
and persist it with `shmod use work --save`; new shells then auto-load it.
Caveat: both contexts coexist in the tree, so keep secrets out of the module
tree if a work overlay must not be present on a personal machine.

## Example

The [example/](example/) directory is a self-contained module tree you can point
`SHMOD_ROOT` at to try every command without touching your real shell config.
See [example/README.md](example/README.md).

## Files & locations

`shmod` follows the XDG Base Directory convention, keeping user-authored
**config** separate from program-managed **state**:

- **Config (`shmod.yaml`)** lives at the module tree root — yours to edit and
  version-control. Resolved as `--root` → `SHMOD_ROOT` → XDG config dir
  (`$XDG_CONFIG_HOME/shmod`, default `~/.config/shmod`).
- **Modules** live alongside `shmod.yaml` by default, or wherever
  `settings.modules_root` points if set (see
  [Separate module tree location](#separate-module-tree-location-modules_root)).
- **State (active profile)** is written by the program on `use --save` and
  stored at `${XDG_STATE_HOME:-$HOME/.local/state}/shmod/active-profile`. It's a
  runtime selection, not configuration, so it stays out of your config and can
  be cleared with `reset --save`.
