# RFC 002 — Action-id catalog and keybinding configuration

**Status.** Proposed
**Tracks.** The stable catalog of user actions and how the config binds keys to them. Referenced by
the internal design (`CF-03`, `TU-05`) as a Program-Design deliverable.
**Touches.** A new action-id definition (proposed home: `stikk-core`, since actions are operations,
not widgets); `stikk-state`'s config (the `[keybindings]` surface); both frontends' input mappers.

## Summary

The internal design binds keybindings "per stable action id" (`CF-03`) and resolves TUI keys "through
the config's action-id map" (`FE-02`), but the catalog of those ids does not exist yet. This RFC
proposes what an action id *is*, where the catalog lives, and how the config binds keys to it —
without enumerating every id (that grows with the views) but fixing the rules the enumeration must
follow.

## The problem

Two frontends must offer the same operations (`FR-123`), the config must bind keys to operations in a
way that survives a UI rewrite, and the command palette must list every operation by name with its
binding and required capability (`FR-125`, `TU-07`). All three need one thing: a **stable identifier
per user action** that is neither a keycode nor a widget, so that a binding, a palette entry, and a
menu item all refer to the same action.

## Proposed rules

1. **An action id names a user intent, not a key or a widget.** `history.open-patch`,
   `work.commit`, `merge.execute`, `verify.run` — stable kebab-case strings, grouped by view/family.
   The mutating-vs-reading nature and required capability come from the operation it maps to
   (`stikk-model::RequestCategory`, `Capability`), not from the id.
2. **Ids are stable across UI changes and are the binding surface.** A key rebind, a palette entry,
   and a GUI menu item all reference the same id; renaming an id is a breaking change to a user's
   config, so ids follow the same "stable forever" discipline as prikk's object type codes.
3. **The catalog is defined once, in the operation layer** (proposed: `stikk-core`), because an action
   is an operation with a name — not in a frontend, which would break parity, and not in
   `stikk-state`, which should not know the operation set. Each frontend's input mapper consumes the
   catalog; neither invents an action.
4. **Every action declares its default binding(s) and its capability requirement** in the catalog, so
   the palette can show them and disable-with-reason (`TU-07`) without a frontend hard-coding either.
5. **The config binds, it does not define.** A `[keybindings]` section maps keys to existing action
   ids; an unknown action id is preserved-and-warned exactly as an unknown config key is (`INV-4`), so
   a newer stikk's action survives an older stikk reading the file.
6. **Mutating actions keep the uppercase-key discipline** (`TU-06`): default bindings for
   queue-affecting and publishing actions are distinct from reading ones, and the catalog encodes
   that so it is not re-decided per frontend.

## Consequences

- The command palette (`FR-125`), menu inventory (`GU-02`), and keybinding config (`CF-03`) all read
  one catalog; a new operation adds one catalog entry and is reachable everywhere.
- The TUI/GUI parity test (`TS-08`) becomes concrete: assert both frontends' input mappers cover the
  same catalog — an action in one and not the other fails the build.

## Open questions

- Should chord bindings (multi-key sequences) be in scope for v1, or single keys only until a user
  needs more? Proposed: single keys for 0.2, chords deferred.
- Where exactly the catalog type lives if a future non-`stikk-core` consumer needs it (e.g. a
  completions generator) — `stikk-core` is proposed, but a tiny `stikk-actions` crate is an
  alternative if the dependency direction gets awkward. Decide when the second consumer appears, not
  before.
- Localization: action *ids* are never translated (`NFR-I02`); action *display names* are (`NFR-I01`).
  The catalog must separate the two — id vs. localizable label — and this RFC records that split as a
  requirement on the catalog's shape.
