# stikk

**stikk is a history browser and workbench for the [prikk](https://github.com/nabbisen/prikk)
version control system** — a terminal (TUI) front-end, with a graphical (GUI) one planned, over one shared
operation layer. The name is Norwegian for *to set a course, to take a bearing.*

Its founding property, from which the whole design follows: **stikk owns no repository authority and
no secrets.** Every repository fact is re-derived from prikk, and prikk — never stikk — reads signing
key material. Its stance mirrors prikk's own: *where prikk refuses, stikk explains.*

## Reading paths

- **New here?** Start with [Getting started](./guide/getting-started.md).
- **Reviewing the design?** The reference section is the full design set, in the order the project's
  workflow produced it: [Requirements](./reference/requirements.md) →
  [External design](./reference/external-design.md) →
  [Internal design](./reference/internal-design.md), with the
  [Data model](./reference/data-model.md) and [Threat model](./reference/threat-model.md) beside them.
- **Contributing?** See [Development](./contributing/development.md).

## Status

**0.18.x chooses whose keys a repository trusts, gets an earlier version of your files back, and
installs in two plain steps on every platform — prikk's, then stikk's — against prikk 0.55.0.** Its
AUR packages are on the way: [Install](./guide/install.md#arch-linux) says when they are published.
Each program installs itself: one line installs stikk, and it prints prikk's own install command
rather than installing prikk; **`T`** lists the maintainer keys this repository trusts, where
**`a`** adopts one and **`x`** removes one — each confirmed by typing the key id, and a removal
says first what it can cost your own history at Verify; **`F`** and **`A`** in Block detail save
one file, or every file at that block, outside the repository and never over anything already
there; a bundle card says which adopted key each recorded sealer names; **Exchange** opens a bundle file and shows prikk's verification and what importing would
change (`i`), imports it as a received branch (`I`) and writes a branch out to a file (`E`);
**Merge** brings one branch into another where prikk's own plan proves the two sides confluent;
**Verify** (`V`) shows prikk's verdict on whether the repository is sound; and **Compare** reaches
any two points, including any two refs. Every repository string is still shown as it is.
The security-critical
layers (shared model, prikk seam, state layer), the interactive **TUI** (shell + Orientation, built on
`ratatui` — RFC 001), **History** + Block detail (RFC 006), the refusal-explanation and glossary surfaces
(RFC 007), **worktree Changes** (RFC 008) and the **Queue view** (RFC 028) are all implemented and
tested; every seam read runs off the UI thread (RFC 010), and config/state resolve per platform on Linux,
macOS, and Windows (RFC 012). stikk can **commit** a worktree capture into prikk's active queue (RFC 014)
and **seal** that queue into permanent, MAINTAINER-signed history (RFC 016) — both behind preview-first,
tiered-confirmation machinery that makes skipping a step a compile error, not a review finding (RFC 013).
At prikk ≥ 0.39 commit is unavailable, with prikk's reasons, when prikk says it would refuse (RFC 027), and
the seal confirmation names the patches it freezes (RFC 028); a confirmed commit authors only the
worktree its preview showed (RFC 030). **stikk notices a change made outside it** — a commit, seal or
branch switch in another terminal — when the terminal regains focus or within five seconds while idle,
refreshes what is on screen, says *"repository changed outside stikk — refreshed"*, and makes an open
confirmation stale at once; `r` refreshes in place, and every refresh re-reads the ref its own screen
shows (RFC 031). **The Changes view and commit's confirmation say what a commit will author**: a
`prikk mv` whose two halves prikk lists is shown as one declared rename and counted, a declaration prikk
will not author as a rename is named instead, and a ref with no published history is named as one rather
than shown against a baseline it does not have (RFC 032). **A declaration's fate is prikk's own
verdict at prikk ≥ 0.44** rather than stikk's inference, one prikk would refuse makes commit
unavailable, a checkout that stopped part-way is shown, and a ref with no published history reads
again on prikk 0.45 and 0.46, where prikk refused it by name (RFC 034). Piped/CI invocation keeps the
one-shot orientation. stikk targets prikk **≥ 0.28**, validated through **0.55.0**, and on prikk 0.42
opens on prikk's current branch (RFC 029). **Patch detail** (`FR-030`) is built at prikk ≥ 0.39
(RFC 035): Block detail lists a block's patches and the Queue lists the queued ones, and either opens
one patch's operations — a text edit's spans, a create's content and mode, a delete's preimage, a
binary replacement's blob ids and sizes, a permission change's two modes, and a rename with the key
that asserted it — with `v` showing prikk's own fields, and content shown byte for byte: quoted, with
everything that does not show as itself escaped. **Every string stikk did not write is now rendered
so that what you read is what prikk reported** (RFC 036): a bidirectional override, a zero-width
character or a look-alike space in a path, message, ref name or key id is written as its escape, in
every view and in the one-shot printout, and two different names never look alike. **Block detail
lists the files at any block, with their modes**, and the ref picker offers the branch you are on
before its first seal (RFC 037, prikk ≥ 0.46 and ≥ 0.47). **Compare** (`FR-033`, delivered in full) is built at prikk
≥ 0.46 (RFCs 038 and 041): `m` and `d` in History, `d` in Block detail and in Changes, and `m`
and `d` in the **ref picker** for any two branches, tags or received refs — showing what changed
between two points in prikk's own `diff` hunks, quoted line by line, with stikk computing no
difference of its own. **Verify** (`V`, `FR-100` in part) shows prikk's verdict on whether the
repository is sound, its failing conditions and its stages, at prikk ≥ 0.39 (RFC 040), and names
what prikk's JSON leaves out. **stikk manages branches and tags, not only browses them**: from the
ref picker, `n` creates a branch at the selected ref's tip, `t` tags it (or any block, from
History), `x` closes it and `s` switches to it — each previewed before it runs, a switch as the
exact files it will write and delete, with the branch you are standing on never offered for
closing and a branch the queue holds work for never offered for switching (RFCs 044 and 045,
prikk ≥ 0.42 and ≥ 0.46). **stikk reopens each repository where you left it** (`FR-122`,
RFC 046): the focused ref, the top-level view and the untracked filter, **re-checked against
prikk before any of it is shown** and stored outside every repository — `STIKK_PRIVATE=1` for a
launch that remembers nothing, and deleting the records is always safe. **stikk merges one branch
into another** (RFC 047, prikk ≥ 0.42): mark the branch to merge from in the ref picker, select
the one it goes into, `M` shows prikk's own plan for the pair against a baseline stikk names only
where it can prove it, and `Enter` runs the merge where that plan proves the two sides confluent
— previewed with every guard re-read at the moment you confirm, never into the branch you are on,
and writing nothing to your worktree. **stikk shares work as a bundle file** (RFC 050,
prikk ≥ 0.53): `i` opens one and shows prikk's own verification and what prikk says importing it
would change against your branch of that name, `I` imports it as `remotes/<name>` — read-only
history beside your own, declined where it would change nothing or move that branch back — and
`E` writes a branch out to a file outside the repository, never over one that is already there.
**stikk gets an earlier version of your files back** (RFC 053, prikk ≥ 0.54): in Block detail, `F`
saves one file as it was at that block and `A` saves every file at it into a new folder — always
outside the repository and never over an existing path, both refused in stikk's own words before
prikk is asked, with nothing in the repository or the worktree changed. **stikk shows which keys a
repository trusts** (RFC 054, prikk ≥ 0.54): `T` lists the adopted maintainer keys with your own
marked, and a bundle card says which of them, if any, each recorded sealer's id names — a name, not
a signature, which Verify checks after the import. **stikk chooses them, too** (RFC 056, prikk ≥
0.54): `a` adopts a key and `x` removes the selected one, each previewed and confirmed by **typing
the key id** — stikk's first typed confirmations, and neither needs a key of your own. The remove
card says that removing a key which sealed this history makes Verify fail it until the same key is
adopted again under the same id, and says so again for your own key; an id already trusted under a
different key is shown as the security event it is, with both public keys and nothing to confirm.
Sync, and checkout of a block or a path, remain unbuilt; **adopting a received tag** and **author
keys**, and a **doctor**/lock-inspector view, are named gaps too.
