use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_shmod")
}

const CONFIG_NO_DEFAULT: &str = "settings:\n  extensions: [\"sh\"]\n  off_extension: \"off\"\n  startup:\n    - \".core/environment.sh\"\n    - \".core/alias.sh\"\n    - \".core/function.sh\"\n    - \"preload\"\nprofiles:\n  k8s: [\"dev/kind\", \"dev/helm\"]\n  git: [\"dev/git\"]\n";

const CONFIG_WITH_DEFAULT: &str = "settings:\n  extensions: [\"sh\"]\n  off_extension: \"off\"\n  startup:\n    - \".core/environment.sh\"\n    - \".core/alias.sh\"\n    - \".core/function.sh\"\n    - \"preload\"\ndefault: git\nprofiles:\n  k8s: [\"dev/kind\", \"dev/helm\"]\n  git: [\"dev/git\"]\n";

/// Build a fixture module tree with core, preload, and profile modules.
fn fixture() -> TempDir {
    fixture_with_config(CONFIG_NO_DEFAULT)
}

/// Same tree, but with an explicit `shmod.yaml` body so tests can vary the
/// `default:` key without duplicating the module layout.
fn fixture_with_config(config: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(root.join(".core/environment.sh"), "export MOD=x\n");
    write(root.join(".core/alias.sh"), "alias l='ls'  # @doc: list\n");
    write(
        root.join(".core/function.sh"),
        "foo() {  # @doc: does foo\n:\n}\n",
    );

    write(root.join("preload/osdep.sh"), "export OS=mac\n");

    write(
        root.join("dev/kind/kind.sh"),
        "# @dep: kind docker\nknd() {  # @doc: kind helper\n:\n}\n",
    );
    write(root.join("dev/kind/old.sh.off"), "echo disabled\n");
    write(root.join("dev/helm/helm.sh"), "export H=1\n");
    write(root.join("dev/git/git.sh"), "export G=1\n");

    write(root.join("shmod.yaml"), config);

    dir
}

fn write<P: AsRef<Path>>(path: P, contents: &str) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn run(root: &Path, state: &Path, args: &[&str]) -> String {
    let out = Command::new(bin())
        .args(["--root", root.to_str().unwrap()])
        .args(args)
        .env("XDG_STATE_HOME", state)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "command {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// Run and return `(success, stdout, stderr)` without asserting success, for
/// exercising error paths.
fn try_run(root: &Path, state: &Path, args: &[&str]) -> (bool, String, String) {
    let out = Command::new(bin())
        .args(["--root", root.to_str().unwrap()])
        .args(args)
        .env("XDG_STATE_HOME", state)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn init_sources_core_and_preload_but_no_profile_when_none_active() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();
    let out = run(f.path(), state.path(), &["init", "bash"]);

    assert!(out.contains("export SHMOD_ROOT="));
    assert!(out.contains("shmod() {"));
    assert!(out.contains(".core/environment.sh"));
    assert!(out.contains(".core/alias.sh"));
    assert!(out.contains(".core/function.sh"));
    assert!(out.contains("preload/osdep.sh"));
    // no profile active -> no profile modules
    assert!(!out.contains("dev/kind/kind.sh"));
    assert!(!out.contains("dev/helm/helm.sh"));
}

#[test]
fn use_emits_profile_modules_and_skips_off_files() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();
    let out = run(f.path(), state.path(), &["use", "k8s"]);

    assert!(out.contains("dev/kind/kind.sh"));
    assert!(out.contains("dev/helm/helm.sh"));
    assert!(!out.contains("old.sh.off"));
}

#[test]
fn use_save_persists_and_init_loads_it() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();
    run(f.path(), state.path(), &["use", "k8s", "--save"]);

    let active = run(f.path(), state.path(), &["active"]);
    assert_eq!(active.trim(), "k8s");

    let out = run(f.path(), state.path(), &["init", "bash"]);
    assert!(out.contains("dev/kind/kind.sh"));
    assert!(out.contains("dev/helm/helm.sh"));

    run(f.path(), state.path(), &["reset", "--save"]);
    let active = run(f.path(), state.path(), &["active"]);
    assert_eq!(active.trim(), "(none)");
}

#[test]
fn reload_sources_startup_without_shim_or_export() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();
    let out = run(f.path(), state.path(), &["reload"]);

    // Re-sources the startup list...
    assert!(out.contains(".core/environment.sh"));
    assert!(out.contains("preload/osdep.sh"));
    // ...but does not re-emit the export or shim (that's init's job).
    assert!(!out.contains("export SHMOD_ROOT="));
    assert!(!out.contains("shmod() {"));
    // No active profile -> no profile modules.
    assert!(!out.contains("dev/kind/kind.sh"));
}

#[test]
fn reload_includes_active_profile() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();
    run(f.path(), state.path(), &["use", "k8s", "--save"]);

    let out = run(f.path(), state.path(), &["reload"]);
    assert!(out.contains(".core/environment.sh"));
    assert!(out.contains("dev/kind/kind.sh"));
    assert!(out.contains("dev/helm/helm.sh"));
}

#[test]
fn reload_uses_committed_default_when_none_persisted() {
    let f = fixture_with_config(CONFIG_WITH_DEFAULT);
    let state = tempfile::tempdir().unwrap();
    let out = run(f.path(), state.path(), &["reload"]);

    // committed `default: git` re-sources its module, like init does.
    assert!(out.contains("dev/git/git.sh"));
    assert!(!out.contains("dev/kind/kind.sh"));
}

#[test]
fn source_force_includes_off_files() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();

    let normal = run(f.path(), state.path(), &["source", "dev/kind"]);
    assert!(normal.contains("kind.sh"));
    assert!(!normal.contains("old.sh.off"));

    let forced = run(f.path(), state.path(), &["source", "dev/kind", "--force"]);
    assert!(forced.contains("old.sh.off"));
}

#[test]
fn list_modes_filter_enabled_and_disabled() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();

    let all = run(f.path(), state.path(), &["list", "--mode", "all"]);
    // Tree output: dir components on their own lines, module files as leaves.
    assert!(all.contains("kind"));
    assert!(all.contains("kind.sh  [on]"));
    assert!(all.contains("old.sh.off  [off]"));
    // core is excluded from listing
    assert!(!all.contains(".core"));

    let enabled = run(f.path(), state.path(), &["list", "--mode", "enabled"]);
    assert!(enabled.contains("kind.sh  [on]"));
    assert!(!enabled.contains("old.sh.off"));

    let disabled = run(f.path(), state.path(), &["list", "--mode", "disabled"]);
    assert!(disabled.contains("old.sh.off  [off]"));
    assert!(!disabled.contains("kind.sh  [on]"));
}

#[test]
fn list_collapses_fully_disabled_directory() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();
    // A directory whose entire subtree is disabled.
    write(f.path().join("dev/dead/a.sh.off"), "echo a\n");
    write(f.path().join("dev/dead/nested/b.sh.off"), "echo b\n");

    let all = run(f.path(), state.path(), &["list", "--mode", "all"]);
    // The directory collapses to one `[off]` line; its children are not expanded.
    assert!(all.contains("dead  [off]"));
    assert!(!all.contains("a.sh.off"));
    assert!(!all.contains("nested"));
    // A mixed directory still expands its files.
    assert!(all.contains("kind.sh  [on]"));
}

#[test]
fn check_reports_missing_deps() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();
    let out = run(f.path(), state.path(), &["check", "dev/kind/kind.sh"]);
    // "kind" is almost certainly not installed in CI; assert the format at least.
    assert!(out.contains("requires:") || out.contains("all dependencies satisfied"));
}

#[test]
fn profiles_lists_defined_profiles() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();
    let out = run(f.path(), state.path(), &["profiles"]);
    assert!(out.contains("k8s:"));
}

// --- Gap 1: committed `default:` profile precedence ---

#[test]
fn init_loads_committed_default_profile_when_none_persisted() {
    let f = fixture_with_config(CONFIG_WITH_DEFAULT);
    let state = tempfile::tempdir().unwrap();
    let out = run(f.path(), state.path(), &["init", "bash"]);

    // committed `default: git` loads its module...
    assert!(out.contains("dev/git/git.sh"));
    // ...but not other profiles.
    assert!(!out.contains("dev/kind/kind.sh"));
    assert!(!out.contains("dev/helm/helm.sh"));
}

#[test]
fn active_reports_committed_default_when_none_persisted() {
    let f = fixture_with_config(CONFIG_WITH_DEFAULT);
    let state = tempfile::tempdir().unwrap();
    let out = run(f.path(), state.path(), &["active"]);
    assert_eq!(out.trim(), "git (default)");
}

#[test]
fn persisted_default_overrides_committed_default() {
    let f = fixture_with_config(CONFIG_WITH_DEFAULT);
    let state = tempfile::tempdir().unwrap();
    run(f.path(), state.path(), &["use", "k8s", "--save"]);

    let active = run(f.path(), state.path(), &["active"]);
    assert_eq!(active.trim(), "k8s");

    let out = run(f.path(), state.path(), &["init", "bash"]);
    assert!(out.contains("dev/kind/kind.sh"));
    // committed default no longer applies once a user default is persisted.
    assert!(!out.contains("dev/git/git.sh"));
}

#[test]
fn reset_save_falls_back_to_committed_default() {
    let f = fixture_with_config(CONFIG_WITH_DEFAULT);
    let state = tempfile::tempdir().unwrap();
    run(f.path(), state.path(), &["use", "k8s", "--save"]);
    run(f.path(), state.path(), &["reset", "--save"]);

    // Clearing the persisted default returns to the committed baseline, not none.
    let active = run(f.path(), state.path(), &["active"]);
    assert_eq!(active.trim(), "git (default)");

    let out = run(f.path(), state.path(), &["init", "bash"]);
    assert!(out.contains("dev/git/git.sh"));
    assert!(!out.contains("dev/kind/kind.sh"));
}

#[test]
fn active_reports_none_without_committed_or_persisted_default() {
    let f = fixture_with_config(CONFIG_NO_DEFAULT);
    let state = tempfile::tempdir().unwrap();
    let out = run(f.path(), state.path(), &["active"]);
    assert_eq!(out.trim(), "(none)");
}

// --- Gap 2: error paths ---

#[test]
fn use_unknown_profile_fails_with_message() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();
    let (ok, stdout, stderr) = try_run(f.path(), state.path(), &["use", "nope"]);

    assert!(!ok, "expected non-zero exit for unknown profile");
    assert!(
        stderr.contains("nope"),
        "stderr should name the profile: {stderr}"
    );
    // Must not emit source lines the shim would eval.
    assert!(
        !stdout.contains("source "),
        "no shell should be emitted: {stdout}"
    );
}

#[test]
fn unknown_subcommand_fails() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();
    let (ok, _stdout, _stderr) = try_run(f.path(), state.path(), &["frobnicate"]);
    assert!(!ok, "clap should reject an unknown subcommand");
}

// --- Gap 4: startup ordering ---

#[test]
fn startup_sources_in_declared_list_order() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();
    let out = run(f.path(), state.path(), &["init", "bash"]);

    let env = out
        .find(".core/environment.sh")
        .expect("environment sourced");
    let alias = out.find(".core/alias.sh").expect("alias sourced");
    let func = out.find(".core/function.sh").expect("function sourced");
    let preload = out.find("preload/osdep.sh").expect("preload sourced");

    // startup list order in shmod.yaml is environment -> alias -> function -> preload.
    assert!(
        env < alias && alias < func && func < preload,
        "startup emitted out of order: env={env} alias={alias} func={func} preload={preload}"
    );
}

// --- Gap 5: root resolution ---

/// Run with an explicit cwd and env overrides, without passing `--root`, so we
/// can exercise the SHMOD_ROOT / XDG_CONFIG_HOME precedence chain.
fn run_in(cwd: &Path, envs: &[(&str, &Path)], args: &[&str]) -> (bool, String, String) {
    let mut cmd = Command::new(bin());
    cmd.args(args).current_dir(cwd);
    // Clear any inherited SHMOD_ROOT / XDG_CONFIG_HOME so tests fully control the
    // resolution chain; callers that want them set can pass them via `envs`.
    cmd.env_remove("SHMOD_ROOT");
    cmd.env_remove("XDG_CONFIG_HOME");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn root_resolves_from_shmod_root_env() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();
    // cwd is an unrelated empty dir; SHMOD_ROOT points at the fixture.
    let elsewhere = tempfile::tempdir().unwrap();
    let (ok, stdout, stderr) = run_in(
        elsewhere.path(),
        &[("SHMOD_ROOT", f.path()), ("XDG_STATE_HOME", state.path())],
        &["init", "bash"],
    );

    assert!(ok, "init should succeed via SHMOD_ROOT: {stderr}");
    assert!(stdout.contains("export SHMOD_ROOT="));
    assert!(stdout.contains(".core/environment.sh"));
}

#[test]
fn root_falls_back_to_home_local_shmod() {
    // Build the module tree at $HOME/.local/shmod and point HOME at a fake home.
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join(".local/shmod");
    write(root.join(".core/environment.sh"), "export MOD=x\n");
    write(root.join("shmod.yaml"), CONFIG_NO_DEFAULT);

    let state = tempfile::tempdir().unwrap();
    // cwd is an unrelated dir with no shmod.yaml upward, and SHMOD_ROOT is
    // cleared by run_in, so resolution must reach the ~/.local/shmod fallback.
    let elsewhere = tempfile::tempdir().unwrap();
    let (ok, stdout, stderr) = run_in(
        elsewhere.path(),
        &[("HOME", home.path()), ("XDG_STATE_HOME", state.path())],
        &["profiles"],
    );

    assert!(
        ok,
        "profiles should succeed via ~/.local/shmod fallback: {stderr}"
    );
    assert!(
        stdout.contains("k8s:"),
        "fallback config not used: {stdout}"
    );
}

#[test]
fn root_resolves_from_xdg_config_home() {
    // XDG_CONFIG_HOME resolves to $XDG_CONFIG_HOME/shmod.
    let xdg = tempfile::tempdir().unwrap();
    let root = xdg.path().join("shmod");
    write(root.join(".core/environment.sh"), "export MOD=x\n");
    write(root.join("dev/git/git.sh"), "export G=1\n");
    write(root.join("shmod.yaml"), CONFIG_NO_DEFAULT);

    let state = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let (ok, stdout, stderr) = run_in(
        elsewhere.path(),
        &[
            ("XDG_CONFIG_HOME", xdg.path()),
            ("XDG_STATE_HOME", state.path()),
        ],
        &["profiles"],
    );

    assert!(ok, "profiles should succeed via XDG_CONFIG_HOME: {stderr}");
    assert!(
        stdout.contains("k8s:"),
        "XDG_CONFIG_HOME config not used: {stdout}"
    );
}

#[test]
fn shmod_root_env_overrides_xdg_config_home() {
    // SHMOD_ROOT takes precedence over XDG_CONFIG_HOME.
    let f = fixture();
    // A distinct XDG tree that should be ignored in favor of SHMOD_ROOT.
    let xdg = tempfile::tempdir().unwrap();
    let ignored = xdg.path().join("shmod");
    write(ignored.join(".core/environment.sh"), "export MOD=x\n");
    write(ignored.join("shmod.yaml"), CONFIG_NO_DEFAULT);

    let state = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let (ok, stdout, stderr) = run_in(
        elsewhere.path(),
        &[
            ("SHMOD_ROOT", f.path()),
            ("XDG_CONFIG_HOME", xdg.path()),
            ("XDG_STATE_HOME", state.path()),
        ],
        &["profiles"],
    );

    assert!(ok, "profiles should succeed via SHMOD_ROOT: {stderr}");
    assert!(
        stdout.contains("k8s:"),
        "SHMOD_ROOT did not take precedence: {stdout}"
    );
}

#[test]
fn explicit_root_flag_overrides_shmod_root_env() {
    let f = fixture();
    let state = tempfile::tempdir().unwrap();
    // SHMOD_ROOT points somewhere bogus; --root should win.
    let bogus = tempfile::tempdir().unwrap();
    let mut cmd = Command::new(bin());
    cmd.args(["--root", f.path().to_str().unwrap(), "profiles"])
        .env("SHMOD_ROOT", bogus.path())
        .env("XDG_STATE_HOME", state.path());
    let out = cmd.output().unwrap();

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("k8s:"),
        "--root did not take precedence: {stdout}"
    );
}
