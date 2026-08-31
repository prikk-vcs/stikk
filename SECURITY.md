# Security Policy

stikk is a front-end for the prikk version control system. Its security posture is documented in
full in the threat model at
[`docs/src/reference/threat-model.md`](docs/src/reference/threat-model.md); this file is the entry
point and the reporting channel.

## The one thing to know

stikk **holds no repository authority and no secrets.** It cannot write repository bytes (only prikk
can), and it never possesses signing key material — it reads `PRIKK_*_SEED` *presence* only, never
their values, and prikk reads the seeds itself when it signs. Most classic VCS-tool risks (corrupting
history, forging a signature, leaking a key through the tool) are therefore out of reach by
construction. The threat model concentrates on what remains: the seam to prikk, untrusted content
rendered as UI, stikk's own state files, and the ways a front-end can *mislead* a user into an unsafe
prikk action.

Two invariants are enforced by test, not just review:

- The seam's environment module materializes **no** signing-key value — only presence.
- stikk's path resolver refuses any repository-internal write target. This is the *primary* control,
  because prikk has no general foreign-file scan of `.prikk/` to catch a stray file.

## Reporting a vulnerability

Please report suspected vulnerabilities **privately**, not as a public issue:

- Open a **GitHub private security advisory** on this repository (Security → Advisories → Report a
  vulnerability), or
- email the maintainer (see the repository owner's public profile).

When reporting, describe the class of problem and how to reproduce it. If the issue could expose
signing key material, repository content, or a way for stikk to write into a repository, say so
prominently. Please do not include working exploit steps in a public channel.

We aim to acknowledge a report within a few days and to keep you informed as we investigate.

## Scope

In scope: stikk's own code and its handling of prikk output, configuration, and state. Out of scope:
vulnerabilities in prikk itself (report those to the prikk project), and the confidentiality of the
channel over which a user moves prikk artifacts (prikk moves no bytes; the channel is the user's).
