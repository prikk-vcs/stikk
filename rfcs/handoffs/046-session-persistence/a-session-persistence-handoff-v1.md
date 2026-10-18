# Handoff A — RFC 046: session persistence (v1)

**Companion to:** [RFC 046](../../done/046-session-persistence.md), **accepted 2026-10-18**, no open question.
**Read the RFC first**, F2 and decisions 1–8. The data model's `DM-04` and `LC-3`/`LC-5`/`LC-6`/`LC-8`
(`docs/src/reference/data-model.md`) are the design it follows.
**Covers:** decisions 1–8. 0.14.0's increment 3.
**Not a release.**

> **What this increment is.** stikk's **first durable write**, and so the first time `C-E2`
> (`ensure_outside_repository`) guards a real file. Everything stored is a convenience that can be deleted at no cost;
> **nothing stored is ever shown before prikk has confirmed it.**
>
> **No branch.** Local gates and the local suite before submitting; after approval, push to `main` and dispatch the
> suite and supply chain there.

---

## 1. Commit 1: the store, in `stikk-state`

A `session` module. `stikk-state` keeps **one dependency edge** (`stikk-model`); no new crate.

- **`SessionRecord`**: format version, canonical repository path, focused ref name (`Option`), last top-level view
  (an enum of F2's four: Orientation, History, Changes, Queue), untracked filter (`bool`). **Names and flags only.**
- **Location**: `<state_dir()>/sessions/<16 lowercase hex>`, the hex being **FNV-1a 64** over the canonical path's bytes
  (`OsStr::as_encoded_bytes`). Write the constant and the algorithm out, with a test pinning two known values, so the
  file name never changes across Rust versions (`DefaultHasher` is not stable: do not use it).
- **`load(state_dir, canonical_root) -> Option<SessionRecord>`**: `None` for a missing file, an unreadable one, a
  version stikk does not know, **any decode failure**, or a stored path that is not this root (a hash collision).
  Never an error, never a panic.
- **`save(state_dir, canonical_root, &record) -> Result<()>`**: **`ensure_outside_repository(target,
  Some(canonical_root))` first**, then create `sessions/` if needed, write a temporary file in `sessions/`, `rename`
  over the target. Then prune to **200** records by modification time, oldest first, never the one just written.
- **The encoding** is yours, with RFC 046 decision 2's two requirements: **every ref name prikk accepts round-trips**
  (a space, non-ASCII, a `=` or newline-looking escape sequence — test hostile ones), and a record that fails to decode
  is ignored **whole**.

**Tests:** the hash's pinned values; round-trips of hostile names; truncated, garbage, future-version and
foreign-path files all `None`; **a `state_dir` inside the repository is refused by `C-E2` and nothing is written** (check
the directory afterwards); the rename leaves no temporary file behind; the bound prunes the oldest; no file holds
anything but the record's fields.

## 2. Commit 2: restore and flush

**a. The decision is a pure function**, so it is testable without a terminal and drivable by the suite:
`restore(stored: Option<&SessionRecord>, refs: &[RefEntry], current: &CurrentBranch) -> Restored { focus, view,
untracked, note }`. The stored focus is used **only if `refs` lists it and it is not closed** (RFC 046 decision 5);
otherwise today's rule in `App::resolve_focus` (`app.rs:2324`) decides, and `note` says the remembered ref is gone.
Put it where both the TUI and the suite can call it (`stikk-core` is the natural home).

**b. In the TUI.** Today focus is resolved once, from the first successful orientation read. **When a record exists,
also read the ref lists before resolving**, then call `restore`. Until both reads are in, show what is shown today
while loading: **nothing stored is drawn early.** Then open the stored view (History of the restored focus, Changes with
the stored filter, or Queue). **The record only ever holds one of the four** (F2): while
Block detail, Patch detail or Compare is open it holds History, and while Verify is open it holds the view beneath it,
so **Verify never reopens**. The `note`, if any, is a banner.

**c. Flush** (decision 4) on a focus change, a top-level view change and the untracked toggle — not on a timer. **A
failed or refused save** shows one banner for the session (*the session will not be remembered*, with the reason) and is
not retried every transition.

**d. The launcher** passes the canonical root (`std::fs::canonicalize` of `handle.root()`) and `state_dir()` into the
TUI. **`STIKK_PRIVATE=1`** (decision 6): no `load`, no `save`, for that launch. The one-shot orientation print reads and
writes nothing (decision 8). `--help`'s notes name `STIKK_READ_ONLY=1` and `STIKK_PRIVATE=1`; read-only still saves.

## 3. Tests and the suite

- **Core/TUI:** `restore` for a valid stored branch, a stored tag, a closed one, a missing one, no record, and a stored
  focus that differs from prikk's current branch (stored wins; the status bar's *prikk's default* shows the difference);
  the TUI draws nothing stored before the reads land; each flush trigger saves; a failed save banners once; private mode
  never calls the store.
- **Suite legs at 0.52**, with `STIKK_STATE_DIR` in a scratch directory and real prikk lists: save a record focusing a
  second branch with Changes and the filter off, `restore` from prikk's real lists, and get all three back; **close that
  branch with prikk** and `restore` falls back with the note; **a `STIKK_STATE_DIR` inside the repository is refused, and
  every file in the repository is byte-identical afterwards** (hash the tree before and after); `STIKK_PRIVATE=1` leaves
  the state directory empty.

## 4. Text

- **The guide**: a short section — what stikk remembers, where (`stikk config path` prints it), that deleting
  `sessions/` is always safe, and `STIKK_PRIVATE=1`.
- `requirements.md`: `FR-122` delivered (focused ref, top-level view, untracked filter); deep restore and anchors not.
- `## Unreleased`: `### Added` per RFC 046's changelog line; `### Breaking` only if `cargo public-api` shows it (a new
  `stikk_tui::run` parameter would be).
- `SECURITY.md` / README design notes: if one says stikk *stores no state*, it is now false — find and fix it.

## 5. The traps

1. **`C-E2` before every write**, with `Some(root)` — not `None`. This is its first real caller; a `None` would test
   nothing.
2. **Nothing stored is shown before prikk confirms it.** A stale branch name on screen for one frame is the bug.
3. **The record holds names and flags, never content, ids or prose.**
4. **A broken record is ignored whole**, never partly applied.
5. **Verify never reopens at launch** (`NFR-P05`).
6. **Paste captures as the file holds them; grep before committing**, including for repeated sections.

## 6. Submit (no branch)

The eight gates and the local suite at 0.28.0 + 0.52.0. Package to
`.git-exclude/review-request/046-session-persistence/review-request-v1.md`, leading with the `C-E2` leg's evidence.
