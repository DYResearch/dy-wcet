# Changelog

## [1.2.1] — 2026-08-27

The first release with a frozen API, and the first that analyses task sets
real systems actually have. Release jitter is in the recurrence, an
unschedulable answer says which of four things went wrong, and the crate will
search for a priority ordering rather than only grading the one you brought.

### On the version number

This jumps from 0.1.2 with no 1.0.0, 1.1.0 or 1.2.0 behind it, and a project
that grades its own numbers should not leave that unexplained. There were no
such releases. The number was chosen to mark an API commitment rather than to
imply a history, and the tags in this repository are the record: `v0.1.0`,
`v0.1.1`, `v0.1.2`, then this. Nothing is missing; nothing was withdrawn.

From here, `Task`, `TaskSet`, `Response` and `Unbounded` are stable. Breaking
them again means 2.0.0.

### Added
- **Release jitter.** `Task::jitter_us` enters the recurrence in its extended
  form, `wⁿ⁺¹ = C + B + Σ ⌈(wⁿ + Jⱼ)/Tⱼ⌉ · Cⱼ`, with `R = w + J`. Until now
  jitter was listed as outside the model, which meant most real task sets were
  outside it too. A test pins that zero jitter reproduces Joseph and Pandya
  exactly, and another shows 300 µs of upstream jitter buying a whole extra
  preemption that the old analysis reported away.
- **`Unbounded`, replacing the single `Unschedulable`.** Four reasons, told
  apart: `NonConvergent`, `ExceedsDeadline(u64)`, `Overflow`, `NoSuchTask`.
  The second carries the number. A task that converges at 400 µs against a
  350 µs deadline now says so, and by how much, where before it said nothing.
- **`TaskSet::optimal_priority_order`** — Audsley's assignment. If any
  fixed-priority ordering of the set meets every deadline, this finds one. It
  returns the ordering rather than applying it, because a set that silently
  reordered itself would hide the assumption its caller arrived with.
- **`TaskSet::slack_of` and `max_wcet_increase`** — sensitivity. How much
  headroom a task has, and how much execution time it could gain before
  something breaks. The search re-analyses the whole set, because raising one
  execution time can sink a lower-priority task, and an answer that checked
  only the task being changed would be wrong in the flattering direction.
- **`Task::new` with chaining setters**, and `Task::name`. Construction no
  longer means remembering the order of four `u64` fields.
- **`TaskSet::utilisation_through`, `liu_layland_bound_ppm`,
  `passes_utilisation_bound`, `first_failure`, `get`, `iter`.**
- **Eight property tests** over roughly twenty-eight thousand generated task
  sets, with no dependency added: the generator is thirty lines of linear
  congruence, seeded, so a failure reproduces anywhere. They check that a
  response never falls below the work it contains, that jitter never shortens
  one, that Audsley's answer actually holds, and that sensitivity reports the
  last value that fits rather than one past it.
- **Five Kani harnesses** in `kani/`: a bounded answer never exceeds its
  deadline, every unbounded variant fails every comparison, the recurrence
  terminates without panicking, a lone task pays only for itself, and an index
  past the end is named rather than guessed.

### Changed
- **Convergence is decided before the loop runs**, from utilisation through the
  priority level, rather than discovered by exhausting the iteration cap. The
  cap remains as defence against a future change to that decision.
- **The search now runs to the fixed point even past the deadline.** That is
  what lets `ExceedsDeadline` carry a real number. Earlier versions stopped at
  the deadline and could not have said how far past it the answer lay.

### Removed
- `Response::Unschedulable`. Match on `Response::Unbounded(_)` for the same
  meaning, or on the reason for more.

### Migrating
```rust
// 0.1.x
let t = Task { wcet_us: 200, period_us: 1000, deadline_us: 1000, blocking_us: 20 };
match set.response_of(1) {
    Response::Bounded(r) => r,
    Response::Unschedulable => panic!(),
};

// 1.2.1
let t = Task::new(200, 1000).blocking(20);
match set.response_of(1) {
    Response::Bounded(r) => r,
    Response::Unbounded(why) => panic!("{why}"),
};
```

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
