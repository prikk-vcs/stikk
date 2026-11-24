# Handoff B — RFC 067: the move into `stikk-app` (v1)

**Companion to:** [RFC 067](../../accepted/067-stikk-app-the-shared-program.md), **accepted 2026-11-24**. **Read the
RFC first**: decisions 1, 2, 6, 7 and 8. **Follows** [Handoff A](./a-the-seams-handoff-v1.md), delivered at
`43e2408`. **Covers:** decisions 1, 2, 6 and 7. Decision 8 carries on: the golden frames stay unchanged. **0.22.0's
increment 2, second half.** **Not a release.**

> **What this increment is.** Handoff A cut the seams, so the program no longer names the terminal. This handoff
> **moves** the program into a new crate, `crates/stikk-app`, that cannot depend on a terminal library, and leaves
> `stikk-tui` as the terminal front end. **A move is `git mv` plus imports and visibility. No logic changes.** A
> diff hunk that is neither of those is a stop-and-report.
>
> **The golden manifest stays byte for byte as it is** (`9647770c…`). **No branch.**
>
> **Sanctioned:**
> - `crates/stikk-app/**` (new) and `crates/stikk-tui/**`;
> - the workspace `Cargo.toml`, `Cargo.lock`'s workspace entries, and `deny.toml`;
> - `.github/workflows/release.yml`'s two crate lists;
> - `crates/stikk-real-binary/tests/*.rs`, the guards only (§4);
> - `crates/stikk/src/main.rs`'s imports;
> - the text named in §6.
>
> **`stikk-core` does not change.**

---

## 0. Evidence plan

| Question | Level | Evidence in the request |
|---|---|---|
| Does anything a user sees change? | headless | `git diff 43e2408..HEAD -- crates/stikk-tui/src/golden/frames.txt` is empty (the manifest stays in `stikk-tui`) |
| Is the move only a move? | review | `git diff -M --stat` shows each moved file as a rename with its similarity; the residue lists every non-import hunk, each justified |
| Is `stikk-app` free of the terminal? | supply chain | `cargo deny check bans`: green, then refusing a planted `ratatui` and a planted `crossterm` in `stikk-app`. Both are shown, then reverted |
| Do the guards still cover the program's text? | real-binary tests | each guard's scanned-file count before and after (§4), which must not fall |
| Is no test lost? | `cargo test` | the per-crate counts before and after: the workspace total must not fall |
| Does stikk work with prikk? | real binaries | the local suite at 0.28.0 + 0.58.0, then the full matrix after the push |
| Can the crates be packaged? | gate 6 | `cargo +stable package --workspace --exclude stikk-real-binary`, with `stikk-app` not yet on crates.io |
| Is the release build whole? | CI | `release-trial.yml`, which the push starts, since it touches `release.yml` |

**All eight gates are run, none exempted.** This handoff moves doc comments across crates, and gate 5 checks every
intra-doc link.

## 1. Commit 1: the crate, and what moves into it

1. **`crates/stikk-app/Cargo.toml`:**
   - `version`, `edition`, `rust-version`, `license` and `authors` from the workspace;
   - a `description` in the house style: what it is, and that front ends depend on it;
   - `[lints] workspace = true`;
   - dependencies: `stikk-model`, `stikk-prikk`, `stikk-state` and `stikk-core`, **and nothing else**. A
     dev-dependency the moved tests need is fine; say which.
2. **The workspace `Cargo.toml`:** add `stikk-app = { version = "0.21.1", path = "crates/stikk-app" }` to
   `[workspace.dependencies]`, beside the others. `members = ["crates/*"]` picks the crate up.
3. **Move with `git mv`:**
   - `app.rs` and `app/` (but see §2 for its tests);
   - `worker.rs` and `worker/`;
   - from `overlay.rs`: the state part, `SavePathKind`, `Overlay`, `RefNameKind`, `RefWriteKind` and their `impl`s,
     into `stikk-app/src/overlay.rs`. Use `git mv` of a copy, or a split in which the moved lines stay recognizable
     to `-M`. **Drawing code stays in `stikk-tui`'s `overlay.rs`;**
   - **`Intent`, `Effect`, `Direction`, `Surface` and `Reading`,** wherever Handoff A put them.

   **`keys::dispatch` and `handle_key` stay in `stikk-tui`.** So do `viewports`, `shell`, the views, `theme`, `text`,
   `title`, `status_bar`, `key_line`, `terminal`, `readme_visuals`, `golden` and `lib.rs`'s run loop.
4. **One path per item.**
   - **`stikk-app` exposes its public items at the crate root** (`stikk_app::App`, `stikk_app::Overlay`,
     `stikk_app::Intent` and the rest), with its modules private. There is then one name for each thing, where
     `stikk-tui` today exposes many twice (Handoff A's `cargo public-api` showed `stikk_tui::app::X` and the
     re-export).
   - **`stikk-tui` does not re-export `stikk-app`.** A front end, and the launcher, name `stikk_app::` directly.
5. **Visibility.** Items that were `pub(crate)` and that `stikk-tui` now needs must become `pub`.
   - **List every one in the request**, with the drawing code that needs it.
   - **Prefer moving a helper with its only caller** over widening it. A `pub` item used only by tests is a
     stop-and-report: put the test where the item is.
6. **Where `Session` belongs.** `stikk_tui::Session` (`lib.rs:80`) is what the launcher builds and hands to `run`. If
   `App` or the worker reads it, it moves. If only the run loop does, it stays. Say which, and why.

## 2. Commit 1, continued: the tests go with what they test

- **A test that draws** (`shell::render`, a `TestBackend`, `test_util`) **or presses a key** (`handle_key`)
  **stays in `stikk-tui`.** That includes Handoff A's F1 and F2 tests and the RFC 060 B key-press tests. They exercise
  the front end and the program together, and that is their point.
- **Every other test of `App`, the worker or `Overlay` moves to `stikk-app`,** unchanged except for imports. Handoff
  A's three `report_reading` tests and the no-report pin are among them.
- **Give the per-crate test counts** before (`43e2408`: 1622 in all) and after. The workspace total must not fall.
  `stikk-tui`'s count falls by exactly what `stikk-app`'s rises by, plus any test you split (say which).

## 3. Commit 2: `stikk-app` cannot grow a terminal dependency

**`deny.toml`'s `[bans]`**, exactly as RFC 067 decision 6 measured:

```toml
deny = [
  { crate = "ratatui", wrappers = ["stikk-tui"] },
  { crate = "terminal-colorsaurus", wrappers = ["stikk-tui"] },
  { crate = "crossterm", wrappers = ["ratatui-crossterm"] },
]
```

**Run `cargo deny check bans`** three times:
1. green as committed;
2. with `ratatui = { workspace = true }` planted in `stikk-app`: refused;
3. with `crossterm = "0.29"` planted in `stikk-app`: refused.

Paste the three outputs' last lines, and revert the plants. **Nothing planted is committed.**

## 4. Commit 3: the guards cover the program wherever it lives

**Three guards find their files by a hand-written list, and the move would silently shrink two of them:**

| Guard | How it finds files today | What the move does to it |
|---|---|---|
| `key_literals.rs:107` | `["crates/stikk-core/src", "crates/stikk-tui/src"]` | **`app.rs` leaves its scan** |
| `emphasis_guard.rs:74` | `crates/stikk-tui/src` | **`app.rs` and `overlay`'s state leave its scan** |
| `seed_guard.rs:33` | `PUBLISHED`, six names | `stikk-app` is not in it |
| `literal_guard.rs:53` | the whole of `crates/` | unaffected |

1. **Replace each list with the published crates, derived from the workspace:** every `crates/*/Cargo.toml` that does
   not say `publish = false`, read by the test. Put it in one helper, in `stikk-real-binary`'s `support`, or wherever
   the guards share code, and have all three call it.
   - **A crate added later is then guarded the day it exists, without an edit.**
   - If `key_literals` or `emphasis_guard` then fails on a crate it never scanned before, **stop and report.** Do not
     widen an allow-list on your own.
2. **The two allow-lists that name `stikk-tui` files** (`emphasis_guard`'s `ALLOWED`, `literal_guard`'s
   `readme_visuals.rs` and `seed_guard`'s `orientation.rs`) name files that do not move. Check each one still exists.
3. **Give each guard's scanned-file count** before and after. It must not fall.
4. **`release.yml`'s two `for crate in …` lists get `stikk-app` between `stikk-core` and `stikk-tui`,** and a new
   test, in `stikk-real-binary`, holds them to the workspace. It reads `release.yml` and asserts, for **both** lines:
   - **the set** equals the published crates (the same helper);
   - **the order** is a dependency order: each crate's `stikk-*` dependencies, read from its `Cargo.toml`, come
     earlier in the line.

   So the next crate cannot be forgotten in the release, and a wrong order fails on a push rather than at a tag.

## 5. The traps

1. **A move moves.** If `git diff -M` shows a moved file at under 90 % similarity, say why. The usual reason is a
   `use` block or a visibility change.
2. **The manifest is not rewritten.** If a frame changes, the move changed behaviour; find it.
3. **No `stikk_tui::` path to a `stikk-app` item.** `grep -rn "pub use stikk_app" crates/stikk-tui` is empty.
4. **No terminal type in `stikk-app`.** `grep -rn "ratatui\|crossterm\|Cell<" crates/stikk-app/src` is empty, and
   `cargo tree -p stikk-app -e normal` shows no terminal crate. Paste both.
5. **The launcher's behaviour does not change.** `main.rs` changes its imports only.

## 6. Commit 4: the text

- **`CHANGELOG.md`, `## Unreleased`, `### Changed`:** RFC 067's line.
- **`docs/src/reference/internal-design.md`:** AR-04 and the crate list at `:86`. The crates are now **seven**:
  model, prikk, state, core, **app**, tui and the launcher. Give one sentence on what `stikk-app` owns (the program)
  and what a front end owns (drawing, input events, scroll positions), and name RFC 067.
- **`docs/src/contributing/releasing.md`:** where it says *"each of the six crates"* (`:52`) and lists the first
  publish, make it seven. Add that `stikk-app` is published by hand once at 0.22.0. The architect writes the exact
  steps into the release-prep handoff, so do not write the steps here.
- **No `README.md` for `stikk-app`.** None of the library crates has one (only `crates/stikk/` does).

## 7. Submit (no branch)

**Run:**
- **all eight gates;**
- the golden module;
- the local suite at 0.28.0 + 0.58.0;
- `cargo public-api -p stikk-tui diff 43e2408..HEAD`;
- `cargo public-api -p stikk-app` (new: list it).

**Package it to** `.git-exclude/review-request/067-stikk-app-the-shared-program-b/review-request-v1.md`, with:
- §1.5's visibility list;
- §1.6's answer;
- §2's counts;
- §3's three outputs;
- §4's counts;
- §5's `grep`s and `cargo tree`.

**After approval:** push, then report CI, Docs, supply chain, `release-trial.yml` (the push starts it), and the
full-matrix suite (dispatch it), all at the pushed SHA.
