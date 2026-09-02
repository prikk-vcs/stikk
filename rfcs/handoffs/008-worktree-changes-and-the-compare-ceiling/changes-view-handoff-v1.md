# Handoff — Worktree Changes (v1)

**Companion to:** RFC 008 (Accepted 2026-09-03). Inherits its state.
**Realizes:** ROADMAP "Next" increment 5 — **Changes / worktree-vs-baseline** (`FR-034`, via the
`UD-03` route). **Compare (`FR-033`) is out of this handoff** — deferred behind the content ceiling
(RFC 008; extends UD-09).
**Design items:** `FR-034` (worktree-vs-baseline: changed/missing/untracked/unsupported), `UD-03`
(worktree-status route — now verified fixed at prikk 0.28), `UD-05` (exit-code overloading — the
dirty-exit rule), `UD-06` (whole-worktree honesty), `UD-08` (untracked view filter), `UD-09` (per-file
content diff deferred), `FR-055` (focused ref = the baseline), `TU-04` (view stack), `TU-11`
(long-content discipline), `C-T2a` (inert), `T-T4` (no confident-but-wrong picture); building on the
shell/overlay/inert-text (RFC 001), the view stack (RFC 006), and the presentation mapping (RFC 007).

This is the program design and decision record for the increment. **Implementation, tests, and the
example follow it.** Where this handoff and RFC 008 or the design set disagree, the RFC/design wins and
this handoff is corrected first.

---

## 1. Scope

**In:**
- A **Changes view**: worktree-vs-baseline for the focused ref — the counts and the
  modified / missing / untracked / unsupported paths, read-only.
- One seam method (`worktree_status`) and its confined parser, with the **dirty-exit rule** (parse
  stdout regardless of a non-zero exit — RFC 008 finding 2).
- One operation (`changes_view`), **version-gated at prikk ≥ 0.28**, returning honest guidance below it
  rather than running the pre-fix command (FR-034/UD-03).
- The **UD-08** display-only untracked filter (with the "a commit still captures these" banner) and the
  **UD-06** whole-worktree reminder; the **UD-09** note where a per-file content diff would open.

**Out (do not build here):**
- **Compare (`FR-033`)** — deferred (RFC 008): no honest command exists; a partial Compare would
  mislead (T-T4). The concrete future route (materialize two ref tips to temp dirs) is recorded in the
  RFC, not built.
- **Per-file content diffs** in Changes (`FR-034`'s "per-file diffs") — UD-09; the path-level
  classification is authoritative and shipped, the content diff is named as awaiting prikk support.
- The **`C` commit action** (FR-050) and any mutation — Changes is the read/preview surface the commit
  increment builds on (preview-first).
- The **status-bar worktree marker** (TU-03 clean/dirty) — deferred to the commit increment; the bar
  shows `unknown` for now (avoids an extra version-gated spawn at every open).

---

## 2. The seam grows (`stikk-prikk`)

One method on the `Prikk` trait (category `worktree-analysis` — CT-03). No mutation.

- **`fn worktree_status(&self, repo: &Path, reff: &str) -> Result<WorktreeStatus>`** — run
  `prikk worktree-status <repo> --ref <reff>`.
- **`WorktreeStatus`**: `reff: String`, `clean: bool`, the six counts (`tracked`, `unchanged`,
  `missing`, `modified`, `untracked`, `unsupported`, all `u64`), and `entries: Vec<WorktreeEntry>`.
- **`WorktreeEntry`**: `kind: String` (one of `modified` / `missing` / `untracked` / `unsupported` —
  a String, as `BlockRow.kind` is), `path: String`, `note: String` (prikk's per-line description).

**The dirty-exit rule (RFC 008 finding 2 / UD-05).** `worktree-status` prints the report to **stdout**
and, when the tree differs, an `error: worktree has changes against the baseline` line to **stderr**
with **exit 1**; a clean tree exits **0**. So `worktree_status` must **not** use the ordinary
run-and-classify path. It runs the command capturing stdout+stderr+exit, then:
1. If `parse::worktree_status(&stdout)` succeeds → `Ok(status)` (this covers both `clean` exit-0 and
   `changed` exit-1 uniformly — the parser reads `worktree: clean|changed against baseline`).
2. Else → classify the failure normally (`classify(&stdout, &stderr, WorktreeAnalysis)`) — a genuine
   error (bad ref, not a repo) still surfaces correctly.

**Parsing (SEAM-03 / UD-02):** a new `cli_backend/parse` reader, confined and version-gated, that
**refuses with `StikkError::Environment` on an unrecognized shape** rather than guessing. It reads the
`<label>: <n>` count lines, the `worktree: clean|changed against baseline` headline, and the indented
per-path lines `  <kind> <path> — <note>` (split kind = first token, path = up to ` — `, note = the
rest, so a path with spaces is preserved). A missing headline or a non-numeric count is a refusal.

### Captured parse targets (golden fixtures — real output at prikk 0.28.0)

Clean:
```
worktree-status repository: <path>/.prikk
ref: heads/main
tracked files: 2
unchanged files: 2
missing files: 0
modified files: 0
untracked files: 0
unsupported paths: 0
worktree: clean against baseline
note: use `prikk commit -m <message>` to author node-addressed worktree changes; …
```

Dirty (stdout; the `error:` line is on stderr and is *not* part of the report):
```
worktree-status repository: <path>/.prikk
ref: heads/main
tracked files: 2
unchanged files: 0
missing files: 1
modified files: 1
untracked files: 1
unsupported paths: 0
worktree: changed against baseline
  untracked notes.tmp — worktree file is not in the baseline
  modified readme.txt — tracked file bytes differ from the baseline
  missing src/main.rs — tracked file is absent from the worktree
note: use `prikk commit -m <message>` …
```

Both fixtures ship verbatim in `cli_backend/parse/tests.rs`. `NullBackend` gains
`with_worktree_status` / `with_worktree_status_refusal` and a `with_version(major,minor,patch)` builder
(so the version gate can be tested both ways; the default handshake stays 0.27.1).

---

## 3. The operation (`stikk-core`)

`changes.rs`, mirroring `history.rs`:

- **`ChangesView`**: `reff`, `clean`, the six counts, and `entries: Vec<ChangeEntry>`.
- **`ChangeEntry`**: `kind: ChangeKind` (enum `Modified | Missing | Untracked | Unsupported | Other(String)` — mapped from the seam's string so the view can group and style, `Other` keeping any
  future kind rather than dropping it), `path`, `note`.
- **`fn changes_view(prikk, repo, reff) -> Result<ChangesView>`** — **the version gate lives here**:
  `let hs = prikk.handshake()?;` and if `(hs.version.major, hs.version.minor, hs.version.patch) <
  (0, 28, 0)` return `StikkError::NotReady { detail: "Worktree review needs prikk ≥ 0.28 — this prikk
  is <v>. Before 0.28, `worktree-status` is unreliable (audit UD-03); the rest of stikk works. Update
  prikk to review changes." }` (present() → inline guidance). Otherwise call `prikk.worktree_status`
  and wrap the result. It computes nothing prikk did not report.

---

## 4. The Changes view (`stikk-tui`)

- **`Screen::Changes { view: ChangesView, hide_untracked: bool }`** — a pushed screen (like History),
  reached from Orientation by the `w` key and from the palette ("Open Changes"), back pops (TU-04).
  `Focus::Changes(&ChangesView, bool)` drives the shell body.
- **`view/changes.rs`** renders:
  - a **headline**: `clean against baseline` (dim/ok) or `N change(s) against baseline` (warn),
  - a **counts** line (tracked · unchanged · modified · missing · untracked · unsupported), text-forward
    so colour is never load-bearing (NFR-A03),
  - the **entries**, grouped modified → missing → untracked → unsupported, each `<kind> <path>` with the
    path **inert** (C-T2a) and the note dimmed; the untracked group is hidden when `hide_untracked`,
  - when untracked are hidden, a **UD-08 banner**: "N untracked hidden (display only) — a commit still
    captures them",
  - a **UD-06 footer**: "commits are whole-worktree — there is no staging",
  - a **UD-09 line** where a per-file diff would open: "per-file content diff awaits prikk support
    (UD-09)". No faked diff (T-T4).
  - Long lists scroll within the pane (TU-11), reusing the History scroll approach.
- **Keys:** add `Action::OpenChanges` (`w`) and `Action::ToggleUntracked` (`u`, active only when a
  Changes screen is focused). `Enter` on a Changes row is a no-op this increment (the content diff it
  would open is UD-09-deferred) — the row's UD-09 note says so. Navigation stays context-free dispatch;
  the app resolves per focus.
- **Palette:** register `Command { id: "view.changes", name: "Open Changes", binding: "w",
  min_capability: Viewer, opens: Some(Target::Changes) }`; add `Target::Changes` and wire
  `activate_view(Target::Changes)` → `open_changes(prikk)`.
- **Guidance path:** when `changes_view` returns the `NotReady` (prikk < 0.28), it flows through
  `present()` → inline guidance (a banner), not a broken-command error — FR-034 satisfied.

---

## 5. Security surface (threat model `stikk-03`)

- **C-T2a — inert.** Every worktree path and prikk note goes through `inert` before a cell. Test: a
  path carrying an escape sequence renders inert in the Changes view.
- **T-T4 — no confident-but-wrong picture.** Changes shows only what `worktree-status` authoritatively
  reports (bytes differ / absent / not in baseline); it never infers a content diff it cannot compute,
  and it never claims a file is unchanged that it hasn't verified. The UD-09 note marks the boundary.
- **UD-05 — the dirty exit is not a refusal.** Tested: a scripted dirty status (report present, would
  be exit 1 on the real CLI) yields `Ok(ChangesView{clean:false})`, never a refusal overlay.
- **UD-06/UD-08 — honesty.** The untracked filter is display-only and always says a commit still
  captures the hidden files; the whole-worktree reminder is always present. Test: with `hide_untracked`
  set, the banner text is rendered and the untracked rows are absent.
- **CON-1 — one path to bytes.** stikk reads nothing from the worktree or `.prikk/` directly; the view
  is entirely `worktree-status` output. (This is why the pre-0.28 "replay/plan workaround" is not
  attempted — it would require stikk to read worktree bytes itself.)

---

## 6. Decision notes (program-level; RFC 008 has the rationale)

1. **worktree-status, not a replay/plan reconstruction.** The command is fixed (0.28, verified) and is
   the honest source; the imagined workaround was never feasible without a content surface + direct
   worktree reads (CON-1).
2. **Parse stdout regardless of exit.** The dirty exit-1 is a status, not a refusal (UD-05 corollary);
   the seam method special-cases it before the classifier ever sees it.
3. **Version-gate in the operation layer.** Below 0.28, don't run the command — return guidance. This
   is the only way to honor "never present the broken command's error" (the 0.27.x defect isn't a clean
   post-hoc signal).
4. **Path-level now, content diff deferred (UD-09).** The classification is correct and useful on its
   own; the content diff is named, not faked.
5. **Compare deferred.** A partial Compare misleads (T-T4); the honest minimum needs content. Recorded
   with a concrete route (materialize-to-temp) for when it is picked up.

---

## 7. Test plan

- **Parser (TS-03):** the clean and dirty golden fixtures parse to the right counts, headline, and
  entries; a path with spaces is preserved; a missing headline / non-numeric count **refuses**
  (Environment, UD-02).
- **Seam dirty-exit (TS-02):** `NullBackend` scripts a dirty status → `worktree_status` returns it as
  success; a scripted refusal → surfaces as a refusal. (The real CLI's exit-1-on-dirty is covered by
  the parser test + the CliBackend dirty-exit branch; a `sh`-based `cli_backend/tests.rs` case asserts
  a report on stdout with exit 1 is parsed as success, not classified.)
- **Operation (TS-01):** `changes_view` on a scripted ≥ 0.28 backend returns the view; on a < 0.28
  backend returns `NotReady` with the version guidance (verbatim version in the message).
- **View (TS-01, TestBackend):** clean vs changed headline; grouped entries; the UD-08 banner appears
  only when untracked are hidden and the untracked rows vanish; the UD-06 and UD-09 notes render; a
  hostile path is inert.
- **App/keys:** `w` opens Changes; `u` toggles the untracked filter only on a Changes screen; the
  palette "Open Changes" command opens it; below-0.28 the open surfaces the guidance banner, not a
  broken error.
- **Gates:** `fmt` / `clippy --all-targets --all-features -D warnings` / `test` all green.

---

## 8. Example

`cargo run -p stikk-tui --example changes_demo` (scripted `NullBackend` at version 0.28.1, no prikk,
no repo): open on Orientation; `w` opens Changes on `heads/main` showing a modified, a missing, and a
couple of untracked paths against the baseline; `u` toggles the untracked filter and shows the "a
commit still captures them" banner; the whole-worktree and UD-09 notes are visible. A second scripted
backend at 0.27.1 (a commented alternative in the example) demonstrates the "needs prikk ≥ 0.28"
guidance.

---

## 9. Acceptance criteria

1. `worktree_status` runs `worktree-status`, parses the report from **stdout regardless of exit**, and
   returns clean/dirty uniformly; an unrecognized shape **refuses** (UD-02); a genuine failure (no
   report) classifies normally.
2. `changes_view` is **version-gated at ≥ 0.28**, returning stikk-authored guidance below it instead of
   invoking the pre-fix command (FR-034/UD-03).
3. The Changes view shows the counts and the modified/missing/untracked/unsupported paths, path-level,
   with paths **inert** (C-T2a); no content diff is faked (T-T4/UD-09).
4. The **UD-08** untracked filter is display-only with its always-on "a commit still captures them"
   caveat; the **UD-06** whole-worktree reminder is present.
5. Compare is **not** shipped and its deferral + route are recorded (RFC 008).
6. `fmt` / `clippy -D warnings` / `test` green; `changes_demo` builds and runs against `NullBackend`.

---

## 10. Out of this increment, queued next

- **Compare (`FR-033`)** via the materialize-to-temp route (RFC 008), or a prikk `compare`/content
  surface — whichever lands first.
- **Per-file content diffs** for Changes (and block/patch detail) — UD-09.
- The **commit action** (`FR-050`) building on this Changes preview (the mutation increment: preview →
  message → tier-2 confirmation → `prikk commit`).
- The **status-bar worktree marker** (TU-03), computed once worktree state is loaded in the commit
  increment.

---

## 11. Delivered — 2026-09-03

Built and merged to `main`:

- **Investigation (verified at the live prikk 0.28.0 binary):** `worktree-status` is fixed (UD-03 was
  0.27.x); it computes against the committed/queued baseline (no seal needed); it reports **path-level**
  (modified = bytes differ / missing / untracked / unsupported), with **no per-file content**; and it
  **exits 1 for a dirty tree** (report on stdout, `error:` on stderr). No `diff`/`compare`/`show`;
  `checkout` targets ref tips only. These grounded every decision below.
- **Seam** (`stikk-prikk`): `WorktreeStatus`/`WorktreeEntry`; `Prikk::worktree_status`; a `run_capturing`
  helper + the dirty-exit rule (parse stdout regardless of exit, else classify); a confined parser that
  distinguishes indented per-path entries from flush-left counts and **refuses on an unknown shape**
  (UD-02); golden clean/dirty fixtures captured verbatim from 0.28; `NullBackend::with_version` /
  `with_worktree_status` builders.
- **Operation** (`stikk-core`): `changes_view` with the **≥ 0.28 version gate** (below it, stikk-authored
  guidance via `NotReady`, never the broken command — FR-034/UD-03); `ChangesView`/`ChangeEntry`/
  `ChangeKind` (with `Other` preserving future kinds).
- **TUI** (`stikk-tui`): `Screen::Changes` + `Focus::Changes`; `view/changes.rs` (headline, counts,
  grouped path entries, the UD-08 display-only untracked filter with its "a commit still captures them"
  caveat, and the UD-06 whole-worktree + UD-09 content-diff notes); `w` opens Changes, `u` toggles the
  filter; a palette "Open Changes (worktree)" command + `Target::Changes`; glossary key entries.
- **Example**: `cargo run -p stikk-tui --example changes_demo` (scripted 0.28.1; a commented 0.27.1
  line demonstrates the version guidance).
- **Gates**: `fmt` / `clippy --all-targets --all-features -D warnings` / `test` green — 164 tests.
  Verified live under niri with tiled-window screenshots of the Changes view and its filtered state.

**Compare (FR-033) was deliberately not built** — a partial Compare would mislead (T-T4); it is
recorded deferred with the materialize-to-temp route (RFC 008 §Upstream dependency). Items in §10
remain queued. RFC 008 stays **accepted** (Compare still open).
