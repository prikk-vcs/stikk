# Handoff A — RFC 036: repository text renders as it is, everywhere (v1)

**Companion to:** [RFC 036](../../accepted/036-repository-text-renders-as-it-is.md), **accepted 2026-10-14**, no
open question. **Read the RFC first**; this handoff does not repeat its reasoning.
**Covers:** all eight decisions. 0.10.0's increment 1 (`ROADMAP.md`, *Next — 0.10.0*).
**Not a release.** 0.10.0 stays unreleased; nothing is tagged.

> **What this increment is.** 0.9.0 escaped Patch detail's content from one table, and its notes say every other
> view still passes bidi overrides and zero-width characters to the terminal. This applies that table everywhere,
> under two rules: **prose** is escaped in place; **identifiers** are plain or quoted, so two never look alike and
> layout cannot change them.
>
> **The floor:** nothing that does not show as itself reaches the terminal from repository or prikk text, in any
> view or in the one-shot printout. **One predicate, `stikk_prikk::escape::needs_unicode_escape`. No second table.**

---

## 1. Measure first, at prikk 0.51.0

RFC 036 F2 rests on prikk's source. Two things it does not settle; **measure them before writing the renderer**,
and keep the script and its output in the review-request folder:

- **(a) Filenames prikk reports but will not track.** In a scratch repository, create on-disk files named with a
  bidi override (U+202E), a zero-width space (U+200B), a non-breaking space (U+00A0), a double space (`a  b.txt`),
  a leading space (` lead.txt`), and a `"`. Record what `worktree-status --format json` reports for each: the kind
  (`untracked`/`unsupported`), and **the path's exact characters** (decode the JSON; do not eyeball it).
- **(b) A branch name** carrying U+202E, and one carrying U+200B. Does `prikk branch create` accept it? If so, what
  does `branch list --format json` report?

**If prikk refuses or normalises something RFC 036 F2 says can pass through, stop and report.** The design narrows
(an identifier path that cannot occur needs no test leg); it does not widen.

**Seeds stay in `0600` files from `key generate --out`; only `public key:` is ever read.** On filesystems that
refuse a name (Windows, some macOS settings), the suite leg skips that name and says so; it does not fail.

## 2. Two functions, in `stikk-tui/src/text.rs`

**Prose: `inert`, redefined.** Every character for which `needs_unicode_escape` holds is written as its escape:
`\n`, `\t`, `\r`, the two short ones `escape.rs` already uses, and `\uXXXX` (with surrogate pairs above U+FFFF).
**`"` and `\` are left alone; nothing is quoted.** This replaces `U+FFFD`. `escape.rs` has no public single-character
helper (`push_unicode_escape` is private), so **add one there** (for example `escape_invisible_into(out, text)`), so
the prose rule and `escape_into` share one spelling. **Do not copy the table or the spelling into `text.rs`.**

**Identifiers: `identifier`, new.**
- **Plain** when the value is non-empty, no character needs an escape, it contains no `"` and no `\`, it has no
  leading or trailing whitespace, and it has no run of two or more whitespace characters.
- **Otherwise quoted**: `"`, then `escape_into` (so `"` and `\` are escaped), **with every whitespace character
  written as an escape** (space as ` `), then `"`. The quoted form contains no whitespace.
- The empty value renders as `""`.

**Both return a `String`, and both are pure**, testable without a terminal.

## 3. Classify every call site: the table goes in the request

There are **69** non-test `inert(` call sites in `stikk-tui`, plus `stikk/src/main.rs`. For **each**, decide prose or
identifier, and put the whole table in the review request (file, line, what the string is, which rule).

- **Identifiers:** paths (every view, overlay headings, Patch detail titles), ref names (titles, the ref picker,
  queued-elsewhere clauses, the status bar), key ids, prikk's kind word in a tag position (`ChangeKind::Other`).
  Hex object ids are identifiers too; they are always plain, so nothing changes visibly.
- **Prose:** messages (patch and queued), prikk's refusals, notes, reasons, routes, warnings, and its version line.
- **When a string is neither clearly, it is prose** (RFC 036 decision 4), the rule that cannot drop a character.
- **A string built by `format!` from both kinds** (`"{kind} {path}"`) applies the rule **per piece**, before
  formatting. Never apply `identifier` to a whole sentence.
- **stikk-authored text goes through neither.**

## 4. A Changes entry row never clips its path (RFC 036 decision 6)

`view/changes.rs`, `entry_lines`: today the tag, path and note are one `Line`, and a long path is cut at the
border. **The path continues on further rows at the entry's indent, hard-split** (`wrap_verbatim`; the identifier
form has no whitespace runs to protect, and a plain path's single spaces must survive), and **prikk's note follows,
wrapping as prose.** The tag stays on the first row. **Test at 80 columns** with a path of 150 characters: every
character appears exactly once, in order.

## 5. The launcher's printout (RFC 036 decision 5)

`stikk/src/main.rs`, `print_orientation`: **the repository root** (identifier: a path), **the prikk version line**
(prose), **`heads/main`'s id** (identifier), and the queued target (identifier, where today it is `inert`). The root
is a local path and may legitimately be non-ASCII. Visible non-ASCII stays as itself; only what does not show as
itself is escaped.

## 6. The Glossary

0.9.0's `Notation` entry (*Quoted content rows*) gains **one** paragraph, or a second `Notation`: *a path or name
stikk cannot show plainly is quoted, with whitespace written as an escape so layout cannot change it*. Keep the
example free of real runs of spaces: the Glossary panel itself word-wraps (the lesson from RFC 035 v3 §4).

## 7. The traps

1. **No second table.** `text.rs` calls `escape.rs`; it does not decide what is invisible.
2. **Do not quote prose.** A refusal that contains `"` must read as prikk wrote it.
3. **`U+FFFD` is visible, and stays itself.** prikk substitutes it for filename bytes it cannot show, and
   `stikk-core/src/commit.rs` keys a next step on it (*"A name shown with � is not the file's real name"*). Once stikk
   stops *producing* `U+FFFD`, a `�` on screen means prikk put it there, which makes that step exact. **Check the
   step still fires** on prikk's substitution, and say so.
4. **A quoted identifier must survive `wrap_indented` unchanged**: it holds no whitespace. Test it: an identifier
   embedded in a sentence and wrapped at several widths reappears byte-identical.
5. **`joined()` collapses whitespace in your own tests** (RFC 035 v2). Use `on_a_row` for anything about spacing.
6. **Tests that assert `U+FFFD` from `inert` change**, because that is the behaviour this RFC changes. List every one
   in the request, old assertion against new. **A test that asserts `U+FFFD` for any other reason** (`json.rs`'s
   unpaired surrogate, `commit.rs`'s step) **does not change.**
7. **Paste captures as the file holds them** (`feedback`: a review document must not be able to reorder itself).

## 8. Tests

- **Unit, `text.rs`:** each plain/quoted trigger alone (a bidi override, a zero-width space, a non-breaking space,
  `"`, `\`, a leading space, a trailing space, a double space, empty); **injectivity across a look-alike set** (at
  least the sixteen from RFC 035's set, plus `a b`/`a  b`/` a b`) for `identifier`; prose escapes every row of RFC
  035 A3's table and leaves `"`/`\` alone; trap 4.
- **One view test per rule per view**, at least: a ref name in a title, a path in Changes, a message in the Queue, a
  refusal in the overlay.
- **§4's long-path test, and §5's printout test** (the existing `stikk/src/tests.rs` is the place).
- **Suite legs at 0.51** for whatever §1 found prikk passes through: the filenames in the Changes view, and the
  branch name in the ref picker, rendered as the rules say. **At 0.28** the same legs run where 0.28 can create the
  state; otherwise they announce a skip, as RFC 035's 0.51-only legs did.
- **Captures at 80 columns:** a Changes view with §1's filenames and a 150-character path; the ref picker with §1's
  branch; the overlay with a refusal containing `"`; the launcher's printout.

## 9. The changelog: this increment opens `## Unreleased`

Create **`## Unreleased`** above `## 0.9.0`, and write this increment's entries as the end state:
- **`### Security`**: what was passed through before (bidi overrides, zero-width and other default-ignorable
  characters, non-ASCII whitespace, in paths, messages and ref names, in every view but Patch detail's content, and
  in the one-shot printout), and what happens now, under both rules. Name it as the gap 0.9.0's notes declared.
- **`### Changed`**: control characters are shown as escapes, no longer as `�`.
- **`### Fixed`**: a long path in a Changes row no longer clips; double spaces in a path no longer collapse.

## 10. Gates, runs, submit

The eight gates as always: 1–5, 7, 8 on the MSRV; gate 6 on stable with a fresh `CARGO_TARGET_DIR`. Then, from a
`prep/036-repository-text` branch: CI, the real-binary suite with `full_platform_matrix=true`, and the supply-chain
gate, **all at one SHA**, every job and leg read, the branch deleted before submitting.

Package to `.git-exclude/review-request/036-repository-text/review-request-v1.md`. **Lead with §1's measurement**,
then the classification table, then the captures. **If §1 contradicts RFC 036 F2, put that first of all.**

**Push your own commits once approved.**
