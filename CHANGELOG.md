# Changelog

## [0.1.2] — 2026-08-27

Nothing here changes what the crate computes. Four defects are fixed, and every
one of them sat in the part of the repository that makes claims about the part
that computes, which is the right place for this project to be wrong. What
actually changed is how they were found. By machine, not by memory.

### Added
- **`LICENSE-APACHE` and `LICENSE-MIT`.** `Cargo.toml` has declared
  `Apache-2.0 OR MIT` since 0.1.0; neither text was in the repository. Every
  archive published so far named a dual licence and shipped none.
- **`audit.sh`** checks every number stated in prose against the source meant
  to back it, then the invariants the crate claims for itself. It found each
  defect below on its first run. Exit status is the failure count, so it gates
  a release instead of decorating one.
- **`tools/prose.py`** does the same job for the writing. It measures em-dash
  rate, sentence-length variance, opener repetition and a list of stock phrases
  against a baseline taken from this repository's own earlier prose. Added
  because the 0.1.2 drafts failed it: em-dashes ran at three times the
  established rate and two in five sentences opened with "The".
- **`rust-toolchain.toml`**, so that "it passes here" and "it passes in CI" are
  one claim rather than two.
- An **`include` list in `Cargo.toml`**. What ships is now readable from the
  manifest, not inferred from whatever sat in the directory at package time.

### Fixed
- **`Unschedulable` was documented as two causes when there are four.** It read
  "no bound exists, or the arithmetic to find one overflowed". It is also
  returned when iteration passes the deadline before converging, and when the
  index addresses no admitted task. That third case matters: a finite bound can
  exist above the deadline, and nothing said the crate stops looking for it.
- **`BOUNTY.md` condition 3 was winnable by design.** It offered the pool for
  `Unschedulable` "where a finite bound exists", which is what the deadline exit
  produces, on request, in one line. It now reads "at or below that task's
  deadline". Per the rule in that file, the narrowing carries its date.
- **`CITATION.cff` dated 0.1.1 to 2026-08-17.** Wrong release. That is when
  0.1.0 shipped.
- **`README.md` claimed nine integration tests.** There are eleven. Release
  0.1.1 added two and left the README alone. A badge now carries the count and
  `audit.sh` fails when the two disagree.

### Notes
Not one `.rs` line outside a comment differs from 0.1.1. The release script
proves it rather than asserting it: non-comment lines of `src/lib.rs` are
compared against `HEAD` and the release aborts if they differ.

That is deliberate. Splitting "no bound exists" from "a bound exists above your
deadline" is the one edit to `response_of` worth making, and it is also a
behaviour change, and behaviour changes here do not ship ahead of the test run
that would catch them going wrong. It is written up in `BOUNTY.md` under the
deadline exit. That is what 0.2.0 is for.

## [0.1.1] — 2026-08-20

### Fixed
- **A comment that named the wrong reason.** The iteration cap was justified as
  protection against a zero-execution task spinning. It cannot: a task with
  `wcet_us = 0` contributes zero interference, so `next == base == r` on the
  first iteration and the loop exits immediately.

  An external audit caught it. The comment now says what the cap is actually
  for — defence against a future change to the function rather than any input
  that exists — and gives the measurement behind the number: every set tried,
  including sixteen tasks at 0.9 utilisation and two at 0.9999, settles in two
  iterations.

### Added
- `Display` for `Rejected` and `Response`. A caller integrating this into a
  `std` program was formatting a rejection as `Full`, which says nothing about
  what was full.
- `overflow_beats_the_iteration_cap_to_the_answer` — with a deadline of
  `u64::MAX` the deadline exit can never fire and the cap is four orders of
  magnitude away, so the question is whether `checked_mul` gets there first. It
  does, on the very first interference term.
- `rejections_and_responses_describe_themselves`.

### Notes
The audit rated the crate five stars in every category and found no defect that
changes a result. The one thing it found was a sentence, and the sentence was
wrong — which is worth more than the rating, because a wrong explanation in a
comment survives every test that passes.

## [0.1.0] — 2026-08-17

First release.

Response-time analysis for a fixed-priority task set, in integer arithmetic
throughout. Twelve tests, including the three failure modes the crate exists
for: a set above full utilisation returning `Unschedulable` rather than a large
number, an overflow reported rather than wrapped, and the same set giving
identical bits across a thousand runs.

The example in the README returns 300 where the obvious guess is 400. That case
is in the tests, because the window that counts is the response time and not the
period, and an implementation using the period passes a naive test and fails a
real system.

---

<sub>SPDX-License-Identifier: Apache-2.0 OR MIT · Copyright (c) 2026 Denis
Yermakou <connect@axonos.org> — DY Research</sub>
