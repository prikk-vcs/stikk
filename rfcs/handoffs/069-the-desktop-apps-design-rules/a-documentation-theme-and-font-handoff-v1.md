# Handoff A — RFC 069: the documentation, the theme and the font (v1)

**Companion to:** [RFC 069](../../accepted/069-the-desktop-apps-design-rules.md).
**Issued** 2026-11-27. **0.23.0 increment 1.**

> **What this is.** RFC 069 writes rules, and RFC 070 builds them. This handoff does the two things RFC 069 needs
> before RFC 070 starts:
> - **the documentation of rule 14,** a docs change in the repository;
> - **three measurements,** none of which touches the owner's settings or opens a window on their desktop.
>
> **Sanctioned:**
> - in the repository: `docs/src/reference/requirements.md`, `external-design.md`, `internal-design.md`,
>   `README.md`, and RFC 001's Status line;
> - outside it: the prototype (`.git-exclude/reports/065-gui-toolkit/snora/proto/`), and a new
>   `.git-exclude/reports/069-design-rules/`.
>
> No crate changes, and no new dependency.

## 0. Read first

- **RFC 069, whole.** Rules 2, 3, 4 and 14 are the ones this handoff serves.
- **RFC 065's *Outcome*,** and its report's M4, M5 and M7 sections.
- **The owner's GUI rules** (`.git-exclude/rules/project-instructions-gui-common.md`). They govern §2's real-window
  step.
- **The fixed-window method** (`snora/nested.sh`, `assert-size.sh`): a real-window step runs in the nested compositor
  at an asserted 1280×720.

## 1. The documentation (rule 14)

**Write the text RFC 069 gives, in the house style.** Where RFC 069 quotes a sentence, use it as written. Where it
describes a change, make the change and nothing more.

| File | Change |
|---|---|
| `docs/src/reference/requirements.md` | `NFR-A02` as rule 14 words it |
| `docs/src/reference/external-design.md` | `GU-08` the same; a new **`GU-10 — The desktop app's design rules`**, one paragraph that points to RFC 069 and names its fourteen rules by title; the traceability table row for `GU-06/07/08` gains `GU-10` |
| `docs/src/reference/internal-design.md` | `FE-03`: the toolkit is Snora on iced (RFC 065), with rule 14's condition |
| `README.md` | rule 14's statement about screen readers, placed where the README describes the front ends. If it has no such place, beside the first mention of the desktop app; say where you put it |
| `rfcs/done/001-frontend-toolkit-selection.md` | an amendment note in its Status line, dated 2026-11-27: the acceptance's screen-reader gate reads *"when the toolkit exposes them"*, per RFC 065's outcome and RFC 069 rule 14 |

**Gates:** the repository's own, all of them. The rustdoc gate checks source doc comments, and this changes none, but
run it anyway; it's never exempted. Then `cargo test -p stikk-real-binary --test rfc_index`, which checks the RFC
index and every RFC's Status line.

**One commit,** `docs: …`, pushed after `git log origin/main..main` shows only it.

## 2. The measurements

Results go in `.git-exclude/reports/069-design-rules/`. Each one has a script whose output is the record, nothing
added by hand.

### 2.1 Does iced read the desktop's light or dark setting here?

**The question:** does `iced::system::theme()` return the mode this session's portal reports? And when no portal is
reachable, does it return no preference, so rule 3 falls back to `light_aaa`?

- **The truth to compare against:** read the setting first, read-only. Use the portal's
  `org.freedesktop.appearance color-scheme` through `gdbus call` or `busctl`, and **record the command and its
  output.**
- **Two runs of the prototype,** each logging `system::theme()`'s answer and the preset it would choose:
  1. the session as it is;
  2. the same with `DBUS_SESSION_BUS_ADDRESS` unset, so no portal is reachable.
  
  The second is **the run that fails it** for any claim that "it always reports a mode".
- **Level:** if `system::theme()` answers in `iced_test`'s headless runtime, stay headless. If it needs a real event
  loop, use the nested compositor (it's on the same session bus), with no keys sent.
- **The owner's setting is read, never changed.** Don't toggle it, not even to restore it.

### 2.2 Does naming the interface font make iced draw it?

**The question:** with the font named explicitly, does iced draw that face, and not Open Sans (RFC 069 F1)?

- **The name:** the face fontconfig resolves for `sans-serif`. Record `fc-match sans-serif`. On this machine it's
  Noto Sans.
- **Logic level:** ask the font database iced uses (`iced_graphics`'s font system, through `cosmic-text`) for that
  family by name. Record the face it returns. Then do the same for `Font::DEFAULT`, which **must** return Open Sans
  here. That's the run that fails it.
- **Real level, one capture:** the prototype with its default font set to the named family, in the nested
  compositor at an asserted 1280×720. One capture of Orientation, and one with the default font unchanged, which
  is the baseline.
  - **Read the words in both captures.** Then say what tells the faces apart: a glyph shape (Noto Sans's and Open
    Sans's `g`, `a` and `1` differ), or a measured advance.
  - Don't compare numbers from two different renders as if they were one measurement.
- **Windows (Segoe UI)** is RFC 070's first Windows CI job, not this handoff's. **macOS** is unverified, and the
  report says so.

### 2.3 Which fonts bracket the label-fit guard?

**The question:** which installed sans-serif fonts are the narrowest and the widest, so that a label fitting both
fits any reasonable interface font (RFC 069 rule 2)?

- **Candidates:**
  - every sans-serif family installed here;
  - **separately,** the families on GitHub's `ubuntu-latest` image, from the runner image's published software
    list. Cite its URL and date.
- **Measure:** each family's average advance width over one fixed sample string. Use the catalog's longest English
  labels, recorded verbatim, at 16 px, through the same font system iced uses.
- **Report:** the full ranking. Then propose the narrowest and the widest that are **both available on the CI
  runner**, since that's where the guard runs.

## 3. Evidence plan (the owner's GUI rules)

| Question | Level | Evidence |
|---|---|---|
| §1: docs say what rule 14 says | review | the diff |
| §2.1: the theme | headless if possible, else real in the nested compositor | a record per run; the portal's own answer beside it |
| §2.2: the font by name | logic, then one real capture, plus one baseline | the database's answers; two captures, read |
| §2.3: the bracket | logic | the ranking, by script |

**The repeat rule applies:** a capture set repeated a second time means ask before a third. **Nothing runs on the
owner's desktop.** The nested compositor is a window there, but nothing in it takes their focus or reads their keys.
`account-keys` holds that.

## 4. Submit

- **The documentation:** `.git-exclude/review-request/069-the-desktop-apps-design-rules-a/review-request-v1.md`,
  with the commit and its CI.
- **The measurements:** in the same request, each with its record verbatim and what it shows.
- **Say what was not measured,** and why.
