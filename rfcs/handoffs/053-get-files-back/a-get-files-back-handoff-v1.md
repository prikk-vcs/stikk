# Handoff A — RFC 053: getting files back (v1)

**Companion to:** [RFC 053](../../done/053-get-files-back.md), **accepted 2026-10-29**. **Read the RFC first**,
F1–F6 and decisions 1–8. **Covers:** all of it. 0.17.0's increment 1.
**Not a release.**

> **What this increment is.** From Block detail, **save one file as it was at that block** (`F`) or **all of the
> block's files into a new folder** (`A`), each previewed at Tier Two with no key, **outside the repository and never
> over an existing path**. Nothing in the repository or the worktree changes.
>
> **No branch.** Local gates and the local suite before submitting; after approval, push to `main`, and dispatch the
> suite and supply chain there.

---

## 1. Measure first

Run `.git-exclude/reports/053-get-files-back/measure053.sh <prikk> <empty scratch dir>` at **0.54.1** (and at 0.55.0
if you have it) and diff against `measure053-0.54.txt` / `measure053-0.55.txt`, ignoring ids, scratch paths and the
`tmp-<pid>-<n>` suffix. If any other row differs, stop and report.

## 2. Commit 1: the seam (`stikk-prikk`)

**`FILES_FROM_MINOR = 54`** in `version.rs`, documented beside `EXCHANGE_FROM_MINOR` (`extract` starts at 0.54; one
band for both keys, RFC 053 decision 7). Four methods, **each at a block id, never a ref name**:

| Method | Command | Category | Returns |
|---|---|---|---|
| `file_meta(repo, block, path)` | `cat --path <p> --ref <block> --format json` | `ReadState` | `FileMeta { kind, encoding, mode, size }` from `path-content-v1` |
| `save_file(repo, block, path, output)` | `cat --path <p> --ref <block> --output <output>` | `Exchange` | `()` — prikk prints nothing on success |
| `extract_plan(repo, block, dir)` | `extract --ref <block> <dir> --plan-only --format json` | `ReadState` | `ExtractReport` |
| `extract(repo, block, dir)` | `extract --ref <block> <dir> --format json` | `Exchange` | `ExtractReport` |

`ExtractReport { target_block_id, provisional, written, file_count, total_bytes, largest_file_bytes }` from
`extract-report-v1`. A missing or unknown-typed field is a shape error (`UD-02`).

**Two things this seam has not had to handle before:**

1. **`cat` and `extract` take no repository argument.** The seam runs prikk with the repository as its working
   directory, so **a relative `output`/`dir` would resolve against the repository**, not against where the user
   started stikk. The seam takes **absolute paths only**: assert it (`debug_assert!(path.is_absolute())` and a
   refusal in release), and let core resolve (§3).
2. **`refusal-v1`.** On exit 1, `extract --format json` prints `{"schema_version": "refusal-v1", "command": "extract",
   "refusal": <id>, "reason": <id|null>, "message": <text>}` on **stdout** and the same message on stderr. Recognise it
   **by `schema_version` only**, and return a refusal carrying `message` verbatim. Never parse a `refusal-v1` as an
   `ExtractReport`; never match on its prose. `cat`'s refusals are stderr prose, classified as today.

**Never `--force`, `--allow-full-replay`, `--max-bytes` or `--max-files`.**

**Tests:** argument vectors (one argument per path, a space intact, the path absolute; none of the four flags ever);
`path-content-v1` and `extract-report-v1` parsed from the captures in `measure053-0.54.txt`; a `refusal-v1` document
(`destination-exists`, and 0.55's `destination-inside-repository`) becomes a refusal with the message verbatim; a
truncated report is a shape error. Below the band: unavailable, **nothing spawned** (a recording fake).

## 3. Commit 2: the operations (`stikk-core`)

Two previews and two executes, shaped like `bundle::export_preview` / `export_confirm_and_execute` (RFC 050 B), under
`RequestCategory::Exchange` (Tier Two, no key — RFC 053 decision 2; read-only mode declines them as it declines
export).

**a. Suggested destinations** (decision 4), outside the repository by construction:
- one file: `<repo parent>/<stem>-<short block>.<ext>` — `a.txt` → `a-f687c8ff.txt`; no extension → `<name>-<short>`;
  a leading-dot name (`.env`) has no extension → `.env-<short>`; a path's directories are dropped (`dir/sub/d.txt` →
  `d-<short>.txt`);
- all files: `<repo parent>/<repository directory name>-<short block>`.
Use the same short-id function export uses.

**b. Resolution.** The prompt's text resolves against the directory stikk was started in, as export's does, and the
**resolved absolute path** is what the card shows and what the seam receives.

**c. The preview** (inside `confirm::preview`, so everything is re-read at confirmation):
1. **inside the repository** → blocked, `C-E2`'s check (`ensure_outside_repository` with `Some(repo)`), with stikk's
   reason: *the copy would sit in your worktree as untracked files, and the next commit would capture them. Choose a
   path outside the repository.* **Nothing spawned.**
2. **an existing path** (file or directory) → blocked: *something is already there; stikk does not overwrite it —
   choose another name.* **Nothing spawned.** (prikk's `--plan-only` does not check this — RFC 053 F6.)
3. one file: `file_meta`; all files: `extract_plan`. A refusal → blocked with prikk's text verbatim.
4. otherwise the card.

A **missing parent directory** is not pre-checked: prikk refuses it at execution, and its text is shown verbatim
(F5).

**d. The card** (Tier Two), with `TargetId::named` rows:
- one file — **`block`** short id, **`file`** the repository path (an identifier), **`size`** bytes, **`to`** the
  resolved destination;
- all files — **`block`**, **`files`** and **`bytes`** from the plan, **`to`** the resolved directory.
Consequence, stikk's words, no names (RFC 044 C1): *a copy is written there; nothing in the repository or your
worktree changes.* Notices, each only when it applies:
- **executable** — `meta.mode & 0o111 != 0`, one file only: *prikk saves the file's bytes, not its execute
  permission — the copy will not be executable. Saving all files keeps it.*
- **provisional** — `plan.provisional`, all files only: *no trusted checkpoint is in reach, so prikk replays every file
  from the start of history. The copy will match that history, but nothing has checked the history itself, and it may
  be signed by a key this repository does not trust. Verify (`V`) checks it.*

**e. After success** — no re-read (nothing in the repository changed):
- one file: *Saved `<path>` as it was at `<short block>` to `<destination>`.*
- all files: *Wrote N files (B bytes) to `<directory>`.* from the report, with the provisional notice again if the
  report says so.
A refusal at execution (a race, a missing directory) is prikk's text, verbatim.

## 4. Commit 3: Block detail (`stikk-tui`)

**a. The cursor** ranges over **the block's patches, then its files** (RFC 037's list). `Enter` on a patch opens
Patch detail as today; on a file it does nothing. Scrolling follows the cursor into the file list exactly as it follows
it through the patches; the line count stays exact (the module doc's rule — wrap by hand).

**b. The keys**, advertised in Block detail's hint line and the help/glossary:
- **`F`** — on a file row: the path prompt prefilled with §3a's one-file suggestion. On a patch row: stikk's sentence
  that `F` saves the selected **file**, and the files are listed below.
- **`A`** — anywhere in Block detail: the prompt prefilled with the folder suggestion.
- Where the state files are **unreported** (below `TREE_FROM_MINOR`) or **failed**, and below `FILES_FROM_MINOR`: one
  sentence saying why, **nothing spawned**.

**c. The card and result** through the existing confirmation overlay; no new overlay type unless the existing one
cannot carry the notices.

## 5. Tests and the suite

- **Core:** both blocks (**inside**, **existing**) with **nothing spawned** (a recording fake); the suggestions
  (extension, none, leading dot, nested path); the exec notice on `0o100755` and not on `0o100644`; the provisional
  notice on `provisional: true` only; Tier Two; no key needed; read-only mode declines.
- **TUI:** the cursor crosses from the last patch to the first file and back; `F` on a patch row answers without a
  prompt; `A` from any row; unreported/failed/below-band answer without spawning.
- **Suite legs at the ceiling** (and **unavailable at 0.28**, nothing spawned):
  1. one file saved; its bytes equal `prikk cat --path … --ref <block>`'s;
  2. all files extracted; the set of paths and sizes equals `tree --ref <block> --format json`'s;
  3. **a destination inside the repository declined, no prikk write spawned, and `worktree-status` unchanged**
     (compare its report before and after) — for `F` and for `A`;
  4. an existing destination declined, and its bytes unchanged;
  5. an executable file's card carries the notice;
  6. a received branch's plan carries the provisional notice (import a bundle from a second repository first, as RFC
     050's legs do).
- **Captures at 80 columns:** Block detail with the cursor on a file; both cards (one with each notice); the inside
  refusal; both results.

## 6. Text

- `requirements.md`: `FR-053` — its non-destructive half delivered at prikk ≥ 0.54 (save a file, save all files,
  outside the repository); materializing into the worktree not.
- The guide: a short **Getting an earlier version back** section — open a block in History, `F` on a file or `A` for
  all, where the copy goes, that nothing in the repository changes, and that to use it you copy what you want into
  your worktree and commit.
- `CHANGELOG.md` `## Unreleased`: `### Added`, and the `### Breaking` row for the trait's four methods.

## 7. The traps

1. **Absolute paths only into `cat`/`extract`** — prikk resolves a relative one against the repository.
2. **A block id, never a ref name** — a ref can move between preview and confirm.
3. **Never `--force`, `--allow-full-replay` or `--max-*`.**
4. **`refusal-v1` by `schema_version`**, and its `message` verbatim; no prose matching.
5. **Stikk's two blocks spawn nothing** — inside the repository and an existing path.
6. **Nothing is re-read after success**, and nothing in the repository may change: the suite proves it with
   `worktree-status`.
7. **Paste captures as the file holds them; grep before committing.**

## 8. Submit (no branch)

The eight gates and the local suite at 0.28.0 and the current ceiling. Package to
`.git-exclude/review-request/053-get-files-back-a/review-request-v1.md`, leading with §1's measurement diff and suite
leg 3.
