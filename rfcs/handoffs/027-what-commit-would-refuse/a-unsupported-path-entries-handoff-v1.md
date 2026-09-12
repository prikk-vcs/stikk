# Handoff A — the unsupported paths stikk has never listed (v1)

**Companion to:** [RFC 027](../../accepted/027-what-commit-would-refuse.md) (Accepted 2026-09-13, **Q1
ruled (b)**) — **F0 only**.
**Handoff B** — decisions 2–7: the JSON reader, the authoring verdict, refused entries in the Changes
view, commit's prevention, and the 0.7.0 bump — follows once this lands, and is built on the reader you
fix here.
**Design items:** `FR-034`, `ER-02`, `UD-02`, `ASM-2`.

> **Every stikk release since 0.1.0 has counted unsupported paths in the Changes header and listed none
> of them.** This is small, it is not breaking, and it is in every release — so it goes first.

---

## 1. Scope

**In:** the prose reader keeps every entry; `unsupported-path` maps to `ChangeKind::Unsupported`; two
documented guarantees become true; fixtures captured at both ends; an invariant test over every
fixture; a real-binary test; the changelog.

**Out — all Handoff B's:** `--format json`, the authoring verdict, marking refused entries, commit's
prevention, any change to a public struct or enum, and the version bump. **A must not be breaking.** If
the fix turns out to need a public shape change, stop and report it.

## 2. The defect, measured

prikk names an unrepresentable path `unsupported-path` — `UnsupportedPath => "unsupported-path"` at every
tag from 0.28.0 to 0.41.0 — and prints it that way at both ends of the supported range:

```
unsupported paths: 1
worktree: changed against baseline
  unsupported-path /tmp/…/back\slash.txt — worktree path is not representable as a safe Prikk path: invalid name: backslashes are not allowed in repository paths
```

stikk looks for `"unsupported"`:

- `WORKTREE_KINDS` at `crates/stikk-prikk/src/cli_backend/parse.rs:35`;
- `parse_worktree_entry` returns `None` for any first word outside that list;
- `ChangeKind::from_label` in `crates/stikk-core/src/changes.rs` maps `"unsupported"`.

**The count is kept and the entry is dropped.** And two documented guarantees have never been reachable:
`WorktreeEntry::kind` is *"kept as text so a future kind renders rather than breaks parsing"*, and
`ChangeKind::Other` *"preserves any future kind rather than dropping it"* — but the parser discards an
unrecognized kind before either sees it. The test at `changes/tests.rs:91` calls `from_label` directly,
which the parser never does with an unknown word.

## 3. The fix

### The rule, measured at four tags

Read `print_worktree_status` in `crates/prikk-cli/src/output/worktree.rs` at `0.28.0`, `0.38.0`,
`0.39.0` and `0.41.0`. After `worktree: changed against baseline`, prikk prints **exactly one indented
line per change** — `"  {} {} — {}"` at 0.28 and 0.38, and `"  {} {} — {}{}"` at 0.39 and 0.41, the
last `{}` being the refused suffix. Everything after the loop is unindented: `live rename declarations:
N` and the `note:` lines. The declarations' own indented lines sit under that unindented header, so the
region RFC 021 F0 already scopes stops before them.

**So every indented line in that scoped region is an entry, and its kind is its first word — whatever
the word is.** No closed list. `parse_worktree_entry`'s justification for returning `None` — *"e.g. a
wrapped note"* — describes a line prikk has not printed at any of those four tags. Remove it with the
list. If you believe some indented line still needs to be skipped, name the line prikk prints.

### Keep

- **RFC 021 F0's scoping, and its tests, unchanged and green**: the suite's
  `f0_a_renamed_kind_word_path_is_not_a_fabricated_entry` and the captured declaration fixtures in
  `parse/tests.rs`. Widening what counts as an entry is precisely the change that could reopen F0; those
  tests are what stop it.
- The ` — ` split between path and note, and today's handling of a line without one.

### Change

- **`ChangeKind::from_label` maps `"unsupported-path"` to `Unsupported`, and the `"unsupported"` arm
  goes.** prikk has never printed that word, and a mapping for a word nobody prints is how this defect
  hid. An `"unsupported"` would still render, as `Other`.
- **Both documented guarantees are true afterwards.** Keep their wording only if it now is.

### The refused suffix

At 0.39 and above, a refused entry's note ends `[refused: …]`. **Leave it in the note.** A does not parse
it; B reads the verdict from JSON instead.

## 4. Tests

1. **Fixtures captured verbatim** from `prikk worktree-status` at **0.28.0** and **0.41.0**: one ordinary
   modification, one backslash-named file, one non-UTF-8-named file, with a provenance comment in the
   file's own style. **Run the probe from a neutral directory such as `/tmp/repo`.** Unsupported entries
   carry an absolute path, and a fixture must neither contain anyone's home directory nor be edited after
   capture to remove it.

   **Measured, so you do not chase it:** for a non-UTF-8 name, prikk itself prints `U+FFFD` (bytes
   `EF BF BD`) at 0.28 and 0.41, in prose and JSON. The output is valid UTF-8, a `&str` fixture holds it,
   and the seam decodes stdout with `from_utf8_lossy` regardless.

2. **The invariant that would have caught this.** For every `worktree-status` fixture in `parse/tests.rs`,
   each kind's parsed entry count equals prikk's own counter for it — `modified files:`,
   `missing files:`, `untracked files:`, `unsupported paths:`. prikk computes those counters from the same
   list it prints (`count_kind` over `report.changes`), so equality is the contract. One helper, applied
   to **every** fixture, the old ones included — that is where it proves it holds on shapes that already
   worked.

3. **An unknown kind, through the parser.** An indented entry with an invented first word parses,
   survives `from_status`, and arrives as `ChangeKind::Other(word)`. Keep the direct `from_label` test
   beside it.

4. **The Changes view at 80 columns**, a `TestBackend` capture of the 0.41 fixture. Report what a long
   absolute path does to the row. **It is in scope only if the entry becomes unreadable** — its path
   clipped before it can be told apart. Otherwise report it and leave the layout alone.

5. **Real binaries, at both ends.** A backslash-named file yields exactly one entry of kind
   `unsupported-path`, and that count agrees with `unsupported paths:`. **Announce a skip on Windows**,
   where a backslash is the path separator and cannot be part of a name — the same idiom as the suite's
   existing 0.28 Windows skip. A non-UTF-8 name is **not** in the suite: macOS is expected to refuse
   creating one and Windows cannot express one, and the captured fixture covers the shape. If either
   platform expectation proves wrong on the matrix, say so.

## 5. Changelog

0.6.0 is released, so open `## Unreleased`. One `### Fixed` entry: the Changes view lists unsupported
paths. Since 0.1.0 it counted them and showed none, because stikk matched a word prikk never printed.
Those paths also block commit, and prikk's reason is now visible beside them. **No `### Breaking`.**

## 6. Gates

The eight, per `.git-exclude/specs/02-implementer-handoff.md`: gates 1–5, 7 and 8 on the MSRV; gate 6 on
stable with a fresh `CARGO_TARGET_DIR`.

**Gate 6, specifically.** 0.6.0 is published and the workspace still says `0.6.0`, so `cargo package`'s
verification resolves published siblings. **A adds no cross-crate API**, so it should pass. If it does
not, that is the 023-B shape: report it, and do not bump — the bump is B's.

**The real-binary suite on the full matrix**, because §4.5 has a Windows skip that only the Windows leg
can show. Name the `CI` run id and the suite's at one SHA.

## 7. Acceptance criteria

1. Every indented line in the scoped entries region is an entry, with any first word as its kind;
   `WORKTREE_KINDS`' closed list is gone.
2. `"unsupported-path"` maps to `Unsupported`; the never-printed `"unsupported"` arm is gone.
3. Both documented guarantees are true, with a test that goes through the parser.
4. Fixtures captured verbatim at 0.28.0 and 0.41.0 from a neutral directory, with provenance.
5. The count invariant applied to every `worktree-status` fixture.
6. RFC 021 F0's tests unchanged and green.
7. The 80-column capture reported; layout changed only if an entry was unreadable.
8. A real-binary `unsupported-path` test at both ends, the Windows skip announced; matrix run id named.
9. A `### Fixed` entry; no public shape change; no version bump.
10. Eight gates under the toolchain rule; `CI` and suite run ids at one SHA.
11. Nothing tagged or published.

## 8. Submit

Package to `.git-exclude/review-request/027-a-unsupported-path-entries/review-request-v1.md`.

**Lead with the Changes view at 80 columns, before and after**, from the 0.41 fixture — the header
counting two and the list showing none, then both. **Then the invariant run over the old fixtures.**

**Push once approved.** Handoff B follows.
