# Changelog

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


