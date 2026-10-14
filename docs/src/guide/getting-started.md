# Getting started

stikk is a front-end for the prikk version control system. It does not replace prikk's own command
line — VCS verbs stay prikk's — it helps you *read and work with* prikk history.

## Install

[Install](./install.md) has the one-per-platform steps for getting a working `stikk` and `prikk`
without a compiler. In short: run prikk's installer, download stikk's archive for your platform,
check it against the published checksum, and put it on your `PATH`. Rust users can
`cargo install stikk` instead.

This page assumes both are installed and starts from there.

## Point stikk at prikk

stikk drives the `prikk` binary. If prikk is on your `PATH`, nothing is needed. Otherwise, point at a
specific build:

```sh
export STIKK_PRIKK_BIN=/path/to/prikk
```

stikk requires prikk **≥ 0.28** and is validated through **0.51.0**. A prikk below the floor degrades
to read-only where it can; a prikk above the validated ceiling still runs, but Orientation says its
output shapes have not actually been checked against it, rather than silently assuming they have.

**One known upstream limitation at the floor, on Windows only:** prikk **0.28** cannot commit a file
that lives in a subdirectory when running on Windows — its worktree scan builds the repository path
with the platform separator and its own path validator then refuses the backslash
(`invalid name: backslashes are not allowed in repository paths`). This is prikk's, not stikk's, and
prikk fixed it in **0.29.0**; on Windows, use prikk ≥ 0.29. Every other supported version is
unaffected, and so is 0.28 on Linux and macOS. stikk's real-binary suite covers this combination
explicitly rather than skipping it quietly.

## Open a repository

```sh
stikk /path/to/repo
```

On a terminal this launches the interactive **TUI**: an Orientation view — the prikk version and
whether stikk supports it, the queue depth, your signing readiness, and the capability that readiness
gives you — inside a shell with a status bar and a Help overlay. Run it inside a repository with no
argument and stikk discovers the repository root by walking upward for a `.prikk` directory, the same
way prikk does.

**A checkout that stopped part-way** — prikk's own term for a materialization it did not finish — is
named in Orientation in prikk's words, on prikk ≥ 0.43. While it stands, commit's preview is blocked;
History and Changes keep working.

**Where stikk opens.** On prikk ≥ 0.42, stikk opens focused on prikk's current branch, and when you
choose another ref the status bar shows prikk's default beside it. Below 0.42, or when prikk reports its
current branch unresolved, stikk opens on `heads/main` if that is published, and otherwise with no ref
focused and the ref picker open. Choosing a ref in stikk never moves prikk's current branch and never
touches the worktree.

Key reference: `Enter` opens History and drills into a block; `b` chooses which ref to view; `w` opens
Changes (worktree vs. baseline), `u` toggles its display-only untracked filter; `Q` opens the Queue view,
the patches waiting to be sealed; `V` runs prikk's repository verification and shows its
verdict; `C` commits worktree changes and `S` seals the queue, each after a
preview and a confirmation; `:` opens the command palette; `R` shows the session's recent refusals; `o` lists background operations still in flight or
recently finished (a listing only — there is no cancel action); `?` opens the glossary and full key
reference; `m` marks a block in History and `d` compares — see *Compare* below; `v` toggles Patch
detail's, Compare's and Verify's raw view of prikk's own fields; `r` refreshes Orientation and the
screen you are on — History, the Queue, Changes, the tip's Block detail, a Patch detail opened from
the Queue, or Verify (which re-runs it) — **in place**, each view staying visible while its read runs, and **re-reading the ref that screen
shows**, which need not be the ref you have focused; `Esc`/`q` steps back, and quits at the root.

**Is the repository sound?** `V` runs prikk's own verification and shows what prikk said: the
verdict, every condition prikk is currently failing in prikk's own words, and every stage prikk ran
with its status. `v` shows prikk's own JSON report and `r` re-runs it — a verdict is about the
moment it was taken, so stikk never shows you a cached one. **Verify changes nothing**: it repairs
nothing, clears no lock, and never runs on its own, because it scans the whole repository.

prikk's JSON report does not say *which* objects or refs failed, or how many signatures could not be
verified — those live only in prikk's prose output. Verify says so on every report, sound or not,
and names the command to run for them. stikk does not parse that prose. Below prikk 0.39 stikk has
no report to read, and says that rather than reporting a repository it never checked as sound.

**When another terminal changes the repository.** stikk checks for a commit, seal or branch switch made
outside it — when your terminal reports that you have come back to it, and every few seconds while stikk
is idle. When it finds one it refreshes what is on screen and says *"repository changed outside stikk —
refreshed"*, and a commit or seal confirmation that is open becomes stale at once, so nothing armed
survives a change it was not built for. The check is silent otherwise: it never appears among background
operations. **Its limit:** editing a file in your worktree is not a repository change, so it is not caught
this way — `r`, opening Changes, or pressing Enter on a confirmation catches that.

**Two things the Changes view says that prikk's own report does not.** A file moved with `prikk mv` is
listed by prikk as one missing path and one untracked path; stikk marks them as the two halves of one
**declared rename** and counts `renames N`, because that is the single operation a commit will author.
**On prikk ≥ 0.44 that verdict is prikk's own** — prikk states for each declaration whether it will be
authored as a rename, as a deletion, or refused, and stikk shows what prikk said rather than inferring
it from which halves are listed; below 0.44 stikk still infers. **A declaration prikk says it would
refuse makes commit unavailable**, with prikk's refusal in prikk's words, even on a worktree prikk
otherwise calls clean. And when the ref has **no published history** — nothing sealed on it yet —
stikk says so in place of "against baseline", since there is no baseline to compare against: a commit
would be the ref's first, or, once patches are queued for it, those queued patches are its baseline.

Run `stikk` piped or in CI (no terminal) and you get the same orientation as a one-shot print instead.
To see the TUI with no repository at all: `cargo run -p stikk-tui --example orientation_demo` (also see
`history_demo`, `explanation_demo`, and `changes_demo` for the other views, all scripted — no prikk
binary or repository needed).

**Patch detail** is built, at prikk ≥ 0.39. Open a block from History and its **patches** are listed:
`Enter` on one shows what it changed — a text edit's spans, a create's content and mode, a delete's
preimage, a binary replacement's blob ids and sizes, a permission change's two modes, and a rename with
the key that asserted it. `v` switches to prikk's own fields. The Queue opens a *queued* patch the same
way.

**File content is shown quoted**, one row per line, with the line's own newline inside the quotes:
`"    let x = 1;\n"`. That is deliberate — a span edit is very often a whitespace edit, and a renderer
that tidied the spacing would show a re-indent, a tab turned into spaces or a removed trailing newline
as *no change at all*. Inside the quotes the spelling is prikk's own JSON, and it covers every
character that does not show as itself: a tab is `\t`, a line ending `\n`, and anything invisible or
direction-changing — a zero-width character, a non-breaking space, a bidirectional override — is
written as `\uXXXX`. Visible text stays as itself. Two lines that differ in any byte never look
alike, which is the whole point of showing content this way; the Glossary (`?`) names the notation. A
binary file's create or delete shows its blob id and byte count, never its bytes.

Three more things it says rather than guesses: content past stikk's display bound is cut with the
withheld byte count stated; where prikk resolves no path for an operation it says so and names prikk's
node id — which also happens for a sealed patch inside a block of several, where prikk resolves paths
**at the block's end**, so a path there is labelled *as at the end of this block*; and if the patch
list itself cannot be read, the rest of the block is still shown with the reason in its place.

### Compare: what changed between two points

**`d` opens Compare** (prikk ≥ 0.46), which shows what changed between two points in **prikk's own
`diff` hunks**. What it compares depends on where you press it:

- **In History**, `m` marks the selected block — the row shows a `●` and the title says a mark is
  set — and `d` then compares the marked block with the selected one, **older first** whichever you
  marked first. `m` on the marked block again clears the mark.
- **In History or Block detail with nothing marked**, `d` compares the block with **the previous
  block in this ref's history**: the next older row in the lineage you have loaded. On the oldest
  row loaded, stikk says there is no previous block in view rather than fetching more.
- **In Changes**, `d` compares the worktree with **what `commit` would author against** — the
  sealed tip plus anything queued, and the heading says how many queued commits that is.

Each differing path gets a section under prikk's own status word (`added`, `deleted`, `modified`,
`renamed`, `mode`, `binary`), with the path as a name. **Every line of a hunk is quoted**, with the
line's own ending inside the quotes, so a trailing space, a CRLF and a missing final newline are all
visible — two lines that differ in any byte never look the same. prikk's `@@` header and its
`\ No newline at end of file` note are prikk's words and are not quoted. A rename names both paths,
a `mode` entry shows both modes, and a `binary` entry shows both sizes and content ids — never
bytes. `↑`/`↓` move between entries and `v` shows prikk's own fields for the selected one.

**stikk computes no part of a comparison**: it does not renumber, re-hunk or infer a rename. Where
prikk says its hunks are not the shortest edit script, stikk says so too. Below prikk 0.46, Compare
says that stikk compares from 0.46 on and asks prikk nothing.

**Comparing two different refs is not built yet**, and is on the roadmap.

## Capability and signing

What you can do is *derived* from which signing keys are ready in your environment, not from any
account stikk keeps:

**Where stikk gets the answer depends on your prikk**, because prikk changed how signing keys are
configured at 0.40:

| your prikk | stikk asks |
|---|---|
| **≥ 0.41** | prikk itself, with `prikk key status` — including whether the key matches what this repository already records |
| **0.40 exactly** | nothing it can trust. 0.40 moved keys to a key directory and shipped no way to ask about them, so stikk reports signing readiness as **unknown**, says why, and points at 0.41 |
| **≤ 0.39** | the environment, as presence only: `PRIKK_<ROLE>_KEY_ID` **and** `PRIKK_<ROLE>_SEED` both set |

What that readiness grants is the same on every version:

- No keys → **Viewer** (every read surface).
- AUTHOR ready → **Author** (can queue commits and rollback drafts).
- MAINTAINER ready → **Maintainer** (can seal, merge, publish refs and tags).
- `STIKK_READ_ONLY=1` forces Viewer regardless of keys.

**On prikk ≥ 0.40, a `PRIKK_*_SEED` you exported previously is no longer what signs** — 0.40 refuses
it and 0.41 ignores it in favour of the key directory. stikk says so when it sees one still set,
because it is easy to believe otherwise.

stikk reads only the **presence** of these variables. It never reads a seed's value — prikk reads
seeds itself when it signs. Your keys never enter stikk.

## Launcher utilities

```sh
stikk --version           # print the stikk version
stikk config path         # where stikk's config and state live (never inside a repository)
stikk config check [file] # validate the config file; exits non-zero on a notice, for CI
```

stikk's own files live in user scope (following the XDG convention: `~/.config/stikk`,
`~/.local/state/stikk`), never inside a repository — so a repository stays byte-identical whether or
not stikk ever opened it.
