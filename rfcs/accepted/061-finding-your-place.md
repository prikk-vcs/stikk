# RFC 061 — Finding your place

**Status.** **Accepted by the project owner 2026-11-21**, decision 2's key change included (*"Accepted."*, answering the question put on acceptance); [Handoff A](../handoffs/061-finding-your-place/a-finding-your-place-handoff-v1.md). **Proposed 2026-11-21** by the architect, as 0.21.0's increment 4, the last
(`.git-exclude/release/0-21-0-scope.md`, authorized 2026-11-04, last amended 2026-11-21). Written to the owner's
philosophy (memory, 2026-11-21): *"finally clean, safe and secure, and robust and sophisticated design"*, and
*"APIs and UI/UX for users not to be confused or misunderstand"*. **One decision goes beyond the scope's words,
and is put to the owner on acceptance:** decision 2 takes the letter key away from fourteen actions that sit on
case pairs or are rare. The scope said *"the letter keys stay as shortcuts"*. **No other open question.**
**Measured** from the code at `main` `472e6e0` (each site cited where it is used):
- the key map, `keys.rs`;
- `Enter`'s meaning in every screen, `app.rs`'s `select` and `select_screen`;
- the palette's catalog, `stikk-core/src/palette.rs`;
- the design rules `TU-05`, `TU-06` and `TU-07`, and requirements `FR-121` and `FR-125`.

**Tracks.** `TU-05`, `TU-06`, `TU-07` (rewritten by this RFC), `FR-125`, `NFR-A01`. RFC 002, whose catalog this RFC
delivers. RFC 059 (the palette roles; reverse video's contrast). RFC 060 (the 60 × 15 layouts, the key line's row,
the title builder).
**Touches.**
- `stikk-core`: a new `actions` catalog, which replaces `palette.rs`'s list.
- `stikk-tui`: `keys.rs`, `app.rs`'s `select`, a new action-list overlay, the key line, `status_bar.rs`, `title.rs`,
  and every view's selected row.
- The Glossary's *Keys*, the guide's key reference, `TU-05`, `TU-06` and `TU-07`, and the changelog.

## Summary

Today a user finds their way by remembering letters:
- 29 of the 37 actions in `keys.rs` are letter keys;
- **nine letters do one thing in lower case and another in upper case** (F1);
- the selected row is marked only by `▶` and a bold id;
- the one key hint is the first thing the status bar drops.

This RFC:
- builds **one catalog of every action**, which the palette, the action lists, the key line and the help all read;
- gives **each letter one meaning, in either case**;
- makes **Enter on a selected row open the list of what can be done with it**;
- highlights **the whole selected row**;
- adds **a key line that stays**.

## Findings

### F1 — nine letters change meaning with case, and Caps Lock flips every one

From `keys.rs:61-95`:

| Letter | lower case | upper case |
|---|---|---|
| q | back | open the Queue |
| s | switch branch | **seal** |
| u | show untracked | **undo a block** |
| a | adopt a key | **save all files** |
| i | open a bundle | **import** |
| m | mark a block | **merge plan** |
| v | prikk's own fields | open Verify |
| r | refresh | the refusals list |
| t | **new tag** | open Trust |

**And one letter changes meaning by screen:** `x` closes a branch in the ref picker, and removes a key in Trust
(`app.rs:1809`, *"One key, two screens"*).

A terminal sends characters, not keys. With Caps Lock on, a user who presses `q` to go back sends `Q`, and opens the
Queue. One who presses `s`, meaning to switch, starts a seal. The confirmation cards stop the harm, **but the user is
shown a card for something they did not ask for.** That is the confusion the owner's second line forbids.

**The reason for the pairs is `TU-06`'s design rule**, *"mutating actions are uppercase"*. It is meant as a deliberate
act: Shift for a write. **Caps Lock removes that meaning**, and every write is confirmed by its card anyway
(`FR-121`'s tiers, which do not mention case).

### F2 — the palette is not the catalog it is meant to be

`palette.rs` lists **13 commands**, and stikk has 37 actions (`keys.rs`'s `Action`). Missing from it:
- switch, merge, new branch, new tag and close branch;
- open a bundle, import and export;
- Trust, adopt a key;
- save a file, save all, background operations;
- marking and comparing.

`FR-125` asks for a palette *"exposing every operation by name with its keybinding"*.

**It also lists a key that does not exist:** *"Go to Orientation — `g o`"*. `keys.rs` binds no `g`. A user who reads
the palette and presses `g` gets nothing.

### F3 — the design documents and the code disagree about keys

`TU-05` says:
- `b` is *"background operations"*; in the code, `b` opens the ref picker, and background operations is `o`;
- there are `/` find, `y` copy, and `Tab` pane cycle; **none exist**.

`TU-06`'s context keys are mostly not the keys the code binds. For example, History's `a` and `s` are *"send to
Compare A/B"* in `TU-06`; in the code they are adopt and switch. **There is no single source, so each copy
drifted.**

### F4 — Enter means "open", or nothing

`app.rs`'s `select_screen` decides Enter per screen:
- the ref picker opens History;
- History opens Block detail;
- Block detail opens Patch detail;
- the Queue opens Patch detail;
- **Changes does nothing**.

Each selected item's other actions are separate letters, several of them F1's pairs.
- A History block has, besides open: mark, compare, undo and new tag.
- A ref in the picker has eight: open, switch, merge, mark, close, new branch from it, new tag at it, and export.

**Nothing on screen tells a user any of this**, except the palette, for the 13 actions it has.

### F5 — where you are, and which keys you have

- **The selected row** is `▶` plus `selected()`'s bold id (RFC 060). Nothing marks the rest of the row, so on a wide
  screen the eye has to travel back to the left edge.
- **The keys:** the status bar's one hint, `:palette ?:help q:back`, is the first thing `fit` drops
  (`status_bar.rs:96`). Views put more key hints in their titles, and RFC 060's title builder drops them first. **At
  the sizes RFC 060 opened, a user sees no keys at all.**

## Decisions

1. **One catalog of every action, in `stikk-core`** (RFC 002's rules 1–4, delivered here):
   - **Each entry has:**
     - a stable id, such as `ref.switch`;
     - a name, such as *"Switch to this branch"*;
     - its key, or none;
     - **where it applies**: everywhere, a screen, or a kind of selected row;
     - its confirmation tier, from its operation;
     - **its availability**, with the reason when it is unavailable, as `palette.rs`'s `unavailable_reason` gives
       today;
     - **its rank in the key line**.
   - **Every consumer reads it, and none keeps a list of its own:**
     - the palette, **which shows every action** (`FR-125`);
     - the action list (decision 3);
     - the key line (decision 5);
     - the Glossary's *Keys* section;
     - `keys.rs`'s map from key to action.

     The guide's key reference is checked against it by a test. *(It cannot be generated, because the guide is
     prose.)*
   - **RFC 002 moves to `archive/`, superseded by this RFC.** Its rule 5, rebinding keys in the config, is not built
     here, and gets its own RFC when a user needs it. Its rule 6 is replaced by decision 2.
2. **One letter, one action, in either case.** Letters are matched without regard to case, everywhere outside a text
   prompt. **No letter has two meanings.** `TU-06`'s *"mutating actions are uppercase"* is withdrawn: the confirmation
   card is what makes a write deliberate (`FR-121`), and Caps Lock makes case meaningless.

   **The keys, on every screen:**

   | Key | Action | Before |
   |---|---|---|
   | `Enter` | the actions for the selected row (decision 3) | open |
   | `→` | open the selected row: Enter's first action, at once | — |
   | `←`, `Esc`, `q` | back; at the root, quit | `Esc`, `q` |
   | `↑` `↓`, `k` `j` | move; in a card, read on (RFC 060) | same |
   | `?` | help | same |
   | `:` | every action, by name | same |
   | `b` | branches and tags (the ref picker) | same |
   | `w` | Changes | same |
   | `p` | the Queue: **p**atches waiting to be sealed | `Q` |
   | `v` | Verify | `V` |
   | `t` | Trust | `T` |
   | `o` | **o**pen a bundle file | `i` (`o` was background operations) |
   | `c` | commit | `C` |
   | `s` | seal | `S` |
   | `r` | refresh | same |
   | `d` | compare: Changes' file diff, or History's marked block with the selected one | same |
   | `m` | mark a block, to compare | same |
   | `f` | prikk's own **f**ields, in Patch detail, Compare and Verify | `v` |
   | `u` | show untracked files, in Changes | same |

   **Fourteen actions lose their letter, and move to the action lists and the palette:**
   - switch (`s`), merge plan (`M`), close branch (`x`), new branch (`n`), new tag (`t`);
   - import (`I`), export (`E`);
   - adopt a key (`a`), remove a key (`x`, in Trust);
   - undo a block (`U`);
   - save a file (`F`), save all (`A`);
   - the refusals list (`R`), background operations (`o`).

   Each was either half of an F1 pair or rare. Each is now found where it applies, by name.

   **This is the decision put to the owner**, since the scope said the letters stay. The alternative, keeping every
   letter, leaves F1's nine pairs in place. **stikk has no users yet** (the owner, 2026-10), so the keys can be set right
   once, before anyone learns the old ones.
3. **Enter on a selected row opens its action list.**
   - **The list is a small overlay** beside the row, titled by the item, for example *"Block 1a2b3c4d5e6f"*.
   - **It shows the row's actions, then the screen's own actions**, each with its key if it has one:
     *"Open this block   →"*, *"Mark it, to compare   m"*, *"Undo this block"*, *"New tag at this block"*.
   - **An action that cannot run now stays, dimmed, with its reason** (`TU-07`'s rule), as the palette shows it.
   - **The first action is the one `→` runs, and it is highlighted.** So Enter, Enter does what Enter alone did
     before, and `→` does it in one key.
   - In the list, `↑` `↓` move and Enter runs. `Esc`, `←` and `q` close it.
   - **A screen with no selectable row** (Orientation, a loading screen) answers Enter with its screen actions
     alone. A Changes row gets Changes' own actions: *"Compare the worktree with what commit would author"* (`d`, which
     compares the whole worktree, not the row), *"Show untracked files"* (`u`) and *"Commit"* (`c`).
   - **Inside an overlay, Enter keeps choosing the highlighted entry**, as today: every overlay with entries is
     already a list.
4. **The selected row is highlighted across its whole width.**
   - It is drawn in **reverse video**, its cells' foreground and background swapped, padded to the list's full
     width.
   - **`▶` stays**, for terminals that cannot show reverse video, and in `plain`.
   - **`selected()`** (RFC 060) adds reverse video. The highlight is decided there, and nowhere else.
   - **Contrast:**
     - a reversed cell in the terminal's own pair keeps that pair's contrast;
     - a reversed cell in one of RFC 059's colours has that colour as its background, under the terminal's
       background as text. That is the same ratio RFC 059 already holds at 7 : 1;
     - in `high-contrast`, the pair swaps, at 21 : 1.

     RFC 059's matrix learns to swap a reversed cell before it compares.
5. **A key line that stays.**
   - **One row, above the status bar**, on every screen, at every size from 60 × 15 (RFC 060 F4 counted it). It lists
     the screen's keys from the catalog, **by rank**, for example:
     *"Enter actions · ↑↓ move · b branches · w changes · c commit · s seal · ? help · q back"*.
   - **When it does not fit,** the lowest-ranked keys drop first, a whole entry at a time, never cut.
   - **`? help` and `q back` are never dropped**, and *"Enter actions"* is dropped only after every other entry.
   - **It is the one place keys are named.**
     - The status bar loses its hint, and `fit`'s step 1 goes.
     - **View titles lose their key hints**: RFC 060's `Title` keeps the name and the position only. RFC 060 had
       kept the key hints in view titles until this RFC.
     - An overlay's foot line (RFC 060 decision 3) stays, because it names the one key that reads on, where the key
       line cannot be seen under the card.
6. **The documents follow the catalog.**
   - `TU-05`, `TU-06` and `TU-07` are rewritten to describe the catalog and decision 2's rule, not to list keys. The
     Glossary's *Keys* lists them, from the catalog.
   - The guide's *Key reference* is rewritten to decision 2's table, and a test asserts that it names every key the
     catalog binds, and no other.
   - `TU-05`'s `/` find, `y` copy and `Tab` pane cycle are recorded as **not built**, with `FR-124`'s copy carried,
     rather than claimed.

## Tests and evidence plan

| Question | Level | How |
|---|---|---|
| Is the catalog whole, and are the keys unique ignoring case? | unit | every `Action` but text entry has a catalog entry; no two entries share a key, ignoring case; every key the catalog names is bound in `keys.rs`, so no phantom `g o`; the palette shows every entry |
| Does Caps Lock change nothing? | unit | every key, upper case and lower case, maps to the same action outside text prompts, and in a prompt typing is unchanged |
| Does Enter open the right list for each kind of row, with the reasons? | headless | a capture per row kind (ref, block, file, patch, queued patch, Changes row, trust key), and per screen with no row, at 60 × 15 and 80 × 24; disabled entries with their reasons; Enter, Enter and `→` each do what Enter did before |
| Is the whole selected row highlighted, at 7 : 1? | headless | every list view: every cell of the selected row is reversed and the row is padded to full width; RFC 059's matrix, with reversed cells swapped, stays at zero below 7 : 1 |
| Does the key line stay, and never cut? | headless | every screen at 60 × 15, 80 × 24 and 120 × 40: `? help` and `q back` are always present; entries drop whole; RFC 060's overflow recorder holds the line |
| Does the guide match the catalog? | unit | its key reference names exactly the catalog's bound keys |
| Real terminal | **Linux**, the architect, after approval | the relay of RFC 060's check, at 80 × 24: Enter on a History block, then `→`. The evidence is the transcript, with reverse video (SGR 7) on the selected row and the list's entries |
| Windows and macOS | documented | reverse video is SGR 7, which every terminal in RFC 059's survey supports |

## Delivery

**One handoff, after acceptance.** Each commit carries its own tests:
1. the catalog, and the palette that shows all of it;
2. the keys, case-insensitive, and the moved actions reachable from the palette;
3. Enter's action list and `→`;
4. the full-row highlight, and the matrix's reverse-video swap;
5. the key line, the status bar's hint removed, and the titles' key hints removed;
6. the documents and the guide check.

## Changelog

- `### Changed`:
  - **every letter key means one thing, in either case**, so Caps Lock changes nothing;
  - **Enter on a selected row lists what can be done with it**, and `→` opens it;
  - the Queue is `p`, Verify is `v`, Trust is `t`, opening a bundle is `o`, commit is `c`, seal is `s`, and prikk's
    own fields are `f`;
  - switch, merge, branch, tag, close, import, export, adopt, remove, undo and save are in Enter's list and the
    palette.
- `### Added`:
  - the whole selected row is highlighted;
  - a key line that stays on screen;
  - the palette lists every action.
