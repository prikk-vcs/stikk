//! The `stikk` launcher (design `stikk-04` MOD-08, external design CL-01…08).
//!
//! stikk is a history browser and workbench for the prikk version control system. The launcher's job
//! is small: parse the launch contract, run the headless utilities, and open a repository. The
//! interactive TUI/GUI render loop is the next increment (its toolkit is a Program-Design decision,
//! deliberately not made here); until then, opening a repository prints a one-shot orientation so the
//! foundation is runnable end-to-end against a real prikk.
//!
//! Exit codes (a subset of external design CL-05, honest for this non-interactive increment):
//! `0` success · `2` usage error · `3` config check failed · `1` runtime error.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use stikk_core::orient;
use stikk_prikk::CliBackend;
use stikk_state::{Config, RepositoryHandle, paths};

const USAGE: &str = "\
stikk — a history browser and workbench for the prikk version control system

USAGE:
    stikk [PATH]            open the repository at PATH (or discovered from the current directory)
    stikk config check [F]  validate the stikk config file (F, or the default location)
    stikk config path       print where stikk's config and state live
    stikk --version         print the stikk version
    stikk --help            print this help

Notes:
    The interactive TUI is the next increment; opening a repository currently prints a one-shot
    orientation. stikk drives prikk through its public CLI; set STIKK_PRIKK_BIN to point at a
    specific prikk build. stikk reads PRIKK_*_SEED presence only, never their values.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.split_first() {
        None => run_open(None),
        Some((cmd, rest)) => match cmd.as_str() {
            "--version" | "-V" => {
                println!("stikk {}", env!("CARGO_PKG_VERSION"));
                ExitCode::SUCCESS
            }
            "--help" | "-h" | "help" => {
                println!("{USAGE}");
                ExitCode::SUCCESS
            }
            "config" => run_config(rest),
            other if other.starts_with('-') => {
                eprintln!("error: unknown option: {other}\n\n{USAGE}");
                ExitCode::from(2)
            }
            path => run_open(Some(path)),
        },
    }
}

/// `stikk config check [file]` and `stikk config path`.
fn run_config(rest: &[String]) -> ExitCode {
    match rest.split_first() {
        Some((sub, tail)) if sub == "check" => {
            let file = match tail.first() {
                Some(explicit) => PathBuf::from(explicit),
                None => match paths::config_file() {
                    Ok(path) => path,
                    Err(err) => return fail(&err.to_string()),
                },
            };
            config_check(&file)
        }
        Some((sub, _)) if sub == "path" => config_path(),
        _ => {
            eprintln!("error: expected `config check [file]` or `config path`\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn config_check(file: &Path) -> ExitCode {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            // A missing config is not an error: stikk runs on defaults (external design CF-02).
            println!("no config at {} — stikk uses defaults", file.display());
            return ExitCode::SUCCESS;
        }
        Err(err) => return fail(&format!("could not read {}: {err}", file.display())),
    };
    let config = Config::parse(&text);
    if config.warnings.is_empty() {
        println!("config at {} is valid", file.display());
        ExitCode::SUCCESS
    } else {
        // Warnings do not block launch (unknown keys are preserved), but `config check` reports them
        // and exits non-zero so a CI gate can catch a typo (external design CL-08, exit 3).
        println!(
            "config at {} has {} notice(s):",
            file.display(),
            config.warnings.len()
        );
        for warning in &config.warnings {
            println!("  - {warning}");
        }
        ExitCode::from(3)
    }
}

fn config_path() -> ExitCode {
    match (paths::config_file(), paths::state_dir()) {
        (Ok(config), Ok(state)) => {
            println!("config: {}", config.display());
            println!("state:  {}", state.display());
            ExitCode::SUCCESS
        }
        (Err(err), _) | (_, Err(err)) => fail(&err.to_string()),
    }
}

/// Open a repository. On a TTY, launch the interactive TUI (design CL-06); otherwise print a one-shot
/// orientation, which stays the machine-friendly path for pipes and CI.
fn run_open(path: Option<&str>) -> ExitCode {
    let start = match path {
        Some(p) => PathBuf::from(p),
        None => match std::env::current_dir() {
            Ok(dir) => dir,
            Err(err) => return fail(&format!("could not read the current directory: {err}")),
        },
    };
    let handle = match open_handle(&start) {
        Ok(handle) => handle,
        Err(err) => return fail(&err.to_string()),
    };
    let backend = CliBackend::new();

    if stikk_tui::stdout_is_tty() {
        let config = load_config();
        return match stikk_tui::run(handle.root(), &backend, &config) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => fail(&err.to_string()),
        };
    }

    // Non-TTY (piped, redirected, CI): the one-shot orientation.
    match orient(&backend, handle.root()) {
        Ok(view) => {
            print_orientation(handle.root(), &view);
            ExitCode::SUCCESS
        }
        Err(err) => fail(&err.to_string()),
    }
}

/// Load the stikk config, falling back to defaults on any problem (design CF-02: a bad config never
/// blocks launch).
fn load_config() -> Config {
    let Ok(path) = paths::config_file() else {
        return Config::default();
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => Config::parse(&text),
        Err(_) => Config::default(),
    }
}

/// If the path is a repository root use it directly; otherwise discover upward.
fn open_handle(start: &Path) -> stikk_model::Result<RepositoryHandle> {
    match RepositoryHandle::open(start) {
        Ok(handle) => Ok(handle),
        Err(_) => RepositoryHandle::discover(start),
    }
}

fn print_orientation(root: &Path, view: &orient::OrientationView) {
    println!("stikk — orientation");
    println!("  repository:  {}", root.display());
    // RFC 009 decisions 6-7: below the floor stikk degrades; above the validated ceiling it still
    // runs, but says so rather than silently asserting a validation it has not done.
    let support = if !view.prikk_supported {
        "OUTSIDE stikk's validated range — read-only"
    } else if !view.prikk_validated {
        "validated through 0.30 — this prikk is newer; its output shapes have not been checked"
    } else {
        "supported"
    };
    println!("  prikk:       {} ({support})", view.prikk_version);
    println!("  capability:  {}", view.capability.name());
    println!(
        "  signing:     author {} · maintainer {}{}",
        ready(view.readiness.author_ready),
        ready(view.readiness.maintainer_ready),
        if view.readiness.read_only {
            " · read-only"
        } else {
            ""
        }
    );
    match &view.queued_target {
        Some(target) => println!(
            "  queued:      {} · targeting {target}",
            view.queued_patches
        ),
        None => println!("  queued:      {}", view.queued_patches),
    }
    if view.trailing_partial_wal_bytes != 0 {
        println!(
            "  warning:     {} trailing partial WAL byte(s) — an interrupted commit left a torn tail",
            view.trailing_partial_wal_bytes
        );
    }
    match &view.main_ref_state {
        Some(id) => println!("  heads/main:  {id}"),
        None => println!("  heads/main:  <unpublished>"),
    }
    println!(
        "\nRunning without a terminal, so this is the one-shot orientation; open stikk in a \
         terminal for the interactive view."
    );
}

fn ready(flag: bool) -> &'static str {
    if flag { "ready" } else { "not ready" }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(1)
}
