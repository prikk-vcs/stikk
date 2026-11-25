# Handoff A — RFC 065: the remaining measurements, under the owner's GUI rules (v1)

**Companion to:** [RFC 065](../../accepted/065-choosing-the-gui-toolkit.md), its measurements M1–M9 and triggers T1–T5.
**Issued** 2026-11-25. **0.23.0 increment 0.** **Not a release, and nothing in the repository changes.**

> **Why this comes to you.** The architect ran M2, M3 and the RTL check, and also wrote the scripts and judged the
> results. The owner ruled that the remaining runs are the dev team's, and the architect reviews them. That separates
> the one who measures from the one who judges. The architect's runs stay on record as the architect's; say so if you
> find a fault in them.
>
> **Sanctioned:** everything under `.git-exclude/reports/065-gui-toolkit/snora/`: the prototype (`proto/`), the
> scripts, `steps/` and `evidence/`. Change any of them as the measurements need, and say what you changed and why.
> **Not sanctioned:**
> - any file in the repository (no commit);
> - `stikk-app` or any other crate. The prototype reads them by path, and a fault found there is reported, not fixed;
> - the owner's settings, niri's configuration, the input method's settings left changed, and any window that is not
>   the instance under test or its control.
>
> **No conclusions on triggers.** Report what each measurement shows against its question. The architect recommends,
> and the owner rules.

---

## 0. Read first

1. **The owner's GUI rules:** `.git-exclude/rules/project-instructions-gui-common.md`. They govern every run, as
   written.
2. **The owner's niri instructions (2026-11-25):**
   - the window size is set with `niri msg action set-window-width/height --id <the instance's window>` (niri ignores
     the app's own request);
   - a run that sends keys takes focus once with `niri msg action focus-window --id <the instance's window>`.
3. **`snora/session.sh`, the one session script.** It isolates HOME and XDG, checks for one instance, sets and asserts
   1280×720, takes focus only when the steps send keys, checks the pid and foreign input before every key, asserts
   after every step, and captures through niri. **Read it before trusting it:** it has been fixed three times
   (2026-11-25), and each fix is commented. Use it for every real-UI run.
4. **What is already measured, and by whom (the architect):**
   - `evidence/m2-keyboard/`: 18 of 18 steps;
   - `evidence/m3-reading-gate/` and `m3-findings.md`: 24 of 24 steps, on `stikk-app` 0.22.1;
   - `evidence/rtl-check-{ltr,rtl}/` and `rtl-findings.md`.
5. **What went wrong in the architect's own runs, so it does not happen twice:**
   - **A batch in zsh passed `VAR=a VAR2=b` as one word.** Every "preset" was drawn as light_aaa, and the good
     dark and high-contrast evidence was overwritten. `run-specimens.sh` now runs under bash and stops if a run's
     recorded preset or scale differs from what was asked. **Never `rm -rf` an evidence directory before the new run
     has passed its own checks.**
   - The specimen did not fit 1280×720 at 150 %. It is now 16 px apart.
   - `m4-measure.py`'s label margins did not grow with the zoom. They are now scaled.
   - **The owner may be at the desktop.** A run that takes focus can catch their typing. The foreign-input check
     stops such a run. **Before any run that sends keys, ask the owner through the architect,** and state how long
     it takes.
6. **The repeat rule:** a capture set run a second time needs the architect's OK before a third. Count from your first
   run. **M4's and M5's sets have already had their two runs**, so their next is a third, and it has the architect's
   OK in this handoff.

## 1. The measurements

| # | Question (RFC 065) | Level and how | Evidence |
|---|---|---|---|
| **M2h, M3h** | **Headless:** does `iced_test` drive the prototype through M2's keys and M3's gate, with the program's state asserted after each key? | **Headless, the cheapest level** (the rules' first choice; RFC 065 names `iced_test`). Add tests to `proto/`. **M3h covers the regression:** a Loading card fully seen, then its confirmation arrives unread, Enter refused, ↓ to the end, Enter confirms. This is also the first proof that RFC 070 can test its screens headless with Snora | the test output, verbatim; the test names |
| **M4** | Is the text readable and the focus visible, in each preset? | `run-specimens.sh`: four presets at 100 %, and `targets.min = 44`. **Add what RFC 065 asks and the specimen lacks: a focused control,** with the ring's contrast against what is behind it (3 : 1), from pixels. Focus by Tab needs keys, so it needs focus and the owner's OK. Or draw the ring without focus if Snora allows it, and say which | each `measure.txt`, verbatim; the captures |
| **M5** | Can the text be made bigger without clipping or overlap? | `run-specimens.sh`: 115, 130 and 150 % (knotra's steps, F3), plus **200 %** (RFC 065's stress case). **Plus one real screen,** not just the specimen: the commit confirmation at 150 % and 200 %, with whether any line is cut. M3's capture shows the scrollbar covering a line end, so measure that. A measurement of the layout is preferred to a picture, where you can get it | the script's output; the captures |
| **M6** | Does Japanese input reach the program on Linux? | `wtype` bypasses fcitx5 for every application (the earlier control). So this is **the owner's two-minute check, prepared by you:** a session that opens the commit message field under the rules, waits once (one `wait` step, no loop) while the owner types a word you name in Japanese through Mozc, and asserts the state log holds it. Both the preedit and the commit are reported. **The input method is left exactly as it was found** (`fcitx5-remote` before and after, recorded) | the record and state log; the owner's word |
| **M7** | What does a screen reader get? | The AT-SPI tree of **the prototype's application only**, and of the GTK control's: **never list or record other applications on the desktop**, which would put the owner's windows in the evidence. `atspi_dump.py` lists every application today, so change it to select by the instance's pid. Then Orca once, with its debug log filtered to the prototype. Expected: nothing from iced. Measured, not assumed | the two dumps; the Orca log excerpt |
| **M8** | Can it be packaged as stikk is? | `ldd` and the highest `GLIBC_` symbol version (`objdump -T`) of the release build; the static musl build's start, as one session that records its error; the Windows `.exe` through mingw (M1's). **Report the glibc floor:** today's build needs 2.43 (the architect's earlier reading), so say what building on an older system would take. RFC 071 designs the answer, not you | command output, verbatim |

**Always in each run's record:** the session line (pid, window, size, backend, preset, scale, direction, prikk), the
`stikk-app` version the prototype was built against (0.22.1), and the window size that produced each capture.

## 2. Submit

Package it to `.git-exclude/review-request/065-choosing-the-gui-toolkit-a/review-request-v1.md`. It should hold:
- one section per measurement: the question, the level, the evidence (verbatim where it is output), and what it shows
  against the question. **No trigger verdicts;**
- what you changed in the scripts and the prototype, and why;
- every run you made, including the ones that aborted, with why (the repeat rule counts them);
- anything you found wrong in the architect's runs or scripts.
