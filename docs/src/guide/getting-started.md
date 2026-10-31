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

stikk requires prikk **≥ 0.28** and is validated through **0.55.0**. A prikk below the floor degrades
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

**To compare two refs**, open the ref picker with `b`, put the cursor on one ref and press `m` to
mark it, then move to the other and press `d`. The marked ref is the left side and the selected one
is the right, and the heading says which is which — two refs have no lineage order, so stikk states
the one you chose rather than inventing one. Beside each ref the heading names the **block** prikk
resolved it to, because a ref can move and a block cannot; `r` re-runs the comparison against
wherever the refs point then, and those block ids change with it.

A second `m` on the marked row clears the mark, and the picker's title says when one is set. The
mark survives closing and reopening the picker — that is where you choose both halves of a pair —
and is forgotten the moment you focus a different ref, because a stale pair is worse than none.

Every ref the picker lists can be a side: branches, tags, and received refs (`remotes/…`), which
prikk compares without adopting them. **The branch you are on before its first seal cannot**: it has
no block for prikk to compare, so `m` and `d` there say so and point you at `w` then `d`, which
compares your worktree with what `commit` would author against.

## Getting an earlier version back

You broke a file and want the version from before. Open its block in History, and stikk writes it
out for you — **outside your repository**, so nothing you have now is touched. Needs prikk 0.54 or
newer.

Open **History** with `Enter`, put the cursor on the block you want and press `Enter` again: that
block's detail lists its patches and then **its files**. `↓` walks down through both.

- **`F`** on a file saves **that file**, as it was at that block.
- **`A`** anywhere in the block saves **every file at it** into a new folder.

Either way stikk suggests a destination beside your repository — `a-f687c8ff7394.txt` for a file,
`<your repository>-f687c8ff7394` for a folder — and you can type another. The card then names the
block, what is being saved, how big it is, and exactly where it goes. Nothing is written until you
press `Enter`.

**stikk will not write inside your repository**, because the copy would sit in your worktree as
untracked files and your next commit would capture them; and it **will not write over anything**
that is already there. Both are refused before prikk is asked, with a sentence saying what to do
instead.

Two things the card tells you when they apply. Saving **one** file keeps its bytes but not its
execute permission, so a script comes back needing `chmod +x` — saving all of them keeps it. And
where prikk has no trusted checkpoint in reach it replays the files from the start of history and
says so: the copy matches that history, but nothing has checked the history itself. `V` does that.

**Nothing in your repository or your worktree changes.** To actually use what came back, copy what
you want into your worktree yourself and commit it — which is also the honest answer while stikk
cannot take a change back for you.

### Going back to an earlier block

The last section is for **looking at or copying** an old version: `F` and `A` write outside your
repository and change nothing. This one is for **working from** an earlier block — having its
files in your worktree and carrying on from there.

prikk writes no block id into a worktree: a checkout needs a branch, and a branch cannot be
created straight from a block id. **The route that works is three steps, each one a key stikk
already has:**

1. **`t`** on the block — in History, or in its detail — tags it. Give the tag a name you can
   live with (see below).
2. **`n`** in the ref picker (`b`), with that tag selected, creates a branch at it.
3. **`s`** in the ref picker, with that branch selected, switches to it.

**What each step writes.** The tag and the branch are **published names**: they are part of the
repository and anyone you share it with will see them. The switch is the step that touches your
files — its card lists **every file it will write and every file it will delete** before anything
happens, so read that list rather than the branch name. Afterwards your worktree is that block's,
and stikk says which branch you are on now.

The switch **refuses** while you have uncommitted changes, or queued work for another branch, and
the card says which. Commit or seal first, or switch somewhere else.

**What it leaves behind: the tag.** stikk offers no way to remove a tag, and prikk's own deletion
of one does not exist — so the name you choose is the name that stays. Something like
`tags/before-the-rewrite` is kinder to read later than `tags/tmp`.

**Going forward again** is the same third step: select your original branch in the ref picker and
press `s`.

**If what you want is to take a change back**, see [Undoing a block](#undoing-a-block) below. That
one records the reversal as a new change instead of moving you to an older one.

## Undoing a block

You sealed something and it was a mistake. **`U` puts back what that block changed**, as a new
change on top of it. Needs prikk 0.54 or newer.

Open **History** with `Enter`, put the cursor on the block you regret, and press **`U`**. It works
from the block's detail screen too.

1. **The message.** stikk fills in `Undo: <the block's own message>`; edit it if you like. This is
   what `prikk log` will show for the undo.
2. **The card.** It lists every file, one line each, with what will happen to it: *put back*,
   *remove*, *rename back*, *put back the permissions*. Read that list — it is the whole of what
   will change. `Enter` does it; `Esc` walks away and changes nothing.
3. **Seal it.** The undo is an ordinary commit, so press **`S`** when you are ready for it to
   become part of the history.

**Nothing is erased.** The block you undid is still there, and so is the undo — History shows
both. That is the difference from [going back to an earlier block](#going-back-to-an-earlier-block),
which moves your worktree to an older block and records nothing at all.

**When stikk will not do it**, each with a sentence saying what is in the way:

- **your worktree is not clean** — commit or remove your changes first. Two reasons, not one: the
  undo must not overwrite what you are working on, **and a new file lying around would be recorded
  as part of the undo**, because a commit here captures the whole worktree. So an untracked file
  anywhere stops it, not only one in the way;
- **work is waiting to be sealed** — seal it, so the undo is recorded after it;
- **a later block changed one of the same files** — undoing this one would throw that later change
  away, so stikk names the file and stops;
- **the block is a merge**, or **the first block on the branch** — there is nothing single to go
  back to;
- **something is already where the undo would put a file back** — stikk names it rather than
  replacing something it did not put there. This looks at your folder itself, so it sees a file
  even where `.prikkignore` hides it;
- **a file the undo would put back is named in `.prikkignore`** — the commit would not record it,
  so the undo would not be complete. Remove that line first;
- **the block changed `.prikkignore` itself**, and would put files back as well — undo that block
  on its own first, so the rules are settled before the files;
- **the block changed no file at all** — there is nothing to put back;
- **you are looking at another branch's history** — undo works on the branch you are on; switch to
  it first.

If something fails part-way — prikk refuses the commit, or a file cannot be written — stikk stops
at once, **commits nothing**, and tells you exactly which files it had already changed. `w` shows
them, and you decide what to do next. It never tries again by itself.

## Sharing work

Two people share prikk history by **bundle files**. One exports a branch to a file, sends it any
way they like — prikk moves no bytes over a network — and the other checks it, sees what it would
change, and imports it. Needs prikk 0.53 or newer.

**To export**, open the ref picker with `b`, put the cursor on one of your own open branches and
press `E`. stikk suggests a name beside the repository (`<branch>-<short tip>.bundle`), which you
can edit. **stikk will not write inside the repository** — the bundle would sit in your worktree
as an untracked file, and your next commit would capture it — and **will not write over a file
that is already there**. Tags, received refs, closed branches and a branch with nothing published
are each declined, with the reason, before anything runs.

**To open a bundle someone sent you**, press `i` anywhere a repository is open and give the path.
stikk shows two things:

- **prikk's verification**, word for word, including prikk's own caveat that it checks structure
  only — *a verified bundle is not yet a trusted one*;
- **what importing would change**, against your branch of the bundle's own name: new here,
  nothing in common with yours, already held, would bring yours forward, or both sides have moved
  on — applying cleanly, conflicting (with prikk's reason), or undetermined. The files that would
  change are listed, and so are the key ids recorded in the file **with prikk's note that
  recording a key is not trusting one**.

Nothing is written by opening a bundle. If prikk will not read the file, you get prikk's reason
and nothing else.

**To import it**, press `I` on that view and confirm. The bundle is stored as a **received
branch** — `remotes/<name>`, read-only history beside your own. No branch of yours is created or
moved, nothing is written to your worktree, and nobody's signing key is trusted by it. Afterwards
stikk takes you to that branch's history, which is where what arrived can be read.

**stikk declines two imports prikk allows.** If the bundle is one you already received, importing
changes nothing and stikk says so. If it is *older* than what you received — a stale file in a
downloads folder is the ordinary way to get one — prikk would quietly move the received branch
back to the older block and report an ordinary import; stikk declines instead and tells you why.
Where your received branch is longer than stikk reads, it says it cannot tell rather than guess,
and names `prikk bundle import` for doing it anyway.

**Taking received work into a branch of yours is not in stikk yet.** A received ref cannot seed a
branch, and adopting one is a merge: `prikk merge --from remotes/<name>` does it today.

### Whose key sealed it, and what that is worth

A bundle records the key ids that sealed what is inside. Beneath each one stikk says which
adopted key, if any, that id **names** — *names a key this repository trusts*, *names your key*,
or *not trusted by this repository* — with prikk's own note still beneath them: recording a key
is not trusting one. Needs prikk 0.54 or newer; below that you get prikk's ids and note alone.

**A name is not a signature, and the card says so.** A bundle records the id its sealer chose
and nothing to check it against, so anyone can seal under an id you trust and the preview will
still read clean. **Import it, then press `V`:** verifying is where prikk checks each signature
against the key trusted here, and a seal that does not verify fails the check.

**None of this stops an import.** An untrusted sealer is a fact about what you are looking at,
not a refusal — received branches exist precisely so history you have not vouched for can sit
beside your own, where `V` can check it.

**`T`** shows the keys this repository trusts, anywhere a repository is open: each key's id and
public key, with your own marked. Adopting a key means prikk accepts that key's signatures on
what it signed; it never lets the key move one of your branches. Where nothing is adopted, the
view says so and gives prikk's own command — and the one caution worth repeating: **get the
public key from its owner, not from a bundle they sent.**

### Adopting and removing keys

From the Trust view, **`a`** adopts a key and **`x`** removes the one you have selected. Each is
previewed and confirmed by **typing the key id**, which is stikk's most deliberate confirmation:
nothing here happens by a stray `Enter`. Neither needs a signing key of your own. Needs prikk
0.54 or newer.

**Adopting** asks for the public key first, then for the id it signs under — prefilled with the
id prikk would give that key, which you can change if its owner signs under another. **Get the
public key from its owner**, not from a bundle they sent: a bundle names a sealer and proves
nothing, which is the whole reason this page says so twice.

**If the id you give is already trusted under a different key, stikk stops and shows you both.**
Two keys cannot share an id, because an id travels in every signature. If its owner really has a
new key, they must sign under a new id and you adopt it under that one. If they did not tell you
they changed keys, treat the key you were given as someone else's.

**Removing** is the one to read slowly. If the key you remove sealed blocks in this repository,
**Verify will fail them** until the same key is adopted again under the same id — stikk cannot
say how many blocks that is, because prikk does not report which key sealed each one, so the card
says *if*. Nothing is lost: adopt the key again, under the same id, and Verify passes again. And
removing **your own** key means you cannot seal here until it is adopted back. prikk keeps at
least one trusted key, so the last one cannot be removed at all.

After either, stikk shows prikk's own words and the list as it now stands. After a removal, press
**`V`** to see what Verify says.

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

## What stikk remembers

stikk reopens each repository where you left it: **the ref you had focused, which of Orientation,
History, Changes and the Queue was open, and whether the untracked filter was on.** Nothing else —
no file contents, no object ids, no messages. Names and flags.

**Everything remembered is re-checked against prikk before you see it.** If the branch you were on
has been closed or is no longer listed, stikk opens where it would have opened anyway and says so
in one line, rather than showing you a branch that is not there. Verify never reopens by itself: it
scans the whole repository, and that is not something a launch should start on your behalf.

The records live beside stikk's other files — `stikk config path` prints where — in a `sessions/`
directory, one small file per repository, **never inside a repository**. **Deleting `sessions/` is
always safe**: you lose your place and nothing else.

To leave no trace of a launch, set `STIKK_PRIVATE=1`: stikk reads no record and writes none for
that run. (`STIKK_READ_ONLY=1` is a different thing — it forbids repository changes, and still
remembers where you were.)

## Launcher utilities

```sh
stikk --version           # print the stikk version
stikk config path         # where stikk's config and state live (never inside a repository)
stikk config check [file] # validate the config file; exits non-zero on a notice, for CI
```

stikk's own files live in user scope (following the XDG convention: `~/.config/stikk`,
`~/.local/state/stikk`), never inside a repository — so a repository stays byte-identical whether or
not stikk ever opened it.
