//! Harness machinery for the real-binary integration suite. Not test functions itself — see
//! `tests/real_binary.rs` for what actually runs.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

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

    /// Resolve one end of the matrix from its environment variable, confirming the binary really is the
    /// version this suite's matrix names.
    ///
    /// **Every failure here is a `HARNESS FAILURE`, not a product one** (RFC 022 §5). Resolution runs
    /// before any test touches stikk, once per end per test, so a missing binary or a wrong version
    /// produces one identical panic per test — the exact shape RFC 021's Windows break had, and the one
    /// the architect hit again reviewing 0.5.0 when their scratch binaries were cleaned between
    /// sessions. Routing it through [`harness_fail`] means the first failure explains itself and the
    /// rest say they are its shadow.
    fn resolve(env_var: &str, expected_minor: u32) -> Self {
        let Some(path) = env::var_os(env_var) else {
            harness_fail(&format!(
                "{env_var} is not set.\n\n\
                 This suite needs two real prikk binaries. Install one for this end of the range:\n\n\
                 \tcargo install prikk --version 0.{expected_minor}.0 --locked --root <dir>\n\n\
                 then set {env_var}=<dir>/bin/prikk before running (see this crate's `lib.rs` and \
                 `tests/real_binary.rs` module docs for the other one and the full invocation)."
            ));
        };
        let path = PathBuf::from(path);
        let backend = stikk_prikk::CliBackend::with_program(&path);
        let handshake = match stikk_prikk::Prikk::handshake(&backend) {
            Ok(handshake) => handshake,
            Err(e) => harness_fail(&format!(
                "{env_var}={path:?} did not answer `prikk --version`: {e}\n\
                 (a missing or non-executable binary at that path is the usual cause — scratch \
                 installs do not survive between sessions)"
            )),
        };
        if handshake.version.minor != expected_minor {
            harness_fail(&format!(
                "{env_var}={path:?} reports prikk {}, but this suite's version matrix (derived from \
                 stikk_prikk::version::supported_minor_range, not written here) expects minor \
                 {expected_minor} at this end of the range. Install the version that range actually \
                 names — do not edit this assertion to match whatever happens to be installed.",
                handshake.version
            ));
        }
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
    /// The prikk minor this fixture was built against — the harness configures prikk differently on
    /// either side of 0.40 (see [`Fixture::set_author_env`]).
    minor: u32,
    author_key_id: String,
    author_seed: String,
    /// A `0600` file holding the AUTHOR seed, for the `PRIKK_AUTHOR_SEED_FILE` override at ≥ 0.40.
    author_seed_file: PathBuf,
    maintainer_key_id: String,
    maintainer_seed: String,
    /// As [`Fixture::author_seed_file`], for MAINTAINER.
    maintainer_seed_file: PathBuf,
}

/// Whether fixture construction has already failed once in this process (RFC 022 F3/§5).
static HARNESS_ALREADY_FAILED: AtomicBool = AtomicBool::new(false);

/// Fail the current test as a **harness** failure — the repository could not be built — rather than as
/// a product failure, and say so on the first line.
///
/// RFC 021's Windows break produced **five identical panics** about `prikk key generate`, because every
/// test builds its own fixture first. One broken setup step, five red tests, and nothing in the output
/// distinguishing *"the harness could not construct a repository"* from *"stikk got an answer wrong"* —
/// on the gate whose only job is telling a human whether to ship. At ten surfaces that is a misleading
/// picture of a release's health.
///
/// Two things make it readable. Every message starts with `HARNESS FAILURE`, so one line answers which
/// kind it is; and only the **first** carries the detail — every later one says the harness is already
/// known broken and that this test never reached stikk at all, so a reader looks at one wall of text
/// rather than N identical ones.
fn harness_fail(what: &str) -> ! {
    if HARNESS_ALREADY_FAILED.swap(true, Ordering::SeqCst) {
        panic!(
            "HARNESS FAILURE (already reported above): fixture construction is broken, so this test \
             never ran stikk at all. Fix the first HARNESS FAILURE in this run; the others are its \
             shadow, not independent findings."
        );
    }
    panic!(
        "HARNESS FAILURE: {what}\n\
         \n\
         This is the test harness failing to build a prikk repository — **not** stikk getting an \
         answer wrong. Nothing below this line exercised stikk. Every other test in this run builds \
         its own fixture the same way and will report the same cause in one line."
    );
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
        for dir in [&repo, &keys_dir] {
            if let Err(e) = fs::create_dir_all(dir) {
                harness_fail(&format!("could not create {}: {e}", dir.display()));
            }
        }

        match Command::new(&bin.path).arg("init").arg(&repo).status() {
            Ok(status) if status.success() => {}
            Ok(status) => harness_fail(&format!("`prikk init` at 0.{} exited {status}", bin.minor)),
            Err(e) => harness_fail(&format!(
                "could not spawn `prikk init` at 0.{}: {e}",
                bin.minor
            )),
        }

        let (author_seed, maintainer_seed) = if bin.minor >= 33 {
            Self::setup_via_prikk_key(bin, &repo, &keys_dir)
        } else {
            Self::setup_manually(bin, &repo, &keys_dir)
        };

        // RFC 026 §2: at ≥ 0.40 prikk no longer reads `PRIKK_*_SEED`, so the harness configures it with
        // `PRIKK_*_SEED_FILE` instead — which needs a file to point at on **every** platform, including
        // the Windows path where `key generate --out` is refused and the seed arrives on stdout. Written
        // here rather than in `generate_key` so the manual pre-0.33 path gets one too and the two eras
        // differ in one place only.
        let author_seed_file = Self::write_seed_file(&keys_dir, "author", &author_seed);
        let maintainer_seed_file = Self::write_seed_file(&keys_dir, "maintainer", &maintainer_seed);

        if let Err(e) = fs::write(repo.join("readme.txt"), "hello\n") {
            harness_fail(&format!("could not seed worktree content: {e}"));
        }

        Self {
            root,
            repo,
            minor: bin.minor,
            author_key_id: AUTHOR_KEY_ID.to_string(),
            author_seed,
            author_seed_file,
            maintainer_key_id: MAINTAINER_KEY_ID.to_string(),
            maintainer_seed,
            maintainer_seed_file,
        }
    }

    /// Write `seed_hex` to `<keys_dir>/<label>.seedfile`, `0600` on Unix, and return the path.
    ///
    /// Distinct from the file `key generate --out` may already have written: that one is prikk's, exists
    /// only on Unix at ≥ 0.33, and this harness must have one path that exists on every platform and in
    /// every era. Overwriting prikk's would be the same file with two owners.
    ///
    /// **The seed is written and never read back by this type.** It leaves only as a path in
    /// `PRIKK_*_SEED_FILE`, which is prikk reading its own secret from disk — the same posture as the
    /// `--out` file, and inside the `C-I1e` boundary for the same reason.
    fn write_seed_file(keys_dir: &Path, label: &str, seed_hex: &str) -> PathBuf {
        let path = keys_dir.join(format!("{label}.seedfile"));
        if let Err(e) = fs::write(&path, format!("{seed_hex}\n")) {
            harness_fail(&format!("could not write the {label} seed file: {e}"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) = fs::set_permissions(&path, fs::Permissions::from_mode(0o600)) {
                harness_fail(&format!("could not chmod the {label} seed file: {e}"));
            }
        }
        path
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
            .status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => harness_fail(&format!(
                "`prikk trust maintainer add` at 0.{} exited {s}",
                bin.minor
            )),
            Err(e) => harness_fail(&format!(
                "could not spawn `prikk trust maintainer add` at 0.{}: {e}",
                bin.minor
            )),
        }

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
        let out = match command.output() {
            Ok(out) => out,
            Err(e) => harness_fail(&format!(
                "could not spawn `prikk key generate` ({label}): {e}"
            )),
        };
        if !out.status.success() {
            harness_fail(&format!(
                "`prikk key generate` ({label}) failed at 0.{}: {}",
                bin.minor,
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }

        let public_key = Self::extract_public_key_line(&out.stdout);
        let seed = if cfg!(unix) {
            match fs::read_to_string(&out_path) {
                Ok(seed) => seed.trim().to_string(),
                Err(e) => harness_fail(&format!("could not read the {label} seed file: {e}")),
            }
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
            .status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => harness_fail(&format!(
                "`prikk trust maintainer add` at 0.{} exited {s}",
                bin.minor
            )),
            Err(e) => harness_fail(&format!(
                "could not spawn `prikk trust maintainer add` at 0.{}: {e}",
                bin.minor
            )),
        }

        (author_seed, maintainer_seed)
    }

    /// A fresh random 32-byte Ed25519 seed, hex-encoded, via `openssl rand` — not a new Rust crypto
    /// dependency, matching this workspace's total absence of one: prikk signs; stikk never does,
    /// product or test code alike.
    fn random_seed_hex() -> String {
        let output = Command::new("openssl")
            .args(["rand", "-hex", "32"])
            .output()
            .unwrap_or_else(|e| harness_fail(&format!("could not spawn `openssl rand`: {e}")));
        if !output.status.success() {
            harness_fail(&format!(
                "`openssl rand` failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
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
        if let Err(e) = fs::write(&priv_der, &der) {
            harness_fail(&format!("could not write the private key DER: {e}"));
        }
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
            .unwrap_or_else(|e| harness_fail(&format!("could not spawn `openssl pkey`: {e}")));
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

    /// The AUTHOR key id this fixture configures — so a test can assert that
    /// [`stikk_prikk::key_id`] reports **this** value rather than a string the test also wrote.
    ///
    /// The id is not a secret and never was: `C-I1` is presence-only for **seeds**, and prikk prints
    /// ids itself. There is deliberately no accessor of this shape for either seed, and there must
    /// never be one.
    #[must_use]
    pub fn author_key_id(&self) -> &str {
        &self.author_key_id
    }

    /// The MAINTAINER key id this fixture configures. See [`Fixture::author_key_id`].
    #[must_use]
    pub fn maintainer_key_id(&self) -> &str {
        &self.maintainer_key_id
    }

    /// The first prikk minor that stopped reading `PRIKK_*_SEED` (RFC 026 F1). At **0.40** setting one
    /// is a refusal; at **0.41** it is ignored in favour of the key directory. Either way the harness
    /// stops configuring the binary it is testing, which is why this had to be rebuilt before anything
    /// else in this re-baseline could be measured.
    const KEY_DIRECTORY_ERA: u32 = 40;

    /// Set this process's environment to AUTHOR signing readiness.
    ///
    /// **Two eras, chosen by version** — the same shape [`Fixture::build`] already uses to pick between
    /// `prikk key generate` and manual derivation at 0.33:
    ///
    /// | prikk | variable | why |
    /// |---|---|---|
    /// | ≤ 0.39 | `PRIKK_AUTHOR_SEED` | the only mechanism; `_SEED_FILE` does not exist at the 0.28 floor |
    /// | ≥ 0.40 | `PRIKK_AUTHOR_SEED_FILE` | `_SEED` is refused (0.40) or ignored (0.41) |
    ///
    /// **The override, not a key directory.** `XDG_CONFIG_HOME` would give prikk a key directory and
    /// work — but it is *shared* state, and every test here owns its own repository **and its own
    /// keys**, which is the per-test isolation RFC 022 measured at ~0.3s a fixture and kept on purpose.
    /// The override keeps one seed pair per fixture with nothing in common between them.
    ///
    /// One of the three call sites `unsafe_code = "deny"` (this crate's `Cargo.toml`, review C2) allows
    /// explicitly — see `lib.rs`'s module doc — and callers must hold `real_binary::ENV_LOCK` for the
    /// duration, since process environment mutation is not safe across concurrent test threads.
    pub fn set_author_env(&self) {
        // SAFETY: the caller (every call site in `tests/real_binary.rs`) holds `ENV_LOCK` for as long as
        // these values are set, and clears them (`Fixture::clear_env`, at both entry and exit of every
        // test that calls this) before the guard is released. Never printed, never read back through
        // any accessor.
        #[allow(unsafe_code)]
        unsafe {
            env::set_var("PRIKK_AUTHOR_KEY_ID", &self.author_key_id);
            if self.minor >= Self::KEY_DIRECTORY_ERA {
                env::set_var("PRIKK_AUTHOR_SEED_FILE", &self.author_seed_file);
            } else {
                env::set_var("PRIKK_AUTHOR_SEED", &self.author_seed);
            }
        }
    }

    /// Set this process's environment to MAINTAINER signing readiness. See [`Fixture::set_author_env`]
    /// for the two eras and the safety discipline this holds to.
    pub fn set_maintainer_env(&self) {
        // SAFETY: see `set_author_env`.
        #[allow(unsafe_code)]
        unsafe {
            env::set_var("PRIKK_MAINTAINER_KEY_ID", &self.maintainer_key_id);
            if self.minor >= Self::KEY_DIRECTORY_ERA {
                env::set_var("PRIKK_MAINTAINER_SEED_FILE", &self.maintainer_seed_file);
            } else {
                env::set_var("PRIKK_MAINTAINER_SEED", &self.maintainer_seed);
            }
        }
    }

    /// Clear every `PRIKK_*_KEY_ID`/`PRIKK_*_SEED`/`PRIKK_*_SEED_FILE` variable this fixture may have
    /// set — **all six regardless of era**, so a fixture built against one end of the range can never
    /// leave a variable behind that configures the other. Called at both the
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
            env::remove_var("PRIKK_AUTHOR_SEED_FILE");
            env::remove_var("PRIKK_MAINTAINER_KEY_ID");
            env::remove_var("PRIKK_MAINTAINER_SEED");
            env::remove_var("PRIKK_MAINTAINER_SEED_FILE");
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
