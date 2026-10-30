# Handoff C — going back to an earlier block, in the guide (v1)

**Companion to:** [RFC 053](../../done/053-get-files-back.md). That RFC gets an earlier version of a file back
**outside** the repository. This handoff documents how to put a whole earlier block **into the worktree**, with what
stikk already offers. **0.19.0's increment 3**, authorized by the project owner on 2026-10-30
(`.git-exclude/release/0-19-0-scope.md`).
**Independent of RFC 057 and of RFC 056's Handoff B.**
**Not a release.**

> **What this increment is.** A short how-to in the guide, and **one suite leg that proves it**. No product code
> changes.
>
> **No branch.** Run the local gates and the local suite before submitting. After approval, push to `main`, then
> dispatch the suite and supply chain there.

---

## 1. What was measured (prikk 0.55.0)

The script is `.git-exclude/reports/057-scope-0-19/measure-checkout.sh`, and its output is
`measure-checkout-0.55.txt`. Re-run it first.

- prikk **refuses** to write a block id into the worktree: *"… needs a branch … a block id names no branch"*.
- prikk also refuses `branch create --from <block-id>`: *"does not resolve to a published ref"*.
- **The route that works:** `tag create --target <block-id>`, then `branch create --from tags/<name>`, then
  `branch switch`. The worktree then holds that block's files (part 3 of the measurement).

stikk offers each step today:

1. **`t`** on the block in **History** (or in Block detail) tags it, by block id.
2. **`n`** in the **ref picker** on that tag creates a branch at the tag's tip.
3. **`s`** in the ref picker switches to the branch.

## 2. The suite leg (`stikk-real-binary`)

Write `going_back_to_an_earlier_block_through_a_tag_at_the_ceiling`. **It drives stikk's own core functions**, the
ones the three keys call, not prikk directly:

1. The fixture seals two blocks; the second changes one file and adds another.
2. Tag the first block by id, through `refs::create_tag_preview` and `create_tag_confirm_and_execute`.
3. Create a branch from that tag, through `refs::create_branch_preview` and `create_branch_confirm_and_execute`.
4. Switch to it, through `switch::switch_preview` and `switch_confirm_and_execute`.
5. **Assert:** the switch preview names the files it writes and deletes, and afterwards the worktree holds the first
   block's files, byte for byte, with the added file gone. Orientation then reports the new branch as current.

**Also assert what is left behind:** the tag still exists. The guide says so, and the leg is the evidence.

At **0.28**, skip the leg with the usual message: switch needs prikk ≥ 0.46.

## 3. The guide (`docs/src/guide/getting-started.md`)

Add a subsection near RFC 053's *"get a file back"* material: **"Going back to an earlier block"**. Cover these points,
in a newcomer's words:

- **Two ways back.**
  - To **look at or copy** one file or a whole block, use `F` / `A` in Block detail (RFC 053). That writes outside the
    repository and changes nothing.
  - To **work from** an earlier block, use the three steps above. Name the keys and the view each is in.
- **What each step writes.**
  - The tag and the branch are published names.
  - The switch rewrites the worktree to that block, and its preview lists every file it writes and deletes.
  - The switch refuses while you have uncommitted changes or queued work for another branch, and the card says so.
- **What it leaves behind.** A tag you may not want. Tags are published, and stikk offers no way to remove one.
  Choose its name with that in mind.
- **Going forward again:** switch back to your original branch with `s`.

**Do not promise a one-step version.** If prikk later accepts a block id for a branch, a later RFC may add one; the
guide says nothing about it.

## 4. The traps

1. **No product code.** If the leg finds that one of the three steps does not work as described, **stop and report**.
   Do not fix it here.
2. **The guide states only what the leg proves.**
3. **Paste captures as the file holds them, and grep before committing.**

## 5. Submit (no branch)

Run the eight gates, and the local suite at 0.28.0 and 0.55.0. Package to
`.git-exclude/review-request/053-going-back-guide-c/review-request-v1.md`, leading with the leg's result.
