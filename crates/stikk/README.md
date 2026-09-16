# stikk

[![license](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/prikk-vcs/stikk/blob/main/LICENSE)

**stikk is a history browser and workbench for the [prikk](https://github.com/prikk-vcs/prikk)
version control system** — a terminal (TUI) front-end over one shared operation layer.

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

**stikk drives the external `prikk` binary at runtime** (it is not a Cargo dependency): install a
compatible `prikk` on your `PATH` as well. This release requires prikk **≥ 0.28** and is validated
through **0.42.0**; a newer prikk still runs, but stikk says its output shapes have not been checked
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

**v0.8.x keeps the screen true to the repository.** It reads orientation, ref history, block detail,
worktree changes and the **Queue view** (each queued patch, with its message on prikk ≥ 0.42), with
refusal explanations and a glossary. It performs two mutations: **commit** (author a worktree capture
into the active queue) and **seal** (freeze that queue into permanent, MAINTAINER-signed history, naming
the patches it freezes) — both always preview-first, with tiered confirmation a compile error can't let
you skip. **A confirmed commit authors only the worktree its preview showed**: if anything changes
between preview and Enter, stikk says so and commits nothing. **stikk notices a commit, seal or branch
switch made in another terminal** — when the terminal regains focus, or within seconds — refreshes what
is on screen, and makes an open confirmation stale. **A `prikk mv` is shown as the rename it will
author**, and a declaration prikk will not author as one is named rather than counted. **A ref with no
published history is named as one**, rather than shown as changes against a baseline it does not have.
Patch detail, Compare, merge, sync, and tag or branch creation are not built yet.

## Links

- Source & issues: <https://github.com/prikk-vcs/stikk>
- Documentation (guide + design set + API): the project's GitHub Pages site
- License: Apache-2.0
