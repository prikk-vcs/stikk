# RFC 068 — Every advisory in the tree is seen

**Status.** **Proposed 2026-11-24** by the architect. **Added to 0.22.0's scope by the project owner, 2026-11-24**
(*"Yes. Authorized."*). **No open question.**
**Measured** on `main` at `55329db` with cargo-deny 0.20.2 and the advisory database of 2026-11-24, in a scratch
worktree. Found while measuring RFC 065 against knotra's `deny.toml`.
**Tracks.** RFC 020 (the supply-chain gate); RFC 065 (the GUI tree, where the same question returns).
**Touches.** `deny.toml`, `Cargo.lock` (one crate), `CHANGELOG.md`.

## Summary

**stikk's supply-chain gate does not see every advisory in its own tree.** `deny.toml` leaves cargo-deny's defaults for
*unsound* and *unmaintained* advisories in place. With those defaults, cargo-deny reports an advisory of either kind only
for a crate the workspace depends on directly. Two unsound advisories sit deeper, in the terminal front end's tree as
shipped today, and the gate passes without naming them.

This RFC makes the gate report every advisory at any depth. It fixes the one that can be fixed, and records the one
that cannot, with the reason it does not reach stikk and the condition for removing the record.

## Findings

### F1 — what the gate reports, and what it does not

`deny.toml`'s `[advisories]` section is `ignore = []` and nothing else.

| Setting | Result on `55329db` |
|---|---|
| as committed | `advisories ok` |
| with `unsound = "all"` and `unmaintained = "all"` | **`advisories FAILED`**, with two `error[unsound]` |

**The two:**

| Advisory | Crate | How it reaches stikk | Fixed in | Can we take the fix? |
|---|---|---|---|---|
| **RUSTSEC-2026-0486** (2026-10-27): *"xattr functions mark unwritten bytes as initialized when given a zero-length buffer"* | `rustix` 1.1.4 | crossterm 0.29 → ratatui-crossterm → ratatui → `stikk-tui` | `>= 1.1.6` | **yes.** `cargo update --dry-run -p rustix` moves `rustix` 1.1.4 → 1.1.6, and nothing else |
| **RUSTSEC-2026-0253** (2026-05-12): *"Potential use-after-free due to lack of panic safety in `LruCache::pop()`"* | `lru` 0.16.4 | ratatui-core 0.1.0 → ratatui → `stikk-tui` | `>= 0.18.2` | **no.** ratatui-core 0.1.0 requires `lru` 0.16 |

**No unmaintained advisory** is reported for today's tree with `unmaintained = "all"`.

### F2 — whether the `lru` defect can reach stikk

The defect needs **`LruCache::pop()`** to be called while a stored key's `Drop` panics.
- **ratatui-core calls only `get_or_insert` and `resize`** on its cache (`layout/layout.rs:303`, `:720`). It never
  calls `pop`.
- **The cache's keys are `(Rect, Layout)`** (`layout.rs:36`): plain data, whose `Drop` cannot panic.

**So the path the advisory describes is not reachable from stikk.** The advisory still stands against a crate in the
tree, and the gate should say so in writing, not by silence.

### F3 — what the GUI will bring

Measured for RFC 065 on the Snora prototype's tree. **With both settings at `"all"`:**
- the same `lru` advisory, through iced's text cache;
- five unmaintained advisories: `paste`, `ttf-parser`, `zbus_names`, `zvariant` and `zvariant_derive`.

That is 0.23.0's question, when a GUI crate joins the workspace. **This RFC makes sure the gate will see them.**

## Decisions

1. **`deny.toml`'s `[advisories]`** gains `unsound = "all"` and `unmaintained = "all"`, each with a comment that says
   why: an advisory deep in the tree is still in the binary we ship (RFC 068).
2. **`rustix` moves to 1.1.6** by `cargo update -p rustix`. **No other line of `Cargo.lock` may move.**
3. **RUSTSEC-2026-0253 is recorded in `ignore`**, as `{ id, reason }`. The reason says, in one sentence each:
   - the path (ratatui-core 0.1.0 → ratatui → `stikk-tui`);
   - why it is unreachable (ratatui calls only `get_or_insert` and `resize`, and its `(Rect, Layout)` keys cannot
     panic on drop);
   - **what retires it:** a ratatui release on `lru >= 0.18.2`, which the next ratatui upgrade must check.
4. **Nothing else changes.**
   - The supply-chain workflow keeps its triggers. Dispatch only is RFC 020's open question for the owner, not this
     RFC's.
   - `yanked` keeps its setting.
   - No other advisory is ignored.

## Tests and evidence plan

| Question | Level | How |
|---|---|---|
| Does the gate now see a deep advisory? | local, recorded | `cargo deny check advisories` with decision 1 alone (before the update and the ignore): `FAILED`, naming both advisories |
| Is the tree then clean? | local, then CI | with all three decisions: `advisories ok`; the dispatched supply-chain run green |
| Did only `rustix` move? | review | `git diff Cargo.lock`: the `rustix` entry, and nothing else |
| Does stikk still work? | gates, real binaries | gates 1–8, the local suite at 0.28.0 + 0.58.0, and the full matrix after the push |

## Delivery

**One handoff, after acceptance.**

## Changelog

`### Security`: *the supply-chain check now reports unsound and unmaintained advisories at any depth of the dependency
tree. `rustix` is updated to 1.1.6 (RUSTSEC-2026-0486). RUSTSEC-2026-0253 (`lru`, through ratatui) is recorded as not
reachable from stikk until ratatui moves to a fixed `lru`.*
