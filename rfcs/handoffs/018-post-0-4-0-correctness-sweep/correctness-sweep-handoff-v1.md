# Handoff — the post-0.4.0 correctness sweep, and 0.4.1 (v1)

**Companion to:** [RFC 018](../../accepted/018-post-0-4-0-correctness-sweep.md) (Accepted 2026-09-06).
Inherits its state.
**Realizes:** **0.4.1** — a patch release, non-breaking, strings and docs only.
**Design items:** `C-T4a–e`, `T-T4`, `ER-02`, `NFR-R03`, `C-E2`/`CON-1` (§2's replacement claim rests
on these), RFC 011 (patch position).

> **0.4.0 published hours ago and tells users stikk never writes their repository, four lines above
> the keybindings for commit and seal.** That is the whole reason this increment exists and it is the
> only urgent thing in it. The other two findings are riding along because the file is open.
>
> **No behaviour changes.** Strings, docs, one manifest version, and tests that pin the strings. If you
> find yourself changing what code *does*, stop and report it.

---

## 1. Scope

**In:** §2 (the Glossary claim), §3 (the Trust & Keys pointer), §4 (`ROADMAP.md`), §5 (the grep scope
rule), §6 (version + changelog).

**Out:** the Trust & Keys **view** (`FR-104` — 0.5.0, its own design); the key-id module (0.5.0); the
real-binary integration suite (0.5.0); any behaviour change; anything that would make this a minor
rather than a patch.

---

## 2. The Glossary claim *[the urgent one]*

`crates/stikk-tui/src/overlay.rs:234`, first line of `render_glossary`:

```rust
"  stikk reads prikk; it never writes your repository.",
```

**RFC 018 decision 1 said to delete the claim rather than qualify it. I am refining that, and the
refinement is the useful part of this handoff — read it before you write the line.**

The problem was never that the sentence made a promise. It is that the promise was **anchored to a
feature set** — true while stikk had no mutations, false the moment it gained one, with nothing
connecting the sentence to the thing that made it true.

**A claim anchored to an architectural invariant is safe.** stikk has one that says almost what the old
line was reaching for, and it cannot go stale, because a test enforces it:

- **`CON-1`** — only `stikk-prikk` talks to prikk, through its public CLI. stikk never touches a
  repository directly.
- **`C-E2`** — stikk performs no repository-internal writes, ever.

The fault screen already words this correctly: *"Only prikk writes repositories, and only on a
confirmed mutation — none happened here."*

**So: replace the line with a claim of that kind, or with no claim at all.** Something true of stikk's
architecture rather than of its current feature list. The rule to apply, and to put in the code comment
beside whatever you write:

> A user-facing claim may rest on an invariant this project enforces. It may not rest on the set of
> features that happen to exist today.

**Do not** simply enumerate current behaviour ("stikk commits and seals") — that is the same defect
with a new expiry date, since the next mutation makes it incomplete rather than false.

**And read the whole panel while you are in it.** The key list two lines down was edited by RFC 016;
nobody looked up. Check every line of `render_glossary` against what 0.4.0 actually does, and report
anything else you find, including things you decide are fine.

**Test:** a render assertion that the Glossary panel does **not** contain a never-writes claim, and
does show the mutating keybindings. Assert the **absence** — that is the thing that regressed.

*If you can see a way to pin the invariant rather than the string — for instance, a test asserting that
if the key list contains a mutating action then the header contains no absolute never-writes claim —
propose it. RFC 018 names "nothing pins a claim" as an open problem and says outright that I do not
have a mechanism. **A real one found here is worth more than the fix.*** Do not force it; a narrow
absence assertion is an acceptable answer.

## 3. The Trust & Keys pointer

`crates/stikk-tui/src/app.rs:1178`:

```rust
Target::TrustKeys => format!("{detail} — see Glossary → Trust & Keys"),
```

**There is no Trust & Keys section.** There is a section called `Keys`, and it is the keyboard shortcut
table. A user hitting a signing refusal is sent to a list of what `j` and `k` do.

**Point at what exists, or at nothing.** The trust-refusal card already carries its own gloss and a
`glossary: <code>` line; a signing-readiness `NotReady` carries prikk's own message naming the missing
environment variable. **A pointer to a place that does not exist is worse than no pointer** — it costs a
navigation to discover.

Whatever you choose, the line must stay true when `FR-104`'s view **does** land — so do not write copy
that will need editing then. If the honest answer today is no pointer, take it.

**Test:** the guidance for a trust refusal and for an absent signing key both render without directing
the user to a Glossary section that does not exist.

## 4. `ROADMAP.md` — three stale entries

Outside both prior grep scopes, and so never re-read:

- **The validated range** — *"validated through `0.30.0`"* → **0.33.0**. Three releases stale.
- **`UD-01`'s row** — *"patch messages are discarded"*. **Retired at prikk 0.32** (RFC 015). The
  `UD-08` row directly above shows this file's own retirement format; follow it.
- **`UD-09`'s row** — *"no per-patch content, no patch-id enumeration"*. **Narrowed at 0.32**: patch
  **ids** are enumerable for messaged patches; **content** is not. RFC 015's own wording is careful
  here — do not overstate the retirement, and `requirements.md`'s `UD-09` entry is the reference.

**Then re-read the rest of the file** against what 0.4.0 does. It has never been swept; the three above
are what a version grep would find, not necessarily what a reader would.

## 5. The grep scope — write it down once

The scope has been widened twice, reactively, after each miss: `README.md docs/` (RFC 015) →
`+ crates/ examples/ CHANGELOG.md` (RFC 017) → and it still missed `ROADMAP.md`.

**Replace the inclusion list with an exclusion list** in `.git-exclude/specs/02-implementer-handoff.md`:
every tracked `.md` and every `crates/**` source file, minus whatever genuinely should not be swept
(`CHANGELOG.md`'s historical sections are dated claims, not current ones — name that exception
explicitly).

**An exclusion list is shorter and fails safe.** An inclusion list has now failed three times.

## 6. Version and changelog

- `[workspace.package] version` and the five `[workspace.dependencies]` pins: `0.4.0` → **`0.4.1`**.
  (The pins are not optional — crates.io rejects a path dependency without an exact version. That was
  found in 0.4.0's prep, not in my handoff; it is written down now.)
- `cargo update --workspace` for the lockfile. Not a `--locked` build.
- A `## 0.4.1 — <date>` section, `### Fixed` only. **Lead with the Glossary claim and say plainly what
  it said and for how long** — one released version, hours. This project's changelog has described
  every prior correction honestly; this one is the most user-facing of them.
- No `### Breaking` section. If you find yourself needing one, this is not a patch release — stop and
  report.

## 7. Gates

All five, including the rustdoc gate 0.4.0 added:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo build --examples -p stikk-tui --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --locked
mdbook build docs
cargo package --workspace --locked
```

## 8. Acceptance criteria

1. The Glossary makes no claim anchored to a feature set; whatever replaces it rests on an invariant or
   says nothing, with the rule recorded in a comment beside it.
2. A render test asserts the **absence** of a never-writes claim and the presence of the mutating keys.
3. Every other line of `render_glossary` checked against 0.4.0's behaviour, with findings reported —
   including "checked, fine".
4. No guidance directs a user to a Glossary section that does not exist; the copy stays true when
   `FR-104` lands.
5. `ROADMAP.md`'s ceiling, `UD-01` and `UD-09` rows corrected, and the rest of the file swept.
6. The grep scope in the handoff spec is an exclusion list, with `CHANGELOG.md`'s historical sections
   named as the exception.
7. Version `0.4.1` including the dependency pins; lockfile refreshed; `### Fixed`-only changelog
   leading with the Glossary claim.
8. All gates green; `cargo package` clean; **no behaviour change**.
9. Nothing tagged or published.

## 9. Submit

Package to `.git-exclude/review-request/018-post-0-4-0-correctness-sweep/review-request-v1.md`.

**Lead with the 80×24 render of the Glossary panel**, before and after. It is a screen; show it.

Then §2's sweep of the rest of the panel, and anything in `ROADMAP.md` a reader would catch that a grep
would not.

**And if you found a way to pin a claim rather than a string, lead with that instead** — it is the more
valuable of the two things this increment could produce.

**Push once approved** (`.git-exclude/specs/02-implementer-handoff.md` §6). The tag and publish are the
owner's to authorize and mine to perform.
