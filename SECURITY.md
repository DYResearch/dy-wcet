<!--
SPDX-License-Identifier: Apache-2.0 OR MIT
SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
-->

# Security

## What a vulnerability means here

This crate has no network, no filesystem, no allocation and no `unsafe`. The
realistic failure is not a compromise — it is **a wrong number that looks
right**.

A task set that returns `Bounded(r)` where the true response time exceeds `r`
is the serious case, because a caller acts on it. If you find one, it is a
security issue in every sense that matters to somebody shipping on this.

## Reporting

connect@axonos.org, or a public issue.

A public issue is fine and usually better. A wrong bound is not exploitable by
a third party — it misleads the person running the analysis, and the sooner
other users see it, the better. There is nothing to coordinate.

If you would rather send it privately, that is respected without argument.

## What you can expect

An answer within a few days, from one person, without a triage process.

If the finding is right, a test reproducing it lands before the fix, so the
repository records what was wrong rather than only that it was fixed.

A claim can be wrong without any code being wrong. If what you find is a number
in a document that the source does not support, it becomes a check in
`audit.sh` rather than a fix, and the same class of mistake then fails the next
build instead of waiting for the next reader to notice it. Four of those were
found on the first run.

---

<sub>© 2026 Denis Yermakou — DY Research · connect@axonos.org</sub>

