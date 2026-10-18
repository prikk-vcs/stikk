# stikk

[![license](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/prikk-vcs/stikk/blob/main/LICENSE)

**stikk is a history browser and workbench for the [prikk](https://github.com/prikk-vcs/prikk)
version control system** — a terminal (TUI) front-end over one shared operation layer.

![stikk's History view: a ref's block lineage, newest first, with the queued tier above it, the tip marked, and a rollback block and one needing attestations called out](https://raw.githubusercontent.com/prikk-vcs/stikk/main/docs/assets/readme/history.svg)

![stikk's Compare view: what changed between two blocks, in prikk's own diff hunks, every line quoted byte for byte so a trailing space is visible](https://raw.githubusercontent.com/prikk-vcs/stikk/main/docs/assets/readme/compare.svg)

![stikk's Verify view: prikk's verdict that the repository is sound, every stage prikk ran, and a sentence naming what prikk's JSON report leaves out](https://raw.githubusercontent.com/prikk-vcs/stikk/main/docs/assets/readme/verify.svg)

*Drawn by stikk's own views from invented data, not photographed — see
[RFC 042](https://github.com/prikk-vcs/stikk/blob/main/rfcs/accepted/042-readme-key-visuals.md).*

Its founding property is that **stikk owns no repository authority and no secrets**: every repository
fact is re-derived from prikk, and prikk — never stikk — reads signing key material. Its stance
mirrors prikk's own: *where prikk refuses, stikk explains.* The name is Norwegian for *to set a
course, to take a bearing.*

This binary crate is the launcher; the layers beneath it are published as
[`stikk-core`](https://crates.io/crates/stikk-core),
[`stikk-tui`](https://crates.io/crates/stikk-tui),
[`stikk-prikk`](https://crates.io/crates/stikk-prikk),
[`stikk-state`](https://crates.io/crates/stikk-state), and
[`stikk-model`](https://crates.io/crates/stikk-model).

## Install

```sh
cargo install stikk
```

This compiles stikk. **If you would rather not compile, or do not have Rust**, every release also
ships ready-to-run binaries for Linux, macOS and Windows, with checksums — see
[Install](https://prikk-vcs.github.io/stikk/guide/install.html).

**stikk drives the external `prikk` binary at runtime** (it is not a Cargo dependency): install a
compatible `prikk` on your `PATH` as well, which prikk's own installer does in one line
(`curl -fsSL https://github.com/prikk-vcs/prikk/releases/latest/download/install.sh | sh`).
This release requires prikk **≥ 0.28** and is validated
through **0.52.0**; a newer prikk still runs, but stikk says its output shapes have not been checked
against it rather than silently assuming they have. If `prikk` is missing, stikk opens and then
explains that — by design.

## Use

```sh
stikk [path]          # open the TUI on a repository (or discover upward from the cwd)
stikk config path     # print where stikk's own config and state live
stikk config check    # validate a config file
```

Piped or non-TTY invocation prints a one-shot orientation instead of opening the TUI.

## Status

**v0.14.x installs in one command, prikk included, and tells you whether your repository is
sound — now against prikk 0.52.0.** **One line installs stikk** — `curl … install.sh | sh`, or `irm … install.ps1 | iex` on
Windows — checking the download against the checksum published beside it, installing where you own
it, and offering to install **prikk** through prikk's own verified installer; `cargo binstall
stikk` fetches the same prebuilt binary. **Verify** (`V`, prikk ≥ 0.39) shows prikk's verdict on
whether the repository is sound, every condition it fails and every stage it ran — and a damaged
repository reads as a verdict, never as a refusal. **Compare** (prikk ≥ 0.46) shows what changed
between two points in **prikk's own `diff` hunks** — two marked blocks or a block and the one
before it, from History and Block detail; **any two refs**, marked and selected in the ref picker;
the worktree against what `commit` would author, from Changes. Every line is quoted byte for byte, so a
whitespace or line-ending edit is visible, and stikk computes no difference of its own. **Every
string stikk did not write is rendered so that what you read is what prikk reported**: a
bidirectional override, a zero-width character or a look-alike space in a path, branch name or
message is written as its escape, and two different names never look alike. **Patch detail**
(prikk ≥ 0.39) renders one patch's operations from prikk's own report — a text edit's spans, a
created file's content and mode, a deleted file's preimage, a binary replacement's blob ids and
sizes, a permission change's two modes, and a rename with the key that asserted it — opened from
Block detail, which lists every block's patches **and the files present at that block, with their
modes** (prikk ≥ 0.46), or from the Queue. `v` shows prikk's own fields instead. Around that it
reads orientation, ref history, worktree changes and the **Queue view**, with refusal explanations
and a glossary; the **ref picker offers the branch you are on before its first seal** (prikk ≥
0.47). It performs six mutations: **commit** (author a worktree capture into the active queue),
**seal** (freeze that queue into permanent, MAINTAINER-signed history, naming the patches it
freezes), and — from the ref picker — **create a branch** (`n`), **create a tag** (`t`, or at any
block from History), **close a branch** (`x`) and **switch to a branch** (`s`, previewed as the
exact files it will write and delete), at prikk ≥ 0.42 and ≥ 0.46 for a switch — every one of
them always preview-first, with tiered confirmation a compile error can't let you skip.
**A confirmed commit authors only the worktree its preview showed.** **stikk notices a commit, seal
or branch switch made in another terminal** and refreshes what is on screen. **A `prikk mv` is shown
as the rename prikk says it will author**, from prikk's own verdict at prikk ≥ 0.44, and a
declaration prikk would refuse makes commit unavailable. **A ref with no published history is named
as one**, and reads again on prikk 0.45 and 0.46, where prikk refused it by name. **Two refs
compare from the ref picker** (`b`, then `m` to mark one and `d` to compare it with the selected
one). **stikk reopens each repository where you left it** — the focused ref, the top-level view
and the untracked filter, re-checked against prikk before any of it is shown, stored outside
every repository, and `STIKK_PRIVATE=1` for a launch that remembers nothing. Merge, sync, and
checkout of a block or a path are not built yet.

## Links

- Source & issues: <https://github.com/prikk-vcs/stikk>
- Documentation (guide + design set + API): the project's GitHub Pages site
- License: Apache-2.0
