//! **The desktop app's binary** (RFC 070 decision 1).
//!
//! Its job is small, and the same as the terminal launcher's: read the launch contract, open a
//! repository, and run. **Every environment read is here**, so the program value itself reads
//! nothing and a test states a whole launch in one literal.
//!
//! `cargo run -p stikk-gui [PATH]`. The executable's user-facing name and its packaging are RFC
//! 071's; nothing here is published.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use stikk_state::{Config, RepositoryHandle, paths};

const USAGE: &str = "\
stikk — the desktop app for the prikk version control system

USAGE:
    stikk-gui [PATH]        open the repository at PATH (or discovered from the current directory)
    stikk-gui --help        print this help

Notes:
    stikk drives prikk through its public CLI; set STIKK_PRIKK_BIN to point at a specific prikk
    build. STIKK_PRIVATE=1 remembers nothing about this launch. STIKK_GUI_LOG=<file> writes the
    program's state after every message, which is what a measured run records.";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let path = match arguments.split_first() {
        None => None,
        Some((first, _)) if first == "--help" || first == "-h" => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Some((first, _)) if first.starts_with('-') => {
            eprintln!("error: unknown option: {first}\n\n{USAGE}");
            return ExitCode::from(2);
        }
        Some((first, _)) => Some(first.clone()),
    };
    let start = path.map_or_else(
        || std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        PathBuf::from,
    );
    let handle = match open_handle(&start) {
        Ok(handle) => handle,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(1);
        }
    };
    let root = handle.root().to_path_buf();
    let config = config();
    let settings = stikk_gui::Settings {
        session: session(&root),
        theme: config.theme,
        zoom: zoom(),
        overrides: stikk_gui_design::Overrides::stikk(),
        opening: Vec::new(),
        repo: root,
    };
    match stikk_gui::run(settings) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

/// **`STIKK_GUI_SCALE` lays the app out at a zoom, for a measured run** — and nothing else sets
/// one in this handoff.
///
/// RFC 069 rule 5's zoom (100, 115, 130 and 150 %, by key, remembered) is RFC 070 Handoff B's. But
/// Handoff A's own real-window reading sequence is measured **at 200 %**, where the Glossary
/// overflows a 1280 x 720 window (§6), and rule 5 deliberately does not offer 200 % to a user. So
/// the scale a measurement needs comes from the environment, where the run can state it, and not
/// from a key nobody has.
///
/// An unparsable or absent value is 100 %: a measurement that wanted another says so in its record.
fn zoom() -> f32 {
    std::env::var("STIKK_GUI_SCALE")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|scale: &f32| *scale > 0.0)
        .unwrap_or(1.0)
}

/// If the path is a repository root use it directly; otherwise discover upward.
fn open_handle(start: &Path) -> stikk_model::Result<RepositoryHandle> {
    match RepositoryHandle::open(start) {
        Ok(handle) => Ok(handle),
        Err(_) => RepositoryHandle::discover(start),
    }
}

/// The user's configuration, or the defaults when there is none to read.
fn config() -> Config {
    let Ok(path) = paths::config_file() else {
        return Config::default();
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => Config::parse(&text),
        Err(_) => Config::default(),
    }
}

/// Where this repository may be remembered, or `None` for a launch that remembers nothing
/// (RFC 046 decision 6) — the launcher's own three ways, read the same way.
fn session(root: &Path) -> Option<stikk_app::Session> {
    if std::env::var_os("STIKK_PRIVATE").is_some_and(|v| v == "1") {
        return None;
    }
    let state_dir = paths::state_dir().ok()?;
    let canonical_root = root.canonicalize().ok()?;
    Some(stikk_app::Session {
        state_dir,
        canonical_root,
    })
}
