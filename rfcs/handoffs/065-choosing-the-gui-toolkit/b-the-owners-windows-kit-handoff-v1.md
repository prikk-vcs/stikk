# Handoff B — RFC 065: the owner's Windows kit (v1)

**Companion to:** [RFC 065](../../accepted/065-choosing-the-gui-toolkit.md) and [Handoff A](./a-the-measurements-under-the-gui-rules-handoff-v1.md).
**Issued** 2026-11-26. **0.23.0 increment 0.** Nothing in the repository changes.

> **What this is.** RFC 065's last measurements need the owner on a real Windows machine: about 20 minutes, by hand.
> There is no niri, `wtype` or session script there. **You prepare everything,** so the owner only follows a page
> and sends the logs back, and **you read the logs afterwards** (the owner doesn't). The architect reviews the kit
> and the page before the owner gets them.
>
> **Sanctioned:**
> - a new folder `.git-exclude/reports/065-gui-toolkit/snora/windows-kit/`;
> - the prototype, where the kit needs it;
> - the owner's page at `.git-exclude/tasks/owner/065-windows-run.md`.
>
> Nothing in the repository, and no crate.

## 1. What the owner's run must answer

| # | Question | Trigger |
|---|---|---|
| W1 | **M2 on Windows:** can the same tasks be done from the keyboard (History, ↓/↑, the action list, Escape, `c`, typing and Backspace, Escape)? | T1 |
| W2 | **M3 on Windows:** at scale 2.0, is an unread confirmation's Enter refused, does ↓ reach the end, and does Enter then commit? | T2 |
| W3 | **M6 on Windows:** does Japanese typed through the Microsoft IME (`nihongo`, Space, Enter → 日本語) reach the commit message? | T3 |
| W4 | **The presets on a Windows screen:** light_aaa and hc_dark, one look each. Is the text readable, and is the focus ring visible after Tab? | (for RFC 069; not a trigger by itself) |

**Each answer must come from the program's state log** (`STIKK_PROTO_LOG`), not from the owner's impression, as on
Linux. The owner's own words go in a short notes file only for what a log cannot hold: for example *"the ring was
hard to see"*.

## 2. What the kit holds

1. **`snora-proto.exe`,** built for `x86_64-pc-windows-gnu` from the current prototype, with its SHA-256. Keep the
   **console subsystem** for this kit: the console shows where the log went and any panic. Say so on the page, so a
   second window is no surprise.
2. **prikk for Windows:** the kit fetches `prikk-x86_64-pc-windows-msvc.zip` from prikk's 0.58.0 release and checks
   it against the release's `.sha256` before use. Don't ship a prikk binary you built yourself.
3. **`setup.ps1`:** makes a throwaway folder (under the kit), unpacks prikk there, and builds the same repository
   `make-repo.sh` builds:
   - throwaway keys;
   - eight sealed blocks;
   - one uncommitted file.

   **Isolate everything prikk and the prototype read.** Point `HOME`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA` and
   prikk's key variables at the throwaway folder, for the session only. **Read prikk's source** for where it looks on
   Windows, rather than assume.
4. **`run.ps1 <W1|W2|W3|W4> [preset]`:** sets the session's variables, starts one prototype, and when it exits,
   copies its log into `results\<run>\` with a header naming the run, the preset, the scale and the Windows version.
   **One instance:** it refuses if `snora-proto.exe` is already running.
5. **A script you run on Linux** when the results come back. It asserts each run's log against W1–W3's expected
   states, as `steps/m2-keyboard.txt` and `steps/m3-reading-gate.txt` do, and prints pass or fail per step.
6. **The window size:** on Windows the prototype's own `window_size(1280, 720)` should hold at launch. **Log the size
   the window actually got** (iced reports it), so the record states it.

## 3. The owner's page (`.git-exclude/tasks/owner/065-windows-run.md`)

Plain steps, for someone who has never seen the kit:
- how to get the kit onto the Windows machine (one zip; the owner chooses how to carry it);
- PowerShell's execution policy, if a script needs it, **for that one process only**
  (`-ExecutionPolicy Bypass` on the command line), never changing the machine's setting;
- then W1 to W4, each as numbered key presses with what the screen should show after;
- the IME step: turning the Microsoft IME on and off as the owner usually does;
- at the end: zip `results\` and send it back; how to delete the whole kit folder; nothing else is left behind.

**About 20 minutes in all.** If a step doesn't match what the page says, the owner writes down what they saw and goes
on. The log is the evidence.

## 4. Submit

`.git-exclude/review-request/065-choosing-the-gui-toolkit-b/review-request-v1.md`:
- the kit's file list with checksums;
- what you verified on Linux: the `.exe` builds, the scripts parse, and the result-checking script passes on a log you
  make on Linux with the same steps;
- **what you could not verify without Windows,** listed plainly;
- the owner's page.

The architect reviews it, then gives the owner the page.
