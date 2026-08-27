<div align="center">

# Embedded Timing Audit

### One problem, traced end to end, in writing.

Denis Yermakou · [connect@axonos.org](mailto:connect@axonos.org)

</div>

---

Focused written audits of hard problems in **embedded timing, concurrency,
schedulers, race conditions, and worst-case execution time**.

This is the analysis behind [AxonOS](https://github.com/AxonOS-org), an
open-source hard-real-time layer for brain–computer interfaces, and
[`dy-wcet`](https://github.com/DYResearch/dy-wcet), a response-time analysis in
integer arithmetic with every expected value in its tests derived by hand.

The defects this work is about do not crash. They produce a plausible number
that passes a test written by whoever wrote the bug.

---

## $2,400 — fixed scope

One clearly-defined, reproducible issue. Asynchronous, written delivery, within
five working days.

### What you provide

- **The problem.** One issue — an intermittent timing failure, a suspected
  race, a scheduler or liveness question, or a WCET bound you need
  sanity-checked.
- **Access to the relevant source** — a repository, the specific files, or a
  minimal reproducer — and any logs or traces you already have.

### What you get

A markdown report containing:

- the failure or behaviour chain traced end to end through the source;
- a clear line between what the evidence supports and what remains hypothesis,
  marked as such rather than blended together;
- concrete next steps, each naming what would confirm or kill it, or a proposed
  fix where the evidence carries that far.

[**Embassy #6528 — RP2350 lost-alarm timing analysis**](case-studies/embassy-6528.md)
is the standard of delivery. It is a complete audit, published in full, not an
excerpt of one.

### Where it stops

So that the fixed price stays fixed:

- **One issue.** Source-level and reasoning-level analysis.
- **Not included:** hardware instrumentation, running your build, multi-issue
  reviews, or ongoing support.
- **If the problem is larger than it looked**, I say so before starting and
  quote the rest separately. There are no surprise charges, and there is no
  version of this where the scope quietly expands and the invoice follows.

---

## Larger work — from $8,000

Deeper audits, hardware-in-the-loop verification, multi-issue reviews and
retained consulting, quoted after a short scoping exchange.

Scoping costs nothing. If the answer is that you do not need this, that is the
answer you get.

---

## Contact

- **Email** — [connect@axonos.org](mailto:connect@axonos.org)
- **Web** — [axonos.org](https://axonos.org) · [dyresearch.github.io](https://dyresearch.github.io)
- **LinkedIn** — [denis-yermakou](https://www.linkedin.com/in/denis-yermakou)

---

<div align="center">

**DY Research** — hard real-time and embedded systems.

© 2026 Denis Yermakou

</div>
