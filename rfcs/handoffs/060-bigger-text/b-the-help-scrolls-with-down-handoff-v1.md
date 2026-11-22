# Handoff B — RFC 060: the help scrolls with ↓ (v1)

**Companion to:** [RFC 060](../../done/060-bigger-text.md), decision 3. **A defect the architect's real-terminal
check found** after Handoff A, and the test gap that hid it. **0.21.0.** **Not a release.** Small.

> **What is wrong.** In a real Alacritty at 2.3× text (102 × 24, Wayland, fullscreened locally with the owner's
> permission), the help overlay opened as RFC 060 says: *"Glossary & Help — lines 1–19 of 204"*, and *"↓ more
> below"*. **Then 60 presses of ↓ moved nothing.** stikk wrote 50 bytes of cursor control after each, and no text.
>
> **Reproduced headlessly** (`.git-exclude/reports/060-bigger-text/glossary-scroll-probe.sh` →
> `glossary-scroll-probe.txt`), at `3e284a8`, before RFC 060, and at `af9da85` alike:
> - ↓ leaves the help where it is;
> - **Enter** scrolls it down: `app.rs:3454`, in `select()`, *"Some(Overlay::Glossary { offset }) =>
>   offset.set(offset + 1)"*;
> - ↑ scrolls it up, in `nav_up` (`:3982`).
>
> The Glossary keeps its own `offset: Cell<u16>`, and so never reaches RFC 060's shared overlay `Scroll`, which is what
> `nav_down` asks first.
>
> **So the help names a key that does nothing**, which is exactly the confusion RFC 060 decision 3 removed everywhere
> else.
>
> **Why no test saw it:** every scrolling test calls an app method (`select`, `nav_down`, or the offset itself), not a
> key. `lib.rs`'s run loop is the only place a key becomes a method, and nothing tests it.
>
> **No branch.** Local gates and suite, then, after approval, push and dispatch the full-matrix suite and supply chain.

## 0. Evidence plan

| Question | Level | Evidence |
|---|---|---|
| Does ↓ scroll the help, and ↑ come back, through the real key path? | headless | the key-path test's name and captures |
| Does every surface that names "↓ more below", or shows a position, move on ↓? | headless | the sweep's name, and the list of surfaces it covers |
| Real terminal | the architect, after approval | not yours |
| Nothing regressed | unit, real binary 0.28.0 + 0.58.0; the full matrix in CI | counts |

## 1. One scroll for every overlay (the fix)

1. **The Glossary moves onto RFC 060's shared overlay `Scroll`**, as `Panel`'s cards did. Its own `offset: Cell<u16>`
   goes, and `render_glossary` sets the shared scroll's extent from what it draws, as `Panel` does.
2. **↓ and `j` scroll down; ↑ and `k` scroll up**, through `nav_down` and `nav_up`'s first branch, the one every card
   uses.
3. **Enter no longer scrolls the help.** The `Some(Overlay::Glossary { offset })` arm in `select()` goes. The help has
   nothing to choose, so Enter does nothing there, as on any overlay with nothing to choose.
4. **The title and the foot line stay RFC 060's**: *"— lines a–b of c"* says where, and *"↓ more below"* / *"↑ more
   above"* says which key.

## 2. Test the keys, not the methods (the gap)

1. **One function turns a key into what it does.** Factor `lib.rs`'s run loop's match, from `keys::dispatch`'s
   result to `App`'s methods, into **`App::handle_key(KeyEvent)`**. The run loop calls it, and tests call it. Nothing
   else changes in the loop.
2. **The help, by keys:** open it with `?`, then press `Down` ten times through `handle_key`, drawing at 102 × 24
   between presses. Each press moves the title's *"lines a–b"* by one, until the end, where *"↑ more above"* shows
   and *"↓ more below"* has gone. `Up` comes back. `Enter` changes nothing. The same with `j` and `k`.
3. **The sweep:** for **every** overlay and view that can draw *"↓ more below"*, or a title position, at 60 × 15,
   drive `Down` through `handle_key` and assert that the position moves, or that the view's own cursor does, where
   that is its meaning. Use RFC 059's `screens()` to reach each one. List what is covered, and why anything is not.
4. **Keep the existing method-level tests.** These are added beside them.

## 3. Traps

1. **One scroll mechanism.** No overlay keeps an offset of its own after this.
2. **Every new test presses keys through `handle_key`.** A test that calls `nav_down` cannot catch this defect.
3. **Paste captures as the file holds them, and grep before committing.**

## 4. Submit (no branch)

Gates 1–3 and 5, and the local suite at 0.28.0 + 0.58.0. Package to
`.git-exclude/review-request/060-bigger-text-b/review-request-v1.md`, with:
- the commit or commits;
- the key-path captures;
- the sweep's coverage;
- the counts.
