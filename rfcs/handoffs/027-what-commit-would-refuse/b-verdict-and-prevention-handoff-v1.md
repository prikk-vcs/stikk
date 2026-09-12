# Handoff B — the verdict, the Changes view, and commit's prevention (v1)

**Companion to:** [RFC 027](../../accepted/027-what-commit-would-refuse.md) (Accepted 2026-09-13, **Q1
ruled (b)**; **F6 ruled by the architect** the same day — read both rulings first).
**Follows:** [Handoff A](a-unsupported-path-entries-handoff-v1.md), approved at `303668b`. B is built on
A's reader.
**Design items:** `FR-034`, `FR-050`, `UD-06`, `UD-02`, `C-T4d`, `C-T2b`, `C-T2c′`, `ER-02`, RFC 009
decision 3 (as amended by F6), RFC 014 decisions 1, 2 and 5b.

> **B is breaking, and 0.6.0 is published.** Your first commit bumps the workspace to **0.7.0**, on its
> own. The 023-B ruling's condition holds: the workspace version names a published release, and B adds
> fields that cross crates, so gate 6 cannot verify without it.
>
> **Three things decide whether B is right, and none is the parser.** A refused entry must be refused on
> screen; below 0.39 the verdict must read *unreported*, never zero; and the queued-elsewhere warning
> must survive the move to JSON with every one of its safety claims.

---

## 1. Scope

**In, in this order:** the 0.7.0 bump; the JSON reader (§2); the model (§3); queued-elsewhere under F6's
ruling (§4); the Changes view, including Handoff A's three asks (§5); commit's prevention (§6); the suite
(§7); the changelog (§8).

**Out:** editing `.prikkignore` (stikk never writes into a repository); preventing commit on
`unsupported-path` (Q1 ruled (b)); rendering rename declarations; partial commit; the Queue view.

## 2. The JSON reader, at ≥ 0.39

Under `CliBackend::reads_json` — the same `39`, in the same one place. Run
`worktree-status --ref <ref> --format json`. **Keep the prose path's exit handling exactly**
(`cli_backend.rs`'s `worktree_status`): prikk exits 1 for a dirty tree with the report on stdout, so
capture without classifying, parse stdout, and classify only if no report parses.

**The schema, as prikk 0.41's emitter writes it and its binary prints it:**

| field | read as |
|---|---|
| `schema_version` | checked first, `worktree-status-report-v1`, through `parse_json`'s `report()` |
| `ref` | a `RefName`, validated at the boundary |
| `tracked_files`, `unchanged_files` | counts |
| `clean` | bool |
| `refused_count` | count — **and it must equal the number of refused entries**; if not, that is a schema error, not a number to pick between |
| `queued_elsewhere` | `null` or a ref name, validated as a `RefName` — §4 |
| `changes[]` | `path`, `kind`, `detail`, `authoring`, `refusal` |
| `declarations[]` | tolerated; nothing is read from it |

- **`authoring` is exactly `"authored"` or `"refused"`.** `"refused"` requires a string `refusal`;
  `"authored"` requires `refusal: null`. Anything else is a schema error. Do not guess.
- **Per-kind counts are derived from `changes`** by kind word. prikk computes its prose counters from the
  same list, so they are equal by construction.
- **Paths are not validated as safe repository paths** (RFC 027 decision 2). An `unsupported-path` entry
  carries an absolute path, possibly with `\` or `U+FFFD`. Carry it as reported; render it inert.

Below 0.39 the prose reader from Handoff A is unchanged.

## 3. The model

**`WorktreeEntry` carries a three-valued verdict**: authored; refused, with prikk's reason verbatim; or
unreported. The names are yours. **The three states are not negotiable**, and "unreported" is not
"authored".

- JSON fills the verdict and takes `note` from `detail`, which carries no refused suffix.
- Prose marks every entry unreported.
- **`WorktreeStatus` carries `refused` as optional**: `None` from prose, `Some(n)` from JSON. **Never
  `Some(0)` from a report that cannot say.**
- **`queued_elsewhere` changes type** — §4.
- `ChangeEntry` and `ChangesView` in `stikk-core` mirror all of it.

These are construction sites to update; I counted them, and you should re-count:
`stikk-core/src/changes.rs`, `changes/tests.rs`, `commit/tests.rs`, `stikk-prikk/src/cli_backend/parse.rs`,
`null_backend.rs`, `stikk-tui/examples/changes_demo.rs`, `app/tests.rs`, `view/changes/tests.rs`.

## 4. Queued elsewhere, under F6's ruling

Measured on 0.41: prose prints prikk's sentence; JSON gives `"queued_elsewhere": "heads/main"` and
nothing more.

**The model holds two different things apart**: prikk's verbatim note, from prose below 0.39, and prikk's
queued ref, from JSON at 0.39 and above. Not one `Option<String>` meaning either.

**At ≥ 0.39 the Changes view renders a warning band in stikk's own words**, outside the *"prikk reported
—"* quote band, which stays reserved for prikk's text. It keeps **every** safety claim of prikk's
sentence, and adds none:

1. the active queue holds unsealed patches for `<queued ref>`, not for the focused ref;
2. that is real committed work, not shown here;
3. an untracked entry here may be exactly that work, seen from this ref's baseline;
4. do not delete on the strength of this view alone.

**A test checks the wording clause by clause** against prikk's sentence as prikk prints it — one assertion
per clause, so a later edit that drops one fails by name. RFC 009 decision 3's suppression of the
untracked filter's *"a commit still captures them"* banner still applies at both bands.

**Below 0.39: prikk's sentence verbatim, in the quote band, exactly as today.**

## 5. The Changes view

- **A refused entry reads as refused** — a text marker, not colour alone (`NFR-A03`), with prikk's reason
  verbatim and inert. **Its kind stays visible**: refused is orthogonal to kind.
- **Unreported, below 0.39**: one line saying this prikk does not report which paths commit would refuse.
  Wording is yours; **it must not be readable as "none"**, and no `refused 0` appears.
- **The header fits at 80 columns** (A's ask 2, required). The counts line already clips `unsupported N`
  today, and B adds `refused N` at ≥ 0.39. Lay it out so every number is visible — two lines is fine.
- **The headline counts what is listed.** It sums four named kinds today, so an entry of an unmodelled
  kind — which A now lists — is on screen and not in the total. Count the entries listed, or state the
  unmodelled ones; either way the number must match the list.
- **An unmodelled kind shows prikk's word**, inert, in the tag position, rather than `changed` (A's ask 3;
  `ER-02`).
- **A refused entry's path stays distinguishable at 80 columns** (A's ask 1). The marker costs columns on
  a row that already spends 14 on the tag. Show it with a long repo-relative path.

`TestBackend` captures at 80 columns for each of these.

## 6. Commit's prevention — RFC 027 decision 5

In `stikk-core/src/commit.rs`'s preview, **after** the cross-ref and clean-worktree checks: if any entry
is refused, commit is unavailable, and the outcome **carries the refused entries themselves** — path and
prikk's reason for each.

**Not `Blocked(String)`.** That outcome renders as the shell's one-line banner (`shell::render_banner`),
which cannot list paths with reasons at 80 columns. **Render it as an overlay in the refusal card's
visual language** — prikk's reasons verbatim and inert in quote bars, stikk's next steps below. **It is
not routed through `present()` or `StikkError`**: RFC 014 decided a blocked preview is not an error.

**The next steps are the measured ones** (RFC 027 F3), in stikk's words:

- remove or replace the path;
- or list it in `.prikkignore` — **and that file is then part of the commit**, which the step must say.

stikk never edits `.prikkignore` (`CON-1`, `C-E2`). **Where a shown name contains `U+FFFD`, the step must
not imply that the displayed name is the file's real name** — prikk substituted a character, so typing it
into `.prikkignore` would not match.

- **Below 0.39, commit is offered exactly as today**, and prikk's refusal comes back verbatim.
- **If the tree changes between preview and commit**, prikk's refusal reaches the seam and is presented
  verbatim, through the existing path (RFC 014 decision 2's shape).
- **`unsupported-path` entries do not block** (Q1 ruled (b)).

## 7. The suite, at both ends

1. **An untracked symlink beside an ordinary modification.**
   - At 0.41: the entry is refused, and **its reason equals what `prikk commit` prints for the same tree**.
     Compare against a real commit attempt in the same test, not a literal. `refused` is `Some(1)`, the
     preview carries that path, and after removing the symlink the preview is ready.
   - At 0.28: the entry is unreported, `refused` is `None`, and `commit` is refused with prikk's verbatim
     message.
2. **Queued elsewhere.** At 0.41 the typed ref is `heads/main`; at 0.28 prikk's verbatim note is present.
3. **Windows**: creating a symlink needs a privilege a runner may not have. Announce the skip, in the
   suite's existing idiom, if it cannot, and measure rather than assume.

Unit tests use `NullBackend::with_version` and `with_worktree_status` for both bands.

## 8. Version and changelog

- **First commit: the workspace and its five pins to 0.7.0**, and the lockfile with them, on its own.
- `### Breaking`: the new fields on the four structs, the preview outcome's new variant, and
  `queued_elsewhere`'s type.
- `### Added`: refused entries marked with prikk's reason; commit unavailable, with prikk's reasons, when
  prikk would refuse.
- `### Changed`: `worktree-status` read as JSON at ≥ 0.39; at ≥ 0.39 the queued-elsewhere warning is
  worded by stikk from prikk's field, with every clause kept.
- `### Fixed`: the counts line no longer hides `unsupported N` at 80 columns; an unmodelled kind shows
  its own word.

## 9. Gates

The eight, under `.git-exclude/specs/02-implementer-handoff.md`'s toolchain rule, with the Breaking table
built by the API-diff method in the same spec — not by grep. **The real-binary suite on the full matrix.**
Name the `CI` and suite run ids at one SHA.

## 10. Acceptance criteria

1. The workspace at 0.7.0, in its own first commit.
2. JSON read at ≥ 0.39 under `reads_json`, with the schema checked first, `ref` and `queued_elsewhere`
   validated, `authoring`/`refusal` held to their two legal pairs, and `refused_count` checked against
   the entries.
3. A three-valued per-entry verdict; `refused` optional and never `Some(0)` from prose.
4. Queued-elsewhere carried as two distinct forms. Verbatim below 0.39; at ≥ 0.39 in stikk's labelled
   words, outside the quote band, with a clause-by-clause test.
5. Refused entries marked in text with prikk's reason and their kind; below 0.39 unreported, never zero.
6. The header fits at 80 columns; the headline matches the list; unmodelled kinds show their word; a
   refused entry's long path stays distinguishable.
7. Commit unavailable at ≥ 0.39 when any entry is refused. The refused entries are carried and rendered
   as an overlay, not a banner and not through `present()`, with the measured next steps, the
   `.prikkignore` clause, and the `U+FFFD` caution.
8. Below 0.39, commit is offered as today; `unsupported-path` does not block.
9. Real-binary tests at both ends, the refusal compared with `prikk commit`'s own output; the Windows
   symlink skip announced if needed.
10. The changelog as §8, and a Breaking table from the API diff.
11. Eight gates under the toolchain rule; `CI` and suite ids at one SHA.
12. Nothing tagged or published.

## 11. Submit

Package to `.git-exclude/review-request/027-b-verdict-and-prevention/review-request-v1.md`.

**Lead with three captures at 80 columns:** the blocked-commit overlay at 0.41 with a refused symlink;
the Changes view with a refused entry and the reworked header; and the queued-elsewhere band at both ends
beside the clause-by-clause test. **Then the suite, leg by leg.**

**Push once approved.**
