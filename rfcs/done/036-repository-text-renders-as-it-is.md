# RFC 036 — Repository text renders as it is, everywhere

**Status.** **Done 2026-10-14**: Handoff A delivered on `main` (`74a1436`, `341d024`); in `## Unreleased` for 0.10.0. **Accepted by the project owner 2026-10-14.** **Proposed 2026-10-14** by the architect, as increment 1 of the authorized 0.10.0 scope
(`.git-exclude/release/0-10-0-scope.md`). **No open question.**
Grounded in stikk's own code at `b29f997` and prikk's source in the clone (`prikk-object/src/path.rs`). The handoff
measures the two things the source does not settle, at prikk 0.51.0 (§ Delivery).
**Tracks.** `T-T2`, `C-T2a`, `C-T2b`, `ER-02`, `T-T4`; RFC 009 (M1), RFC 024, RFC 027 (carried row budget), RFC 035
A3.
**Touches.** `stikk-tui` (`text.rs` and every view that renders repository or prikk text), the `stikk` launcher's
one-shot printout, the Glossary, and the changelog.

## Summary

**0.9.0 made Patch detail's content show exactly what it is, and its release notes say the rest of stikk does not.**
`inert`, which every other repository string passes through, turns control characters into `U+FFFD` and lets
everything else through. A bidi override in a branch name or a message reorders what the terminal shows. A
zero-width character makes two names look alike. And a double space in a path disappears into word-wrapping.

This RFC applies the same table to every string, under **two rules**: one for **prose** (messages and prikk's
sentences), which must never reach the terminal as anything but itself; and one for **identifiers** (paths, ref
names, key ids), which must also never look like a different identifier. It also stops a long path being clipped in
the Changes view.

## Findings

### F1 — `inert` handles `Cc` only, at 69 call sites and one printout

`stikk-tui/src/text.rs`: `inert` replaces every `char::is_control()` character with `U+FFFD` and passes the rest
through. It is called at **69** non-test sites across `status_bar`, `shell`, `overlay` and every view. The launcher's
one-shot orientation (`stikk/src/main.rs`) routes only the queued target through it. **The repository root, the prikk
version line and `heads/main`'s id print raw.**

`is_control()` is category `Cc` alone. **Bidi controls, zero-width characters, non-breaking and other non-ASCII
whitespace, and tag characters all pass through** (measured in Rust 1.88: U+202E, U+2066, U+200B, U+FEFF and U+00AD
are neither control nor whitespace). 0.9.0's notes state this as a limit, and the roadmap carries it.

### F2 — where such characters can come from

- **A tracked path cannot carry one.** prikk's `validate_repo_path` (`prikk-object/src/path.rs`) requires **ASCII**,
  refuses control characters, `\` and `:`, and refuses a component ending in a space or a dot. **But double spaces,
  a leading space and `"` are all legal**, so a tracked path can still lose its shape in word-wrapping.
- **A path prikk reports but will not track** carries whatever the filesystem allows. `worktree-status`
  reports such files (`unsupported`, `untracked`) under their on-disk names, which can be non-ASCII, bidi or
  zero-width. **The handoff measures exactly what prikk 0.51 reports for such a file** before the reader relies on it.
- **Ref names:** stikk's `RefName::parse` rejects only empty names and `Cc`. **prikk's rule for a branch name is not
  settled by the source the architect could find**, so the handoff measures it.
- **Messages and prikk's own sentences** (refusals, notes, routes) are arbitrary Unicode.

### F3 — word-wrapping changes identifiers

`wrap_indented` rebuilds every row from `split_whitespace()`. That is right for prose and wrong for a path such as
`a  b.txt` or ` lead.txt`, both legal (F2): the double space becomes one, and the leading space vanishes. RFC 035
fixed this for content with `wrap_verbatim`. **Identifiers sit inside sentences** (*"create-file a  b.txt"*, a
refused path's heading), so they cannot simply be wrapped verbatim.

### F4 — a long path in a Changes entry row is clipped

`view/changes.rs`, `entry_lines`: the tag, the path and prikk's note are one `Line`, and **the path is never
wrapped**, so a path wider than the row is cut at the border. That is the ceiling half of RFC 027's carried row
budget. The would-refuse overlay already hard-wraps its paths, so it is not affected.

## Decisions

1. **One predicate, no second table.** Every rule below decides "does not show as itself" with
   `stikk_prikk::escape::needs_unicode_escape`, and spells an escape with RFC 035 A3's notation (`\n`, `\t`, `\r`,
   `\uXXXX`, surrogate pairs above U+FFFF).

2. **Prose: every character that does not show as itself is written as its escape, in place.** This applies to
   messages and to prikk's refusals, notes, reasons and routes. `"` and `\` are left as they are; nothing is quoted.
   **This replaces `U+FFFD`**, which said *something was here* but not *what*. The promise is **`T-T2` only**:
   nothing invisible or reordering reaches the terminal. Prose may still reflow its whitespace in layout, as it does
   today.

3. **Identifiers are rendered so that two different ones never look alike.** This applies to paths, ref names, key
   ids, and prikk's kind words where they appear as tags.
   - **Plain** when the identifier is non-empty, no character needs an escape, it contains no `"` or `\`, it has no
     leading or trailing whitespace, and it has no run of two or more whitespace characters.
   - **Otherwise quoted**, with the content escaped as in RFC 035 A3, **and every whitespace character inside the
     quotes written as an escape** (a space as ` `). The quoted form then contains no whitespace, so no
     word-wrap can collapse, trim or break inside it. A quoted identifier longer than the row is hard-split, and
     loses nothing.
   - The empty identifier is `""`.
   An identifier that needs quoting is rare (F2), so the plain form is what a user nearly always sees.

4. **Every repository- or prikk-sourced string goes through exactly one of the two rules.** stikk-authored text goes
   through neither. The dev team classifies **all 69 call sites and the launcher's printout**, and the review
   request carries the table. A string that is neither clearly prose nor clearly an identifier is classified as
   **prose**, because that is the rule that cannot drop a character.

5. **The launcher's one-shot printout applies the same rules** to the repository root, the prikk version line and
   `heads/main`'s id, as well as the queued target. RFC 009's M1 already explains why stdout counts as a terminal.

6. **A Changes entry row never clips its path** (F4). The path continues on further rows at the entry's indent,
   hard-split and losing nothing. prikk's note follows it, wrapping as prose.

7. **The Glossary's notation entry grows by one line**: an identifier stikk cannot show plainly is quoted, with
   whitespace written as an escape so layout cannot change it.

8. **Patch detail's content is unchanged** (RFC 035 A3). Its operation titles are identifiers and follow decision 3.

## Changelog

**This one has a `### Security` entry.** It fixes a weakness that 0.9.0 and every earlier release shipped, and
0.9.0's notes said so. The entry goes in `## Unreleased`, which the first handoff of 0.10.0 creates.

## Delivery

**One handoff, issued on acceptance:** [Handoff A](../handoffs/036-repository-text-renders-as-it-is/a-repository-text-handoff-v1.md). It **measures first**, at prikk 0.51.0:
(a) what `worktree-status --format json` reports for on-disk filenames carrying a bidi override, a zero-width
space, a non-breaking space, a double space, a leading space and a `"`;
(b) whether prikk accepts a branch name carrying a bidi override or a zero-width space, and what `branch list`
reports for it.
**If prikk refuses or normalises something F2 assumes it passes through, the dev team stops and reports**, and the
design narrows; it does not widen.

## What this RFC does not do

- **No change to prose layout**: whitespace in a message may still reflow.
- **No change to what prikk is asked or how its reports are parsed.** This is rendering only, and `CON-1` is
  untouched.
- **No Compare.** RFC 038 builds on this layer.

## Delivered

**One handoff, two commits, green on `main` at `341d024`** (CI and Docs on `main`; the real-binary suite on the full
platform matrix and the supply-chain gate at the same tree, 43 cases at prikk 0.28.0 and 0.51.0).

- **F2 measured before building, at 0.51:** prikk reports on-disk filenames carrying bidi, zero-width and
  non-breaking characters (as `unsupported-path`), and `a  b.txt`, ` lead.txt` and `say"hi.txt` as ordinary
  `untracked` files it would commit. **It accepts a branch name carrying a bidi override or a zero-width space**, and
  prints the override raw in its own success message.
- **Two rules on one table:** `inert` (prose) escapes in place; `identifier` is plain or quoted, with whitespace
  escaped inside the quotes. `escape_invisible_into` was added to `escape.rs`, so `text.rs` holds no table. All 68
  call sites and the launcher's printout were classified, 33 as prose and 35 as identifiers.
- **Decision 8, met in review v1 §2.3:** `stikk-core` returns a sentence as `Piece::{Said, Name}`, and the frontend
  renders each piece (`text::compose`). Core carries structure; each frontend applies its own rules. Four sites were
  converted, and `## Unreleased` carries their Breaking rows.
- **A Changes row never clips its path**, and double, leading and trailing spaces in a path survive.
- **`### Security`** in `## Unreleased`, including that the one-shot printout used to write the repository root and
  prikk's version line with no filtering at all.
- **Found in review, by the dev team:** an escape typed with one backslash was decoded into the character before it
  reached the file, **three times**. One instance had shipped in 0.9.0's rustdoc for `escape.rs` itself. All three
  are fixed, and a sweep of every tracked file now finds no invisible character.

## Carried forward

- **History's queued tier is one unwrapped `Line`**, so a long ref name in it is clipped at the border. It predates
  this RFC (0.9.0 has it) and is the same class as F4. It is a small follow-up in 0.10.0.
- **The Changes foot says *"per-file content diff awaits prikk support (UD-09)"***, which has been false at the
  ceiling since prikk 0.46 (`C-T2b`). It goes to Compare (RFC 038).
- **`queued_elsewhere_clauses` still takes text the frontend rendered**, the one place left with that pattern. Move it
  to `Piece`s whenever it is next touched.
