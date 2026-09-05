# Handoff — the repository change token (v1)

**Companion to:** [RFC 003](../../accepted/003-repository-change-token.md) (Accepted 2026-09-05).
Inherits its state.
**Realizes:** the first increment of **0.4.0** — the primitive `OPL-02`'s preview↔execute binding needs,
and therefore the thing every mutation in that release is gated on.
**Design items:** `LC-4` (the change token), `LC-10` (cache validity), `CT-05` (concurrency contract),
`OPL-02` (preview↔execute binding), `FR-106` (external-change awareness), `OP-04` (the
"repository changed outside stikk" notice), `AR-05` (balance, not over-abstraction).

**Read RFC 003's "finding that splits this RFC" first.** Its original scope had two halves; only one is
being built. The fingerprint half is deferred because **prikk deliberately has no repository identity**
and says so as a security property — not a gap to work around.

**This is a breaking increment** (the trait gains a method). Expected in 0.4.0 per RFC 011.

---

## 1. Scope

**In:**
1. A `ChangeToken` value in `stikk-model`, and `Prikk::change_token()` composing it from **calls the
   seam already makes** — no new process spawns.
2. `stikk-core` capturing it around reads, and the `FR-106`/`OP-04` staleness notice.
3. The `DM-02`/`LC-9` **data-model correction** — the fingerprint field is deferred, and the document
   must stop describing it as present.

**Out (do not build here):**
- **The repository fingerprint.** RFC 003 decision 4. Do not derive one from the root block, and do not
  add a field for it "ready for later".
- **`OPL-02`'s preview token itself, and any confirmation machinery.** That is the *next* increment;
  this one supplies the primitive it will stamp.
- **`DerivedViewCache` (`DM-08`).** `LC-10` becomes *implementable* here; it is not implemented here.
- Any mutation, any change to `Command::output()` or the EPIPE guard, any cancellation work.

---

## 2. The token

**Signal set (RFC 003 decision 3), and nothing more:**

- the ref pointers — every `name → RefState id` from `Prikk::refs`;
- the queued-patch **count** and its **target ref**, from `Prikk::orientation`.

Both are already read at open and on every refresh, so `change_token()` composes them rather than
spawning anything. **The worktree marker is excluded on purpose** — it would cost a `worktree-status`
spawn per token, and the Changes view already carries its own worktree data, so a preview built from it
is self-freshening. Do not add it "for completeness".

**Coarse and global, not per-view** (RFC 003 decision 2). An unrelated ref moving *will* invalidate an
unrelated view. That is the intended trade: over-invalidation costs a re-read nobody notices;
under-invalidation is unsafe.

**Shape.** A comparable, cheap-to-store value — an ordered digest of the signals rather than the signals
themselves, so callers cannot be tempted to *interpret* it. It is an opaque staleness marker, not a
repository fact: nothing above the seam should ever branch on its contents. Put it in `stikk-model`
beside the other shared vocabulary; derive `PartialEq`, and make its `Debug` useful without implying
structure worth reading.

**Do not hash the ref *order* incidentally.** `prikk branch list` sorts by name (verified:
`RefStore::list_ref_pointers` sorts before returning), but relying on prikk's sort silently would make
a future ordering change look like a repository change. Sort explicitly on stikk's side.

---

## 3. The seam

Add `fn change_token(&self, repo: &Path) -> Result<ChangeToken>` (`SEAM-02` names it; category
`read-history`). `CliBackend` composes it from its existing `refs` + `orientation` reads. `NullBackend`
gains a `with_change_token` builder so the layers above can script staleness deterministically.

**Adding a required trait method is the breaking change here.** Expected; do not add a default
implementation to avoid it — a default that returns a constant would silently disable staleness
detection for any implementor that forgot to override it, which is precisely the failure this primitive
exists to prevent.

---

## 4. The operation layer, and the honest limit of what this buys

`stikk-core` captures the token alongside each read's result, and compares on refresh. When it differs,
surface `OP-04`'s passive notice — "repository changed outside stikk — refreshed" — through the
existing `present()` path. **Do not invent a new presentation variant**; a `Banner` is what this is.

**State the limit plainly in the code's own docs**, because it is easy to over-claim: this is a
*detection* primitive, not a lock. Between reading the token and acting on it, the repository can change
again — `CT-05` and `NFR-R02` are explicit that stikk holds no lock across think-time, and prikk's own
locking is the real guard. What the token gives is that a preview cannot be *executed* against a
repository that has demonstrably moved since the preview was computed. It does not make the execution
atomic, and no comment or UI string may imply that it does.

---

## 5. The data-model correction

`docs/src/reference/data-model.md` currently describes `DM-02`'s `RepositoryHandle` as carrying "a
**content-derived repository fingerprint** (see LC-9)". It does not, and after this increment it still
will not. Correct `DM-02` and `LC-9` to say:

- session state is keyed by **canonical path** for now;
- the fingerprint is **deferred**, with RFC 003's three reasons (prikk has no repository identity by
  design; deriving one means walking the whole log; it does not exist for a repository with no sealed
  blocks);
- and the protection it was meant to add is **already carried by `INV-5`** — every stored reference is
  re-resolved on load and a miss degrades to a default.

This is the design set correcting itself to match reality, which is the ordering this project requires.
Do not leave `LC-9` describing a mechanism nobody built.

---

## 6. Security surface

- **No new asset, flow, or trust boundary.** The token carries no repository content — it is a digest of
  ids stikk already displays.
- **`INV-1` holds**: the token is not authority. A cache or preview gated on it is still re-derived from
  prikk on use; the token only decides *whether* to re-derive.
- **Do not let the token become an identity.** It changes with ordinary history growth, so it is
  unusable as one — but the temptation to key session state by it will be real once it exists. Session
  state is keyed by path (§5). A comment saying so, where the type is defined, is worth its two lines.
- Record that the review happened (`NFR-S07`); no threat-model edit expected.

---

## 7. Test plan

- **Composition**: the same repository state yields the same token; a moved ref, a new ref, a removed
  ref, a changed queue count, and a changed queue target each yield a different one.
- **Explicit ordering**: two `refs()` results differing only in order yield the **same** token (proving
  stikk sorts rather than inheriting prikk's).
- **Staleness**: a scripted `NullBackend` whose token changes between two reads produces the `OP-04`
  notice through `present()`; an unchanged token produces none.
- **No extra spawns**: assert `change_token` drives no `prikk` invocation beyond the `refs` +
  `orientation` the caller already performs — a counting backend, as RFC 010's handshake-caching test
  did.
- Gates green; state the count delta.

---

## 8. Acceptance criteria

1. `ChangeToken` exists in `stikk-model`; `Prikk::change_token` composes it from `refs` + `orientation`
   with **zero additional process spawns**, proven by test.
2. The token is order-independent by stikk's own sorting, proven by test.
3. A changed repository produces the `FR-106`/`OP-04` notice through the existing `present()` path; no
   new `Presentation` variant was added.
4. **No fingerprint was built**, and no field was added in anticipation of one.
5. `DM-02` and `LC-9` corrected: keyed by canonical path, fingerprint deferred with its reasons, `INV-5`
   named as what carries the protection.
6. The token's docs state that it is detection, not a lock, and that it must not be used as an identity.
7. Gates green; count delta stated; all four demos build.
8. Nothing tagged, pushed, or published.

---

## 9. Submit

Package to `.git-exclude/review-request/003-repository-change-token/review-request-v1.md`.

Call out: the token's concrete shape and why you chose it; the no-extra-spawns evidence; and anything
you found while composing it that contradicts RFC 003 — the last four increments have each turned up
something the RFC got wrong, and that is the most useful part of the package every time.
