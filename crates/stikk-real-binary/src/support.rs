//! Harness machinery for the real-binary integration suite. Not test functions itself — see
//! `tests/real_binary.rs` for what actually runs.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// One resolved, real `prikk` binary this suite drives, at a known minor version.
pub struct PrikkBin {
    /// The binary's path, as given by the environment variable this was resolved from.
    pub path: PathBuf,
    /// The minor version this binary was expected — and, on construction, confirmed — to report.
    pub minor: u32,
}

impl PrikkBin {
    /// The floor of the range [`stikk_prikk::version::supported_minor_range`] reports, resolved from
    /// `STIKK_TEST_PRIKK_FLOOR_BIN`.
    ///
    /// # Panics
    /// If the environment variable is unset, or the binary it names does not report exactly this minor
    /// version on `--version` — a configuration error, meant to fail loudly rather than run the wrong
    /// binary silently.
    #[must_use]
    pub fn floor() -> Self {
        let (floor, _ceiling) = stikk_prikk::version::supported_minor_range();
        Self::resolve("STIKK_TEST_PRIKK_FLOOR_BIN", floor)
    }

    /// The ceiling of the range [`stikk_prikk::version::supported_minor_range`] reports, resolved from
    /// `STIKK_TEST_PRIKK_CEILING_BIN`.
    ///
    /// # Panics
    /// Same as [`PrikkBin::floor`].
    #[must_use]
    pub fn ceiling() -> Self {
        let (_floor, ceiling) = stikk_prikk::version::supported_minor_range();
        Self::resolve("STIKK_TEST_PRIKK_CEILING_BIN", ceiling)
    }

    fn resolve(env_var: &str, expected_minor: u32) -> Self {
        let path = env::var_os(env_var).unwrap_or_else(|| {
            panic!(
                "{env_var} is not set.\n\n\
                 This suite needs two real prikk binaries. Install one for this end of the range:\n\n\
                 \tcargo install prikk --version 0.{expected_minor}.0 --locked --root <dir>\n\n\
                 then set {env_var}=<dir>/bin/prikk before running (see this crate's `lib.rs` and \
                 `tests/real_binary.rs` module docs for the other one and the full invocation)."
            )
        });
        let path = PathBuf::from(path);
        let backend = stikk_prikk::CliBackend::with_program(&path);
        let handshake = stikk_prikk::Prikk::handshake(&backend)
            .unwrap_or_else(|e| panic!("{env_var}={path:?} did not answer `prikk --version`: {e}"));
        assert_eq!(
            handshake.version.minor, expected_minor,
            "{env_var}={path:?} reports prikk {}, but this suite's version matrix (derived from \
             stikk_prikk::version::supported_minor_range, not written here) expects minor \
             {expected_minor} at this end of the range. Install the version that range actually names \
             — do not edit this assertion to match whatever happens to be installed.",
            handshake.version
        );
        Self {
            path,
            minor: expected_minor,
        }
    }
}

/// The fixed AUTHOR/MAINTAINER key ids this suite always uses — `prikk setup`'s own choice at ≥ 0.33
/// (not configurable there), reused for consistency at < 0.33 where the manual path is free to choose
/// any string (`trust maintainer add --key-id` takes an arbitrary label; AUTHOR needs no registration
/// at all, presence-only, `stikk-prikk::env`).
const AUTHOR_KEY_ID: &str = "author";
const MAINTAINER_KEY_ID: &str = "maintainer";

/// The fixed 16-byte PKCS#8 `OneAsymmetricKey` prefix for an Ed25519 private key (RFC 8410) — identical
/// for every Ed25519 key that has ever existed; nothing here is derived from or depends on any seed.
/// Prepending it to a raw 32-byte seed makes a DER `openssl pkey` accepts, which is how this suite
/// derives a public key from a seed at prikk < 0.33, before `prikk key public --seed-env` existed.
const ED25519_PKCS8_PREFIX: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];

/// A throwaway `prikk` repository this suite builds fresh per test, with a trusted MAINTAINER key and
/// an AUTHOR key ready to use — at whichever of the two eras `bin.minor` falls in: `prikk key generate`
/// from 0.33; manual Ed25519 derivation plus `trust maintainer add` below it (RFC 016 §1's own
/// technique, now behind one function instead of being rebuilt per increment).
///
/// Both the repository and the key material live under one process-temp-dir root, removed on drop —
/// never inside a git-tracked directory, and never left behind past the test that built it short of a
/// `SIGKILL`. Seed values are read into memory only long enough to spawn a child process or write a
/// `0600` file here; [`Fixture`] never prints one, matches one into a panic message, or exposes one
/// through any public accessor — [`Fixture::set_author_env`]/[`Fixture::set_maintainer_env`] are the
/// only way a seed leaves this type, and only into this process's own environment (the `C-I1e` boundary
/// this crate's `lib.rs` module doc describes).
pub struct Fixture {
    root: PathBuf,
    repo: PathBuf,
    author_key_id: String,
    author_seed: String,
    maintainer_key_id: String,
    maintainer_seed: String,
}

impl Fixture {
    /// Build a fresh repository against `bin`, with `readme.txt` already present in the worktree so the
    /// first commit has something to author.
    ///
    /// # Panics
    /// If any step of building the repository, generating keys, or trusting the maintainer key fails —
    /// there is no degraded fixture to fall back to; a broken fixture would make every test built on it
    /// meaningless.
    #[must_use]
    pub fn build(bin: &PrikkBin) -> Self {
        let mut root = env::temp_dir();
        root.push(format!(
            "stikk-real-binary-{}-{}-{:?}",
            bin.minor,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let repo = root.join("repo");
        let keys_dir = root.join("keys");
        fs::create_dir_all(&repo).expect("create fixture repo dir");
        fs::create_dir_all(&keys_dir).expect("create fixture keys dir");

        let status = Command::new(&bin.path)
            .arg("init")
            .arg(&repo)
            .status()
            .expect("spawn prikk init");
        assert!(status.success(), "prikk init failed at 0.{}", bin.minor);

        let (author_seed, maintainer_seed) = if bin.minor >= 33 {
            Self::setup_via_prikk_key(bin, &repo, &keys_dir)
        } else {
            Self::setup_manually(bin, &repo, &keys_dir)
        };

        fs::write(repo.join("readme.txt"), "hello\n").expect("seed worktree content");

        Self {
            root,
            repo,
            author_key_id: AUTHOR_KEY_ID.to_string(),
            author_seed,
            maintainer_key_id: MAINTAINER_KEY_ID.to_string(),
            maintainer_seed,
        }
    }

    /// ≥ 0.33: `prikk key generate` derives a keypair and prints the public key, in the same call that
    /// produces the seed. Only the maintainer's public key is used (`trust maintainer add`); AUTHOR
    /// needs no registration at all.
    ///
    /// **Two forms, because `--out` is refused on Windows by design** (found by the first full-matrix
    /// run, 2026-09-12 — see [`Self::generate_key`]). Both yield the same pair; they differ only in
    /// where the seed travels.
    fn setup_via_prikk_key(bin: &PrikkBin, repo: &Path, keys_dir: &Path) -> (String, String) {
        let (author_seed, _) = Self::generate_key(bin, keys_dir, "author");
        let (maintainer_seed, maintainer_pubkey) = Self::generate_key(bin, keys_dir, "maintainer");

        let status = Command::new(&bin.path)
            .args([
                "trust",
                "maintainer",
                "add",
                "--key-id",
                MAINTAINER_KEY_ID,
                "--public-key",
            ])
            .arg(&maintainer_pubkey)
            .current_dir(repo)
            .status()
            .expect("spawn prikk trust maintainer add");
        assert!(
            status.success(),
            "prikk trust maintainer add failed at 0.{}",
            bin.minor
        );

        (author_seed, maintainer_seed)
    }

    /// One `prikk key generate`, returning `(seed_hex, public_key_hex)`.
    ///
    /// **Unix takes `--out`**, which writes the seed to a `0600` file prikk creates itself — prikk's
    /// own preferred handling, and the one this harness used everywhere until the first full-matrix
    /// run. **Windows cannot**: `prikk key generate --out` refuses there *by design*, not by accident —
    /// prikk declines to write a secret at inherited permissions because Unix mode bits have no
    /// portable equivalent, and its own error names the alternative ("run `prikk key generate` without
    /// `--out`, then save the printed seed yourself"). So Windows takes the no-`--out` form and reads
    /// the seed from captured stdout.
    ///
    /// **The seed never reaches a log on either path.** This uses [`Command::output`], which captures
    /// the child's stdout rather than inheriting it, so prikk's own "this seed is now in your terminal
    /// scrollback" warning does not apply: nothing prints it, and the workflow's `--nocapture` cannot
    /// surface what the parent never wrote. The returned `seed_hex` is handled exactly like the
    /// file-read one — into a `Fixture` field, out only through `set_*_env`.
    ///
    /// *(Considered and not taken: using the no-`--out` form on all three platforms, for a single path
    /// exercised everywhere rather than a branch only one platform runs — an unexercised branch is
    /// precisely how this failure survived until the matrix first ran. Kept the split because prikk's
    /// `0600` file is the more careful handling where it is available, and because the full matrix now
    /// exercises both branches every release. Worth a ruling if you disagree.)*
    fn generate_key(bin: &PrikkBin, keys_dir: &Path, label: &str) -> (String, String) {
        let mut command = Command::new(&bin.path);
        command.args(["key", "generate"]);
        let out_path = keys_dir.join(format!("{label}.seed"));
        if cfg!(unix) {
            command.arg("--out").arg(&out_path);
        }
        let out = command
            .output()
            .unwrap_or_else(|e| panic!("spawn prikk key generate ({label}): {e}"));
        assert!(
            out.status.success(),
            "prikk key generate ({label}) failed at 0.{}: {}",
            bin.minor,
            String::from_utf8_lossy(&out.stderr).trim()
        );

        let public_key = Self::extract_public_key_line(&out.stdout);
        let seed = if cfg!(unix) {
            fs::read_to_string(&out_path)
                .unwrap_or_else(|e| panic!("read {label} seed: {e}"))
                .trim()
                .to_string()
        } else {
            Self::extract_seed_line(&out.stdout, label)
        };
        (seed, public_key)
    }

    /// Pull the seed out of `prikk key generate`'s no-`--out` stdout (`seed: <hex>`), for the platforms
    /// where `--out` is refused. **Never logged, never returned in a panic message** — a parse failure
    /// reports only that the line was absent, deliberately without the text it searched.
    fn extract_seed_line(stdout: &[u8], label: &str) -> String {
        let text = String::from_utf8_lossy(stdout);
        text.lines()
            .find_map(|line| line.strip_prefix("seed: "))
            .unwrap_or_else(|| {
                panic!("no `seed: <hex>` line in `prikk key generate` output for {label}")
            })
            .trim()
            .to_string()
    }

    /// Pull the hex key out of `prikk key generate`'s own `public key: <hex>` line. `stdout` is this
    /// command's captured bytes — never a seed; the seed only ever reaches the `--out` file.
    fn extract_public_key_line(stdout: &[u8]) -> String {
        let text = String::from_utf8_lossy(stdout);
        text.lines()
            .find_map(|line| line.strip_prefix("public key: "))
            .unwrap_or_else(|| panic!("no `public key: <hex>` line in `prikk key generate` output"))
            .trim()
            .to_string()
    }

    /// < 0.33: no `prikk key`/`prikk setup`. Generate both seeds with `openssl rand`, derive the
    /// maintainer's public key with `openssl pkey` over the [`ED25519_PKCS8_PREFIX`]-wrapped seed (the
    /// technique this project has used since RFC 014's own manual verification), and trust it — the
    /// caller already ran `prikk init` before this. The AUTHOR key needs no derivation or registration
    /// at all: presence-only, never checked against anything (`stikk-prikk::env`).
    fn setup_manually(bin: &PrikkBin, repo: &Path, keys_dir: &Path) -> (String, String) {
        let author_seed = Self::random_seed_hex();
        let maintainer_seed = Self::random_seed_hex();
        let maintainer_pubkey = Self::derive_pubkey_via_openssl(&maintainer_seed, keys_dir);

        let status = Command::new(&bin.path)
            .args([
                "trust",
                "maintainer",
                "add",
                "--key-id",
                MAINTAINER_KEY_ID,
                "--public-key",
            ])
            .arg(&maintainer_pubkey)
            .current_dir(repo)
            .status()
            .expect("spawn prikk trust maintainer add");
        assert!(
            status.success(),
            "prikk trust maintainer add failed at 0.{}",
            bin.minor
        );

        (author_seed, maintainer_seed)
    }

    /// A fresh random 32-byte Ed25519 seed, hex-encoded, via `openssl rand` — not a new Rust crypto
    /// dependency, matching this workspace's total absence of one: prikk signs; stikk never does,
    /// product or test code alike.
    fn random_seed_hex() -> String {
        let output = Command::new("openssl")
            .args(["rand", "-hex", "32"])
            .output()
            .expect("spawn openssl rand");
        assert!(output.status.success(), "openssl rand failed");
        String::from_utf8(output.stdout)
            .expect("openssl rand produced non-UTF-8 output")
            .trim()
            .to_string()
    }

    /// Derive an Ed25519 public key from a raw seed via `openssl pkey`, for the era before `prikk key
    /// public --seed-env` existed. `seed_hex` is read into memory here and nowhere logged; the DER files
    /// this writes live under `keys_dir`, inside the fixture's own temp root, removed with it.
    fn derive_pubkey_via_openssl(seed_hex: &str, keys_dir: &Path) -> String {
        let seed_bytes = decode_hex(seed_hex);
        assert_eq!(seed_bytes.len(), 32, "an Ed25519 seed must be 32 bytes");
        let mut der = ED25519_PKCS8_PREFIX.to_vec();
        der.extend_from_slice(&seed_bytes);
        let priv_der = keys_dir.join("derived-priv.der");
        let pub_der = keys_dir.join("derived-pub.der");
        fs::write(&priv_der, &der).expect("write private key DER");
        // `prikk key generate --out` writes its own seed file at mode 0600; this file holds the same
        // kind of secret (a raw Ed25519 seed, DER-wrapped) and gets the same restriction on Unix, where
        // `std::fs::Permissions` can express it. No Windows equivalent is set here — a real gap on that
        // platform, worth closing if this path ever runs there for real rather than under the full
        // platform matrix's occasional release-prep check.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&priv_der, fs::Permissions::from_mode(0o600))
                .expect("restrict private key DER permissions");
        }

        let status = Command::new("openssl")
            .args(["pkey", "-inform", "DER", "-in"])
            .arg(&priv_der)
            .args(["-pubout", "-outform", "DER", "-out"])
            .arg(&pub_der)
            .status()
            .expect("spawn openssl pkey");
        assert!(status.success(), "openssl pkey derivation failed");

        let pub_bytes = fs::read(&pub_der).expect("read derived public key");
        assert!(
            pub_bytes.len() >= 32,
            "derived public key DER shorter than expected"
        );
        encode_hex(&pub_bytes[pub_bytes.len() - 32..])
    }

    /// The repository path this fixture built.
    #[must_use]
    pub fn repo(&self) -> &Path {
        &self.repo
    }

    /// Set this process's environment to AUTHOR signing readiness (design `env.rs`: presence of both
    /// `PRIKK_AUTHOR_KEY_ID` and `PRIKK_AUTHOR_SEED`). One of the three call sites `unsafe_code = "deny"`
    /// (this crate's `Cargo.toml`, review C2) allows explicitly — see `lib.rs`'s module doc — and callers
    /// must hold `real_binary::ENV_LOCK` for the duration, since process environment mutation is not
    /// safe across concurrent test threads.
    pub fn set_author_env(&self) {
        // SAFETY: the caller (every call site in `tests/real_binary.rs`) holds `ENV_LOCK` for as long as
        // these values are set, and clears them (`Fixture::clear_env`, at both entry and exit of every
        // test that calls this) before the guard is released. Never printed, never read back through
        // any accessor.
        #[allow(unsafe_code)]
        unsafe {
            env::set_var("PRIKK_AUTHOR_KEY_ID", &self.author_key_id);
            env::set_var("PRIKK_AUTHOR_SEED", &self.author_seed);
        }
    }

    /// Set this process's environment to MAINTAINER signing readiness. See [`Fixture::set_author_env`]
    /// for the safety discipline this holds to.
    pub fn set_maintainer_env(&self) {
        // SAFETY: see `set_author_env`.
        #[allow(unsafe_code)]
        unsafe {
            env::set_var("PRIKK_MAINTAINER_KEY_ID", &self.maintainer_key_id);
            env::set_var("PRIKK_MAINTAINER_SEED", &self.maintainer_seed);
        }
    }

    /// Clear every `PRIKK_*_KEY_ID`/`PRIKK_*_SEED` variable this fixture may have set. Called at both the
    /// start and the end of every test that calls [`Fixture::set_author_env`]/
    /// [`Fixture::set_maintainer_env`], still under the same `ENV_LOCK` guard: at the end so the next
    /// test never inherits this one's leftover readiness, and at the start so a test's own starting state
    /// is deterministic even when the previous holder of the guard panicked before reaching its own exit
    /// call (review C1).
    pub fn clear_env() {
        // SAFETY: see `set_author_env`.
        #[allow(unsafe_code)]
        unsafe {
            env::remove_var("PRIKK_AUTHOR_KEY_ID");
            env::remove_var("PRIKK_AUTHOR_SEED");
            env::remove_var("PRIKK_MAINTAINER_KEY_ID");
            env::remove_var("PRIKK_MAINTAINER_SEED");
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn decode_hex(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("valid hex digit pair"))
        .collect()
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Compare a live capture against a committed fixture, byte-exact — RFC 019 §5's rule: *captured, never
/// rewritten*. On any difference this panics and prints both sides; nothing here can write to the
/// fixture constant living in the caller's own source. A mismatch means prikk's wording changed since
/// the fixture was captured — a human reads this failure and decides whether and how to update the
/// fixture; this function never does.
pub fn assert_matches_fixture(label: &str, fixture: &str, live: &str) {
    assert_eq!(
        fixture, live,
        "\n{label}: live prikk output no longer matches the committed fixture.\n\
         ---- fixture (committed) ----\n{fixture}\n\
         ---- live (just captured) ----\n{live}\n\
         If prikk's wording genuinely changed, a human reads this diff and updates the fixture by hand \
         — this suite never rewrites one itself (RFC 019 §5)."
    );
}
