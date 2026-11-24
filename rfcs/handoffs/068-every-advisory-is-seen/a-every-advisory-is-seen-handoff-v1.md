# Handoff A — RFC 068: every advisory in the tree is seen (v1)

**Companion to:** [RFC 068](../../done/068-every-advisory-is-seen.md), **accepted 2026-11-24**. **Read the RFC
first**: F1–F2 and decisions 1–4. **Covers:** all of it. **0.22.0, added by the owner 2026-11-24.** **Not a release.**

> **What this increment is.** The supply-chain gate starts reporting unsound and unmaintained advisories at any depth.
> The one that can be fixed is fixed (`rustix` 1.1.6), and the one that cannot is recorded, with why it does not reach
> stikk and what retires it.
>
> **Sanctioned:** `deny.toml`, `Cargo.lock`'s `rustix` entry, and `CHANGELOG.md`. **Anything else, stop and report:**
> another lock line moving, another advisory appearing, or a gate going red. **No branch.**

---

## 0. Evidence plan

| Question | Level | Evidence in the request |
|---|---|---|
| Does the gate now see a deep advisory? | local, recorded | `cargo deny check advisories` after commit 1's `deny.toml` change **alone**: `FAILED`, naming RUSTSEC-2026-0486 and RUSTSEC-2026-0253, and nothing else |
| Is the tree clean after the fix and the record? | local | the same command at the head: `advisories ok` |
| Did only `rustix` move? | review | `git diff Cargo.lock`: the `rustix` entry only (version and checksum) |
| Does stikk still work? | gates, real binaries | gates 1–8, and the local suite at 0.28.0 + 0.58.0 |
| CI | after the push | CI, Docs, the dispatched supply-chain run, and the full-matrix suite |

## 1. Commit 1: the gate sees everything, and the tree meets it

1. **`deny.toml`, `[advisories]`:** add `unsound = "all"` and `unmaintained = "all"`, with a comment: *an advisory deep in
   the tree is still in the binary stikk ships; cargo-deny's default reports these only for direct dependencies
   (RFC 068)*.
2. **Run `cargo deny check advisories` now,** before steps 3 and 4, and keep its output for the request: the before
   case. If it names anything other than the two advisories, **stop and report**.
3. **`cargo update -p rustix`.** `git diff --stat Cargo.lock` shows one entry changed: 1.1.4 → 1.1.6. If anything else
   moves, revert, and stop and report.
4. **`ignore`:** add `{ id = "RUSTSEC-2026-0253", reason = "…" }`. The reason covers decision 3's three points, in
   plain sentences:
   - the path: ratatui-core 0.1.0 → ratatui → `stikk-tui`;
   - why it is unreachable: ratatui calls only `get_or_insert` and `resize` on its layout cache, never `pop`, and the
     cache's `(Rect, Layout)` keys cannot panic on drop;
   - what retires it: a ratatui release on `lru >= 0.18.2`, which the next ratatui upgrade must check.

   **Check both facts in ratatui-core's source yourself** (`layout/layout.rs:36`, `:303`, `:720`), and say that you did.
5. **`cargo deny check` (all four checks) green.**

One commit: the setting, the fix and the record belong together. The before output in the request is the evidence that
the setting alone fails.

## 2. Commit 2: the changelog

`CHANGELOG.md`, `## Unreleased`, **`### Security`**: the RFC's line.

## 3. The traps

1. **Only `rustix` moves.** `cargo update` without `-p` is out of scope.
2. **One ignore, with its reason.** No blanket setting (`unsound = "workspace"` or `"none"`) and no other ignored id.
3. **The workflow's triggers stay as they are.** Dispatch only is the owner's open question (RFC 020).

## 4. Submit (no branch)

Gates 1–8, all of them (none exempted), and the local suite. Package it to
`.git-exclude/review-request/068-every-advisory-is-seen-a/review-request-v1.md`, with:
- §1.2's before output;
- the head's output;
- `git diff Cargo.lock`;
- the ignore's text.

**After approval:** push, dispatch the supply-chain workflow and the full-matrix suite, and report every run at the
pushed SHA.
