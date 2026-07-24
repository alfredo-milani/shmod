# shmod example tree

A self-contained module tree showing the scaffolding `shmod` expects and how to
drive it. Point `SHMOD_ROOT` at this directory to try every command without
touching your real shell config.

## Layout

```
example/
  shmod.yaml              # settings + profile -> module mapping
  .core/                  # sourced at startup via the settings.startup list in
    environment.sh        #   shmod.yaml; order = list position (files and dirs
    alias.sh              #   both allowed, dirs expand via discovery)
    function.sh
  preload/                # also in the startup list, after .core (OS deps, etc.)
    osdep.sh
  dev/                    # optional modules, grouped by profile in shmod.yaml
    kubernetes/kubernetes.sh
    kubernetes/experimental.sh.off   # ".off" = disabled, skipped by discovery
    kind/kind.sh
    terraform/terraform.sh
  base/                   # shared modules reused across context profiles
    git/git.sh
  contexts/               # per-context overlays layered on top of base/
    work/git.sh           #   work identity + host-specific helpers
    personal/git.sh       #   personal identity + host-specific helpers
```

Profiles in [shmod.yaml](shmod.yaml): `k8s` (kubernetes + kind), `infra`
(terraform), `git`, plus two **context** profiles, `work` and `personal`.

## Contexts (work / personal)

To run one module tree across different environments (work laptop, personal
machine, ...), model each context as a profile rather than a git branch. Split
shared modules into `base/` and per-context differences into `contexts/`, then
have each context profile layer the base and its overlay:

```yaml
profiles:
  work:     ["base/git", "contexts/work"]
  personal: ["base/git", "contexts/personal"]
```

The overlay is listed last, so its exports and aliases win over the shared
`base/git` module — shared config lives once, each context holds only its
differences (identity, remotes, host-specific helpers). Pick the context once
per machine and persist it:

```bash
shmod use work --save   # this machine now defaults to work; new shells load it
shmod use personal      # switch THIS shell only, without persisting
```

Both contexts coexist in the tree, so anything sensitive in `contexts/work/` is
also present on a personal machine — keep secrets out of the module tree if that
matters. If the contexts share nothing, drop `base/git` and use
`["contexts/work"]` alone.

## Try it

Build the binary once from the repo root (`cargo build`), then from the repo
root run (the `--root` flag pins discovery to this example tree):

```bash
BIN=./target/debug/shmod
export SHMOD_ROOT="$PWD/example"

# What would a new shell source? (the startup list + the committed default profile, git)
$BIN init bash

# Inspect the tree
$BIN profiles                 # k8s / infra / git and their modules
$BIN list --mode all          # every module, on/off tagged
$BIN list --mode disabled     # just the .off modules
$BIN active                   # -> "git (default)" (committed default, none persisted yet)

# Dependency checks pulled from the `# @dep:` markers
$BIN check dev/kubernetes     # reports kubectl if it's missing from PATH

# Emit source lines for a profile (what the shim evals)
$BIN use k8s
```

## Wire it into a live shell

The binary can't mutate its parent shell directly, so a shim `eval`s its
output. Add to `~/.profile` (with `shmod` on your `PATH`):

```bash
export SHMOD_ROOT="/path/to/example"
eval "$(command shmod init bash)"
```

Then, in any shell:

```bash
shmod use k8s --save   # source k8s now AND persist it as the startup default
shmod use infra        # source infra into THIS shell only (not persisted)
shmod reload            # re-source the startup list + active profile into THIS shell
shmod reset --save      # clear the persisted default; new shells load the startup list only
shmod source dev/kubernetes/experimental.sh.off --force   # load a disabled module ad-hoc
```

New shells auto-load the `startup` list (`.core` + `preload`) plus the active default profile — whichever was last saved with `use --save`, or the committed `default:` (`git`) from `shmod.yaml` if none is saved.
