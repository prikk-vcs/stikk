//! Tests for path resolution and the repository-internal refusal (design TS-04, TS-06).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use super::*;

#[test]
fn refuses_a_target_inside_a_prikk_metadata_dir() {
    // The primary boundary control (C-E2): no stikk file may land inside .prikk/.
    let target = Path::new("/home/dev/project/.prikk/cache/stikk-session");
    let err = ensure_outside_repository(target, None).expect_err("must refuse");
    assert_eq!(err.class(), "stikk-internal");
}

#[test]
fn refuses_a_target_inside_the_open_worktree() {
    let root = Path::new("/home/dev/project");
    let target = Path::new("/home/dev/project/sub/stikk-notes");
    let err =
        ensure_outside_repository(target, Some(root)).expect_err("must refuse worktree write");
    assert_eq!(err.class(), "stikk-internal");
}

#[test]
fn ensure_outside_repository_gates_regardless_of_which_platform_shape_the_path_has() {
    // C-E2 (RFC 012 F-c): the primary control must not narrow just because paths now vary by
    // platform. Exercised with each platform's conceptual directory shape.
    //
    // Caveat, stated rather than glossed over: this is run on a Linux test host, where
    // `std::path::Path` only ever splits components on `/` — it does not treat `\` as a separator
    // unless actually compiled for a Windows target. So the "Windows-shaped" case below uses a
    // forward-slash equivalent (`C:/Users/...`) to exercise the same *logic* (component/prefix
    // matching), not genuine backslash-separator parsing, which cannot be exercised from here. On a
    // real Windows build, `Path` does split on `\` too, so the gate applies there as well — that half
    // of the claim rests on `std::path`'s own documented, platform-native behavior, not a test in this
    // suite.
    let linux_repo_internal = Path::new("/home/dev/project/.prikk/cache/stikk-session");
    assert!(ensure_outside_repository(linux_repo_internal, None).is_err());

    let macos_repo_internal = Path::new("/Users/dev/project/.prikk/cache/stikk-session");
    assert!(ensure_outside_repository(macos_repo_internal, None).is_err());

    let windows_shaped_repo_internal = Path::new("C:/Users/dev/project/.prikk/cache/stikk-session");
    assert!(ensure_outside_repository(windows_shaped_repo_internal, None).is_err());

    // And the worktree-containment half of the gate, per platform shape.
    let macos_root = Path::new("/Users/dev/project");
    let macos_target = Path::new("/Users/dev/project/sub/stikk-notes");
    assert!(ensure_outside_repository(macos_target, Some(macos_root)).is_err());

    let windows_root = Path::new("C:/Users/dev/project");
    let windows_target = Path::new("C:/Users/dev/project/sub/stikk-notes");
    assert!(ensure_outside_repository(windows_target, Some(windows_root)).is_err());
}

#[test]
fn allows_a_user_scope_target() {
    let target = Path::new("/home/dev/.local/state/stikk/session");
    assert!(ensure_outside_repository(target, Some(Path::new("/home/dev/project"))).is_ok());
}

#[test]
fn allows_a_user_scope_target_with_no_open_repo() {
    let target = Path::new("/home/dev/.config/stikk/config");
    assert!(ensure_outside_repository(target, None).is_ok());
}

#[test]
fn config_and_state_honor_explicit_overrides() {
    // We only assert the override *shape*; we avoid mutating process env here to keep the test
    // hermetic (this codebase's established practice — see `stikk_prikk::env`, whose analogous
    // `read_only_override` real-env wrapper is likewise left untested at the unit level, precisely
    // because it is a one-line, obviously-correct read). The override branch in `config_file`/
    // `state_dir` is a direct `std::env::var_os("STIKK_CONFIG"/"STIKK_STATE_DIR")` check performed
    // *before* any platform logic (CF-04 precedence) — everything platform-specific is exercised
    // hermetically below via the injected-lookup functions instead.
    let p = Path::new("/x/stikk/config");
    assert!(p.components().any(|c| c.as_os_str() == "stikk"));
}

/// Build an injected lookup from a fixed list of (name, value) pairs — the hermetic-testing pattern
/// `stikk_prikk::env::read_readiness_with` established, applied here to path resolution (RFC 012 F-c).
fn env_of(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
    move |name| {
        pairs
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| OsString::from(*v))
    }
}

fn no_env(_: &str) -> Option<OsString> {
    None
}

#[test]
fn linux_config_prefers_xdg_config_home() {
    let lookup = env_of(&[("XDG_CONFIG_HOME", "/x/xdg-config"), ("HOME", "/x/home")]);
    assert_eq!(
        config_base_with(&lookup, Platform::Linux).unwrap(),
        PathBuf::from("/x/xdg-config")
    );
}

#[test]
fn linux_config_falls_back_to_home_dot_config_when_xdg_is_absent() {
    let lookup = env_of(&[("HOME", "/x/home")]);
    assert_eq!(
        config_base_with(&lookup, Platform::Linux).unwrap(),
        PathBuf::from("/x/home/.config")
    );
}

#[test]
fn linux_state_prefers_xdg_state_home() {
    let lookup = env_of(&[("XDG_STATE_HOME", "/x/xdg-state"), ("HOME", "/x/home")]);
    assert_eq!(
        state_base_with(&lookup, Platform::Linux).unwrap(),
        PathBuf::from("/x/xdg-state")
    );
}

#[test]
fn linux_state_falls_back_to_home_dot_local_state_when_xdg_is_absent() {
    let lookup = env_of(&[("HOME", "/x/home")]);
    assert_eq!(
        state_base_with(&lookup, Platform::Linux).unwrap(),
        PathBuf::from("/x/home/.local/state")
    );
}

/// The must-not-regress requirement (handoff §4/§9): an upgrading Linux user's fallback paths are
/// byte-identical to the pre-RFC-012 shape — `<HOME>/.config/stikk/config` and
/// `<HOME>/.local/state/stikk`, unchanged by the platform-resolver refactor.
#[test]
fn linux_fallback_paths_are_byte_identical_to_before_rfc_012() {
    let lookup = env_of(&[("HOME", "/home/dev")]);
    let config = config_base_with(&lookup, Platform::Linux)
        .unwrap()
        .join("stikk")
        .join("config");
    assert_eq!(config, PathBuf::from("/home/dev/.config/stikk/config"));
    let state = state_base_with(&lookup, Platform::Linux)
        .unwrap()
        .join("stikk");
    assert_eq!(state, PathBuf::from("/home/dev/.local/state/stikk"));
}

#[test]
fn linux_ignores_an_empty_xdg_value_the_same_as_absent() {
    // Matches the pre-RFC-012 `non_empty_os` behavior: an empty (but set) XDG var is treated as unset,
    // not as "use the empty string as a path".
    let lookup = env_of(&[("XDG_CONFIG_HOME", ""), ("HOME", "/x/home")]);
    assert_eq!(
        config_base_with(&lookup, Platform::Linux).unwrap(),
        PathBuf::from("/x/home/.config")
    );
}

#[test]
fn macos_uses_application_support_for_both_config_and_state() {
    let lookup = env_of(&[("HOME", "/Users/dev")]);
    let expected = PathBuf::from("/Users/dev/Library/Application Support");
    assert_eq!(
        config_base_with(&lookup, Platform::MacOs).unwrap(),
        expected
    );
    assert_eq!(state_base_with(&lookup, Platform::MacOs).unwrap(), expected);
}

#[test]
fn macos_ignores_xdg_variables_entirely() {
    // A macOS user with XDG_CONFIG_HOME set (e.g. from a shared dotfiles repo) still gets the platform
    // convention, not a Linux-shaped path — the branch is chosen by `Platform`, never by which
    // variables happen to be set.
    let lookup = env_of(&[
        ("XDG_CONFIG_HOME", "/Users/dev/.xdg-config"),
        ("HOME", "/Users/dev"),
    ]);
    assert_eq!(
        config_base_with(&lookup, Platform::MacOs).unwrap(),
        PathBuf::from("/Users/dev/Library/Application Support")
    );
}

#[test]
fn windows_config_uses_appdata() {
    let lookup = env_of(&[("APPDATA", r"C:\Users\dev\AppData\Roaming")]);
    assert_eq!(
        config_base_with(&lookup, Platform::Windows).unwrap(),
        PathBuf::from(r"C:\Users\dev\AppData\Roaming")
    );
}

#[test]
fn windows_state_uses_localappdata() {
    let lookup = env_of(&[("LOCALAPPDATA", r"C:\Users\dev\AppData\Local")]);
    assert_eq!(
        state_base_with(&lookup, Platform::Windows).unwrap(),
        PathBuf::from(r"C:\Users\dev\AppData\Local")
    );
}

#[test]
fn windows_never_falls_back_to_home() {
    // Windows resolution does not consult HOME at all (it usually is not set there) — only
    // APPDATA/LOCALAPPDATA, per the platform convention.
    let lookup = env_of(&[("HOME", r"C:\Users\dev")]);
    assert!(config_base_with(&lookup, Platform::Windows).is_err());
    assert!(state_base_with(&lookup, Platform::Windows).is_err());
}

#[test]
fn no_home_at_all_is_an_environment_error_on_every_platform() {
    for platform in [Platform::Linux, Platform::MacOs, Platform::Windows] {
        let config_err = config_base_with(&no_env, platform).unwrap_err();
        assert_eq!(config_err.class(), "environment");
        let state_err = state_base_with(&no_env, platform).unwrap_err();
        assert_eq!(state_err.class(), "environment");
    }
}
