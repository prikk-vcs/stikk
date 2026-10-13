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
through **0.51.0**; a newer prikk still runs, but stikk says its output shapes have not been checked
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

**v0.9.x shows what each patch changed.** **Patch detail** (prikk ≥ 0.39) renders one patch's
operations from prikk's own report — a text edit's spans, a created file's content and mode, a deleted
file's preimage, a binary replacement's blob ids and sizes, a permission change's two modes, and a
rename with the key that asserted it — opened from Block detail, which lists every block's patches, or
from the Queue. Content is shown **byte for byte**: quoted, one row per line, with everything that does
not show as itself escaped, so a whitespace-only edit is visible and a bidirectional override cannot
reorder what you read. `v` shows prikk's own fields instead. Around that it reads orientation, ref
history, block detail, worktree changes and the **Queue view**, with refusal explanations and a
glossary. It performs two mutations: **commit** (author a worktree capture into the active queue) and
**seal** (freeze that queue into permanent, MAINTAINER-signed history, naming the patches it freezes) —
both always preview-first, with tiered confirmation a compile error can't let you skip. **A confirmed
commit authors only the worktree its preview showed.** **stikk notices a commit, seal or branch switch
made in another terminal** and refreshes what is on screen. **A `prikk mv` is shown as the rename prikk
says it will author**, from prikk's own verdict at prikk ≥ 0.44, and a declaration prikk would refuse
makes commit unavailable. **A ref with no published history is named as one**, and reads again on prikk
0.45 and 0.46, where prikk refused it by name. Compare, merge, sync, and tag or branch creation are not
built yet; outside Patch detail, a path or a message is not escaped.

## Links

- Source & issues: <https://github.com/prikk-vcs/stikk>
- Documentation (guide + design set + API): the project's GitHub Pages site
- License: Apache-2.0
