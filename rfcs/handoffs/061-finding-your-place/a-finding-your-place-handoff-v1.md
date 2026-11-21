# Handoff A — RFC 061: finding your place (v1)

**Companion to:** [RFC 061](../../done/061-finding-your-place.md), **accepted 2026-11-21**, decision 2's key change
included. **Read the RFC first**: F1–F5 and decisions 1–6. **Covers:** all of it. **0.21.0's increment 4, the last.**
**Not a release.**

> **What this increment is.** Today a user finds their way by remembering letters, nine of which change meaning with
> case, so Caps Lock flips them (F1). After this handoff:
> - **one catalog** lists every action, and the palette, the action lists, the key line, the help and the key map all
>   read it;
> - **each letter means one action, in either case**;
> - **Enter on a selected row lists what can be done with it**, and `→` opens it;
> - **the whole selected row is highlighted**;
> - **a key line stays on screen**.
>
> **The owner's philosophy decides what this handoff leaves open:** *"finally clean, safe and secure, and robust and
> sophisticated design"*, and *"APIs and UI/UX for users not to be confused or misunderstand"*. The catalog is the
> *clean*: one list, no copies. Decision 2 is the *not confused*: one meaning per letter. Say in the request how you
> chose wherever you had to.
>
> **No branch.** Run the local gates and the local suite before submitting. After approval, push to `main`, then
> dispatch the full-matrix suite and supply chain there.

---

## 0. Evidence plan

| Question | Level | Evidence in the request |
|---|---|---|
| Is the catalog whole, with keys unique ignoring case and none phantom? | unit | test names, and the catalog's entry count |
| Does Caps Lock change nothing outside a prompt, and nothing inside one? | unit | test names |
| Does Enter open the right list for each kind of row, with the reasons; do Enter twice and `→` do what Enter did? | headless | captures at 60 × 15 and 80 × 24, one per row kind and one per screen with no row |
| Is the whole selected row reversed and padded, at 7 : 1? | headless | test names; RFC 059's contrast-check count with reversed cells swapped |
| Does the key line stay at every size, and drop whole entries only? | headless | captures at 60 × 15, 80 × 24 and 120 × 40; RFC 060's overflow recorder still empty |
| Does the guide's key reference match the catalog? | unit | the test's name |
| Real terminal | **the architect**, after your approval | not yours. Say that the build is ready |
| Nothing else regressed | real binary, 0.28.0 + 0.58.0; then the full matrix in CI | counts |

## 1. Commit 1: the catalog, and a palette that shows all of it (decision 1)

1. **`stikk-core/src/actions.rs`**, replacing `palette.rs`'s `Command` list. One `Action` entry per user action, with:
   - `id`: kebab-case and stable, by RFC 002 rule 1;
   - `name`: what a user reads;
   - `key: Option<Key>`, a letter, a named key or none;
   - `applies: Where`: everywhere, a screen kind, or a row kind;
   - `category: Option<RequestCategory>`, from which the tier and the signing gate follow, as `palette.rs`'s do today;
   - `needs_focused_ref`;
   - `rank: u8`, for the key line.

   **Availability** keeps `palette.rs`'s `unavailable_reason` logic, moved, not rewritten: the reason words do not
   change.
2. **The palette lists every entry.** That is `FR-125`. The phantom `g o` is gone, because a key now exists only if
   the catalog binds it.
3. **Every action reachable today stays reachable.** List the catalog's entries in the request, with each one's old
   key and new key.

**Tests:**
- every `keys.rs` `Action` except text entry, `None` and `Quit` has exactly one catalog entry;
- no two entries share a key, ignoring case;
- every key the catalog names is one `keys.rs` binds;
- the palette shows every entry;
- availability reasons are unchanged, using today's palette tests, moved.

## 2. Commit 2: one meaning per letter, in either case (decision 2)

1. **`keys.rs` maps from the catalog.** Outside a text prompt, a letter is matched **without regard to case**. The
   key set is RFC 061 decision 2's table, exactly.
2. **`→` runs the selected row's first action; `←` is back.** In a text prompt, `←` and `→` keep whatever they do
   today. Say what that is.
3. **The fourteen actions that lose their letter** stay reachable from the palette, and from commit 3's lists.
4. **Text prompts are unchanged.** Typing a key id, a message or a path takes every character as typed, upper case
   included.

**Tests:**
- for every letter, upper and lower case map to the same action outside a prompt;
- in each kind of prompt, `Q`, `S` and `x` are typed characters;
- each moved action is reachable from the palette;
- **Caps Lock, simulated as upper-case input, never opens a card the lower-case key would not**: for every letter,
  the action is identical.

## 3. Commit 3: Enter's action list, and `→` (decision 3)

1. **A new overlay, `ActionList`**, on RFC 060's `ListPanel`. It is titled by the item (*"Block 1a2b3c4d5e6f"*,
   *"Branch heads/dev"*, through `inert`). Its entries come from the catalog filtered to the row's kind, then the
   screen's own actions.
   - Each entry shows its name and its key, if it has one.
   - An unavailable entry stays, dimmed, with its reason on the row below it (`TU-07`).
2. **The first entry is the row's open action, and is highlighted.** So Enter, Enter is what Enter alone did before,
   and `→` is that at once.
3. **The row kinds, and their entries, in order.** Take them from today's keys and `select_screen`. List each kind's
   entries in the request, and choose by decision 3. At least:
   - **a ref** in the picker: open its history, switch to it, merge plan, mark to compare, new branch from it, new tag
     at it, close it, export it;
   - **a History block:** open, mark to compare, compare with the marked block, undo, new tag at it, new branch from
     it;
   - **a Block detail file:** open its patch, save the file, save all;
   - **a queued patch:** open;
   - **a Trust key:** remove it, then the screen's *"Adopt a key"*;
   - **a Changes row:** Changes' own actions only, *"Compare the worktree with what commit would author"* (`d`),
     *"Show untracked files"* (`u`) and *"Commit"* (`c`), **because Changes has no per-row action** (RFC 061 decision
     3);
   - **the bundle view:** *"Import this bundle"*;
   - **screens with no selectable row,** such as Orientation: their screen actions alone.
4. **In the list:** `↑` `↓` move, Enter runs, and `Esc`, `←` and `q` close it. **Inside other overlays, Enter is
   unchanged.**
5. **A confirmation that follows** is RFC 060's card, gate included. The list adds no new way to confirm.

**Tests:**
- a capture per row kind and per row-less screen, at 60 × 15 and 80 × 24;
- disabled entries with their reasons;
- Enter, Enter and `→` each equal today's Enter, asserted against today's next screen;
- the list closes on each of `Esc`, `←` and `q`.

## 4. Commit 4: the whole row (decision 4)

1. **`theme::selected()`** returns today's style plus `Modifier::REVERSED`. Every list view pads the selected row to
   its area's full width, with the padding in the same style, so the whole row is reversed. **`▶` stays.**
2. **RFC 059's contrast matrix** learns reverse video: for a reversed cell, swap its foreground and background,
   including `Reset`, which is the terminal's own pair, before it compares. Where both ends are `Reset`, the terminal's
   own pair is swapped, and it is skipped as today.
3. `plain` and `high-contrast` keep reverse video (RFC 059 decision 7).
4. **The emphasis guard** (RFC 060) allows `REVERSED` only in `theme.rs`, as it allows `BOLD`.

**Tests:**
- every list view's selected row is reversed in every cell, across the full width;
- the matrix's count of reversed cells, with zero below 7 : 1;
- the guard.

## 5. Commit 5: the key line (decision 5)

1. **One row above the status bar**, on every screen at every size from 60 × 15. It lists the screen's keys from the
   catalog, by `rank`, as *"Enter actions · ↑↓ move · b branches · w changes · c commit · s seal · ? help · q back"*.
   - Entries are separated by `·`.
   - When it does not fit, **whole entries drop, lowest rank first**.
   - **`? help` and `q back` never drop**, and *"Enter actions"* drops last of the rest.
   - Its text is `dim`, and each key is `fg`.
2. **The status bar's key hint goes**, and `fit`'s step 1 with it.
3. **`title.rs` loses `keys`.** A title is a name and a position (RFC 061 decision 5). Every call site's `.keys(…)`
   goes.
4. **The overlay foot line stays** (RFC 060 decision 3): it names the one key that reads on, where the key line is
   hidden.
5. **RFC 060's overflow recorder covers the key line.**

**Tests:**
- captures at 60 × 15, 80 × 24 and 120 × 40 on three screens;
- `? help` and `q back` present at every size;
- entries whole;
- the recorder empty;
- titles carry no key hints.

## 6. Commit 6: the documents (decision 6)

1. **`TU-05`, `TU-06` and `TU-07`** describe the catalog and decision 2's rule, and list no keys. Write `TU-05`'s `/`
   find, `y` copy and `Tab` pane cycle as **not built**, and carry `FR-124`'s copy.
2. **The Glossary's *Keys* section** renders from the catalog.
3. **The guide's *Key reference*** (`getting-started.md`) is rewritten to decision 2's table. **A test reads it, and
   asserts that it names exactly the keys the catalog binds.**
4. **RFC 002** moves to `rfcs/archive/`, with a status line, *"Superseded by RFC 061 (the catalog); rule 5, rebinding
   keys in the config, is not built, and gets its own RFC when wanted"*, and its index row moves with it.
5. **`CHANGELOG.md`, `## Unreleased`:** the RFC's lines.
6. **README key visuals:** regenerate them, and say what changed in each.

## 7. The traps

1. **No second list.** Nothing outside the catalog names an action's key or name: not the Glossary, not the key line,
   not a title, not a test fixture. Tests read the catalog.
2. **Case-insensitive outside prompts only.** A typed key id must still distinguish `A` from `a`.
3. **The reason words do not change** when availability moves into the catalog.
4. **Enter inside an overlay is unchanged.** Only Enter on a screen's selected row opens a list.
5. **`REVERSED` and `BOLD` come only from `theme.rs`.**
6. **`inert`** on every repository string in a list title or entry.
7. **Paste captures as the file holds them, and grep before committing.**

## 8. Submit (no branch)

Run the eight gates, and the local suite at **0.28.0 and 0.58.0**. Package to
`.git-exclude/review-request/061-finding-your-place-a/review-request-v1.md`, and lead with:
- the catalog, with every entry's old and new key;
- each row kind's list;
- the key-line captures;
- the reverse-video matrix count;
- the guide check.
