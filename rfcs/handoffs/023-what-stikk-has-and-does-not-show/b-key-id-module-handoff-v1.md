# Handoff B — the key-id module (v1)

**Companion to:** [RFC 023](../../accepted/023-what-stikk-has-and-does-not-show.md) (Accepted
2026-09-12), **F3**. Handoff A has landed.
**Design items:** `FL-05` step 5, `FL-06` (amended here), `FL-10`, `FR-104`, `C-I1a–e`, `SEAM-06`,
`LC-13`, `TS-04`, `C-S2`.

> **`FL-05` has required this since before 0.4.0 and RFC 014's review named where it should land:**
> *"reading a key id belongs in its own module… it will be wanted again for the MAINTAINER key id in
> the seal ceremony and in Trust & Keys. Build it once, there."* RFC 016 was that site and **I wrote its
> handoff without re-reading that review**, so the instruction was never carried. Recorded in 0.4.0's
> proposal as my error, and this is it being carried.
>
> **A key id is not key material.** `C-I1` is presence-only for **seeds**; an id is an identifier prikk
> itself prints. Reading it is permitted. Reading it *in the wrong place* is not — see §2, which is most
> of this handoff.

---

## 1. Scope

**In:** the module and its guard (§2); where the id surfaces (§3); the `FL-06` amendment (§4).

**Out:** **`C-S2`** — see §5, which is a trap rather than a boundary. Trust & Keys' view (`FR-104`);
this builds what it will read. Three-valued `Ready` **constructed** (`FR-103`'s increment, with
`C-S2`). Anything that reads a seed, ever.

## 2. Two modules, two guards, pointing opposite ways *[the design]*

**You cannot put this in `env.rs`, and the reason is a test rather than a preference.**
`env/tests.rs::env_module_never_materializes_a_variable_value` scans `env.rs`'s **source** for
`env::var(`, `into_string`, `to_string_lossy`, `to_str(`, `to_owned(` — **precisely the calls reading an
id requires.** That guard is correct and is not to be weakened, narrowed, or excepted: `env.rs` is the
module that must never materialize a value, and an id is a value.

**So: a new sibling module, with the mirror-image invariant and its own source-level guard.**

| Module | May read | Guard asserts |
|---|---|---|
| `env.rs` | presence only, of anything | **no value-materializing call** (existing, unchanged) |
| the new one | `PRIKK_*_KEY_ID` **values** | **no reference to `*_SEED` at all** — not the constant, not the string |

**Neither guard weakens the other, and each is stronger than a comment.** Follow `TS-04`'s existing
shape — `include_str!` on the module's own source, comment lines stripped, as both `env.rs`'s guard and
`cli_backend`'s `C-I1e` scan already do.

**`Readiness` cannot carry the id.** It is `#[derive(…, Copy, …)]` and a `String` field would remove
`Copy` from a type threaded through every capability check — a breaking change with a wide ripple, for a
field most call sites do not want. **The id travels separately**, to the one or two places that display
it.

**There is no read-time race, and do not build machinery against one.** stikk spawns prikk, which
**inherits stikk's own process environment** — the id stikk reads and the id prikk signs with come from
the same place in the same process. Nothing can change between preview and execute. *(This is unlike
repository state, which RFC 003's change token exists for. Say so in the doc comment so the next reader
does not go looking for the equivalent.)*

**Absent is a state, not an error.** `PRIKK_AUTHOR_KEY_ID` unset is exactly the `NotReady` case
`capability_gate` already refuses on. The module returns an absence; it does not invent a placeholder,
and no caller renders `"(unknown)"` where an id goes.

## 3. Where the id surfaces

- **`FL-05` step 5 — commit's confirmation**, which is the requirement that has been unmet. It shows
  `Consumes: AUTHOR`; it must also name the AUTHOR key id that will sign.
- **`FL-06` — seal's confirmation** (§4), the MAINTAINER id.
- **`FL-10`** inherits `FL-05`'s step by reference; nothing to build until Rollback exists.

`ConfirmationSummary` already carries `String` fields and is not `Copy`, so it is the natural home —
**one field, populated by whichever operation has an id to name.** Do not add two.

**It is inert display** (`C-T2a`): a key id is repository-adjacent text, rendered through `text::inert`
like every other, and it is never a next-step, never a glossary trigger.

## 4. `FL-06` — amended here, and why that is not scope creep

**RFC 016 decision 3 removed a *typed MAINTAINER key id confirmation step*** — "two deliberate acts, not
three" — and the amended `FL-06` text does not mention the id at all. **Read literally, `FL-06` does not
currently ask for this.**

**Amend it to ask for the display**, and record the distinction in the amendment: RFC 016 removed an
**act**, not the **information**. A ceremony that freezes patches into permanent, signed history, behind
a tier-3 confirmation and a separate acknowledgement, should name **which key is about to sign** — and
showing it costs no extra keypress, which was decision 3's actual concern.

**This is a design amendment and it is mine, so it is written here rather than left to you to infer.**
If you think the seal confirmation reads worse with it, say so in the review request — you will have it
on screen and I will not.

## 5. `C-S2` — the trap, named because you will be standing next to it

Once the id is on screen, the obvious next thought is: *prikk's docs publish `dev-author` and
`dev-maintainer` as example key ids — warn if the user's id matches.*

**Do not.** `C-S2` says *"by pattern of the **public value**"*, and **a key id is a label, not a value**.
Anyone may legitimately name a real key `dev-author`; flagging it would be a confident-wrong security
warning, which is worse than none. `C-S2`'s real mechanism is comparing **adopted public keys** against
values derived by prikk's own `key public` — RFC 023's Q1 resolution — and it ships with `FR-103`'s
increment.

**If you find yourself wanting the warning, that is the trap working.**

## 6. Tests

- **The guard**: the new module's source contains no `*_SEED` reference. Prove it fires — add one
  temporarily, watch it fail, remove it, and say so.
- **`env.rs`'s existing guard still passes, unmodified.** `git diff` on `env/tests.rs` should be empty;
  if it is not, the id landed in the wrong module.
- **Presence and absence**: an id present is returned; absent is an absence, not a placeholder.
- **Render, at 80×24**: commit's confirmation shows the AUTHOR id; seal's shows the MAINTAINER id.
  Assert the id's **actual text**, not that some field is non-empty.
- **Hostile input**: an id containing control sequences renders inert (`C-T2a`) and forges no chrome.
- **No seed anywhere**: the existing workspace seed guards still pass.

## 7. Gates

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo build --examples -p stikk-tui --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --locked
cargo package --workspace --exclude stikk-real-binary --locked
mdbook build docs
cargo deny check
```

**The real-binary suite: run it.** Unlike Handoff A, this touches the seam — and `Fixture` sets
`PRIKK_*_KEY_ID` for real, so the suite exercises the module against genuine environment values at both
ends. If it tells you nothing, say so; but it is the right call to run here and not there.

## 8. Acceptance criteria

1. A new module reads `PRIKK_*_KEY_ID` values; **`env.rs` and its guard are untouched**.
2. The new module has its own source-level guard asserting no `*_SEED` reference, **proven to fire**.
3. `Readiness` still derives `Copy`; the id does not travel on it.
4. Commit's confirmation names the AUTHOR key id; seal's names the MAINTAINER id; both render-tested at
   80×24 on the id's actual text.
5. `FL-06` amended to require the display, recording that RFC 016 removed an act rather than the
   information.
6. Absence is an absence — no placeholder text where an id goes.
7. **No `C-S2` example-id warning anywhere** (§5).
8. Ids render inert; hostile input forges no chrome.
9. All eight gates green; the real-binary suite run and its result stated.
10. Nothing tagged or published.

## 9. Submit

Package to `.git-exclude/review-request/023-b-key-id-module/review-request-v1.md`.

**Lead with the two confirmation screens at 80×24** — commit's and seal's, ids visible. They are the
requirement.

**Then tell me whether seal's confirmation reads better or worse with the id in it** (§4). I amended
`FL-06` on reasoning; you will be the first person to see it rendered, and that is the better evidence.

**Push once approved.**
