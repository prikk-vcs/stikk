# RFC 018 — Post-0.4.0 correctness sweep: the files nobody greps

**Status.** **Done** — shipped in **0.4.1**, 2026-09-06 (`e6f0388`, `8e0847a`, `5d25f89`, `11ce355`), after three review rounds. Accepted by the project owner 2026-09-06.
**Deferred, carried forward:** the Glossary's **wrap + scroll**, as one increment — wrapping alone was tried and reverted, because without a way to scroll past what wrapping pushes down it turned truncated-but-present into absent (as few as one of eleven terms reachable at 80×24). Proposed the same day, **hours after 0.4.0 published.** Opened on a finding made while
sequencing 0.5.0: **stikk's Glossary tells users it never writes their repository, four lines above
the keybindings for commit and seal.** Two further stale claims found by the sweep that followed.
**Proposes 0.4.1** — non-breaking, string-and-docs only.
**Tracks.** `C-T4a–e` (no confident-but-wrong picture), `T-T4`, `ER-02`, `NFR-R03` (version honesty),
`FR-111`, RFC 011 (patch position).
**Touches.** `crates/stikk-tui/src/overlay.rs`, `ROADMAP.md`.

## Summary

0.4.0 is the release where stikk writes. It shipped with a screen saying it does not.

None of the three findings below is a logic defect; all three are **claims that were true when written
and became false when the product changed**. That is the same failure mode as RFC 017's dead classifier
arms and RFC 015's stale ceiling, and this project has now hit it four times. The difference here is
that one of them is a **safety claim**, and it shipped.

## Findings

### F1 — the Glossary says stikk never writes your repository *[shipped in 0.4.0; the serious one]*

`overlay.rs:234`, the first line a user sees on pressing `?`:

```
  stikk reads prikk; it never writes your repository.
```

**Four lines below it, in the same rendered panel:**

```
  C            commit worktree changes
  S            seal the active WAL
```

The false claim and its own refutation are in one panel, visible together, at every terminal size.

**Why this is the worst kind of stale string this project can ship.** It is not a version number or an
out-of-date caveat — it is a **statement about what stikk will do to a user's repository**, offered as
reassurance, in the release that made it untrue. A user who reads it and then presses `S` has been
told, by stikk, that the thing about to happen cannot happen. `ER-02` and the whole `C-T4` family exist
to prevent exactly this sentence.

**How it survived.** RFC 016 edited this very function — `S` in the key list is *its* line — and
neither the implementer nor I read four rows up. Every review in this cycle asked what a render shows;
none asked what a render **still claims**.

### F2 — "see Glossary → Trust & Keys" points at a keybinding table

`app.rs:1178` renders `{detail} — see Glossary → Trust & Keys` for every `NotReady` routed at
`Target::TrustKeys` — including 0.4.0's new trust refusal.

**The Glossary has no Trust & Keys section.** It has a section titled **`Keys`**, and that section is
the *keyboard shortcut* list. A user following this pointer after a signing refusal lands on a table
telling them what `j` and `k` do.

`Target::TrustKeys` is also a no-op in `activate_view` — correct, since the view is unbuilt (`FR-104`),
but it means the pointer is the *only* thing a user gets, and it misdirects.

### F3 — `ROADMAP.md` was outside every version grep

RFC 015 shipped a stale ceiling because its grep covered `README.md docs/`. RFC 017 widened the scope
to `crates/`, `examples/`, `README.md`, `docs/`, `CHANGELOG.md` and caught two more. **`ROADMAP.md` was
in neither scope**, and it holds:

- *"currently `>= 0.28`, validated through **`0.30.0`**"* — **three releases stale.**
- `UD-01`'s row: *"patch messages are discarded"* — **retired at prikk 0.32** (RFC 015).
- `UD-09`'s row: *"no per-patch content, no patch-id enumeration"* — **narrowed at 0.32**; patch ids
  are enumerable for messaged patches, content is not.

The `UD-08` row directly above shows the correct pattern (`**retired at prikk 0.29**`), so the file
knows how to record a retirement — it simply was never re-read.

## Decisions

1. **Fix F1 by deleting the claim, not by qualifying it.** A hedged version ("only writes when you
   confirm") invites the same drift the next time capability changes. The Glossary header should say
   what stikk *is*, not make a promise about what it will not do. What it must not do is assert a
   safety property that a later release can silently invalidate.
2. **Fix F2 by pointing where the answer actually is.** Until `FR-104`'s Trust & Keys view exists, the
   pointer names the real, present thing — the refusal's own gloss and the `glossary:` code line
   already on the card — or it says nothing. **A pointer to a place that does not exist is worse than
   no pointer**, because it costs the user a navigation to find that out.
3. **Fix F3, and put `ROADMAP.md` in the standing grep scope.** The scope has now been widened twice,
   reactively, after each miss. Write the scope down once, in the handoff spec, as *every tracked
   `.md` and every `crates/**` source file* — an exclusion list is shorter and safer than an
   inclusion list.
4. **Ship as 0.4.1.** Non-breaking, no API change, no behaviour change beyond the strings themselves.
   RFC 011 puts this in the patch position.

## What this RFC does not do

**It does not build the Trust & Keys view** (`FR-104`) — that is 0.5.0 work with its own design.
**It does not touch the key-id gap** carried out of 0.4.0. **It adds no test for "is this claim still
true"**, because I do not know how to write one; see below.

## Consequences, and the one that matters

Three of the four stale-claim findings this project has made were caught by a **grep whose scope had to
be widened after it missed something**. F1 was not caught by a grep at all — it was caught by reading a
screen while thinking about something else entirely.

**This project has no mechanism for "a sentence that was true when written."** Tests pin behaviour;
gates pin format; captured fixtures pin upstream. **Nothing pins a claim.** F1 sat four lines from its
own refutation, in a file two reviewers read that week, and shipped.

I am not proposing a mechanism here because I do not have a good one — a lint for prose is not
obviously possible, and a checklist is what people skip. **But it should be named as an open problem
rather than closed with a wider grep**, which is the third time that answer would have been given.
RFC 019 or later may do better.
