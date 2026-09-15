# Changelog

## [3.0.1] — 2026-09-15

3.0.0 added a gate requiring every action to be pinned to a commit, and shipped
without doing what the gate enforces. Its first run failed on ten references.

### Fixed

- **`dtolnay/rust-toolchain@stable` and `Swatinem/rust-cache@v2` are pinned**,
  at seven and four call sites. Both had been floating since before 3.0.0; the
  new `workflow` job is simply the first thing that looked. A tag or a branch is
  a name its owner can repoint, so a workflow resolving `@stable` at run time
  runs whatever that name means on the day — which is the opposite of the claim
  this repository makes about its own analysis.

  The SHAs come from the upstream refs rather than from memory:
  `git ls-remote https://github.com/dtolnay/rust-toolchain stable` and
  `git ls-remote https://github.com/Swatinem/rust-cache refs/tags/v2.8.0`.

### Note on 3.0.0

The 3.0.0 commit is signed but shows as unverified on GitHub. The signature is
sound; the commit author address is not one GitHub knows belongs to the account,
so it has nothing to check the key against. Adding and verifying that address in
the account settings marks the existing commit verified retroactively — GitHub
computes that at display time, so no history has to be rewritten.

## [3.0.0] — 2026-09-15

Breaking. The enum that says why an analysis produced no number is renamed and
split, because one of its variants was making a claim the code had not
established.

### Fixed

- **`utilisation_through` returned a wrong number rather than refusing.**
  `index + 1` was unchecked. In debug it panicked; in release it wrapped, and
  `utilisation_through(usize::MAX)` answered `Some(0)` — no load at all — for a
  set running at 12 %. Release is the profile an embedded caller ships, so the
  quiet half was the shipped half. The depth is clamped to the array length
  now, not saturated: a saturating add would read the same and `audit.sh` would
  be right to object, since saturation elsewhere in this crate yields a
  plausible wrong number instead of a refusal.

- **`NonConvergent` was returned from four places and meant one of them.**
  Utilisation above one is genuine non-convergence. The busy-period recurrence
  reaching `ITERATION_CAP`, the per-job recurrence reaching it, and a busy
  period holding more jobs than `BUSY_PERIOD_CAP` are all statements about this
  implementation, not about the mathematics — and the last is the sharpest: the
  busy period has *closed*, its length and job count are known and finite, and
  the crate answered with a `Display` string reading "utilisation exceeds one;
  no fixed point exists". The repository's own test proved it, asserting
  utilisation below one percent two lines above asserting `NonConvergent`.

  `Unbounded` is now `AnalysisFailure`, `Response::Unbounded` is
  `Response::Refused`, and `IterationLimit` and `BusyPeriodLimit` are separate
  from `NonConvergent`. One is answered by changing the task set; the others by
  raising a cap. A caller could not previously tell which.

- **Two `proofs:` keys in `.github/workflows/ci.yml`**, the first with an empty
  `steps:`. YAML keeps the last silently, so the job that ran was an accident of
  ordering; reversed, CI would have reported a green Kani job that executed
  nothing — the badge-for-proofs-nobody-ran defect this repository fixed once at
  1.2.1, reintroduced in its own workflow. A `workflow` job now fails on any
  duplicate key, any stepless job and any action not pinned to a commit.

- **`audit.sh` counted prose as code**, so a comment explaining why a forbidden
  call was *not* used registered as a use of it. It now strips line comments
  before counting.

- **`audit.sh` reported zero for a type it could not find.** The refusal-reason
  check grepped `Unbounded`; after the rename the awk range matched nothing and
  the gate announced "carries only 0 reason(s)" — which reads as a finding about
  the crate and was a finding about the grep. A missing type is now a refusal to
  score rather than a score of nothing. Zero is the answer this repository is
  least entitled to accept from a counter.

### Added

- **An independent reference scheduler** (`reference/scheduler.py`): job-level
  discrete-event execution in arbitrary-precision integers, with no `ceil_div`,
  no `W(q)`, no busy-period equation and no interference sum. It releases jobs,
  preempts, and records completions.

  This replaces an arrangement that could not have caught a defect in the crate.
  `tools/differential.py` compared a Python transcription of the recurrence
  against a Python simulation — two Python programs — and `tests/oracle.rs`
  shares `solve_w`, the same ceiling division and the same jitter grouping with
  the implementation it checks. Neither ever read the Rust.

- **`examples/rta_probe.rs`** so the campaign runs against the crate itself, and
  **`tools/campaign.py`** to drive it with boundary-weighted generation.

- **`tests/exhaustive.rs`**: Audsley against every one of the `n!` orderings for
  n ≤ 5, both directions — including that `None` really means no ordering works,
  which is the optimality claim and the half a sampled test cannot reach; and
  `max_wcet_increase` against a full linear scan, asserting it is the maximum
  rather than merely feasible.

- **`tests/adversarial.rs`**: every public entry point at `0`, `1`,
  `u64::MAX - 1`, `u64::MAX`, and the indices around `MAX_TASKS`. Run in debug,
  where a wrap panics, as well as release.

- **`tools/verification_json.py`**, which records the stages that ran and marks
  the ones that did not as `skipped` rather than omitting them.

### Campaign

220 000 generated task sets across four seeds, 621 862 individual response-time
comparisons against the independent scheduler, no mismatches.

The reference model also had to be bounded before it could be a gate. Its first
version re-sorted the ready queue on every event, which made a long busy period
quadratic; a 40 000-case run went half an hour without finishing on a workload
that takes seconds for five hundred, reporting nothing while it did. "Still
running" and "found nothing" look identical from outside. The queue is a heap
and `EVENT_BUDGET` bounds the work per case — at 150 000 events rather than the
two million tried first, because a budget for a gate has to bound time, not
merely guarantee termination.

## [2.0.0] — 2026-09-13

### The analysis, checked a second way

- **`tools/differential.py` runs the schedule the recurrence claims to
  describe.** Jobs are released from the critical instant, the processor runs
  the highest-priority ready one, and completions are measured. Over 5,347
  comparisons across sets with release jitter, blocking and deadlines past the
  period, the simulation never exceeded the computed bound, and equalled it
  every time. Sound, and on this sample exact rather than merely safe.

  An early draft reported 83 unsound results. All 83 were a defect in the
  simulation, which merged a task's queued jobs into one counter and recorded
  the completion of the last as the completion of the first. That is noted in
  the file, because a differential check that has never been wrong about itself
  has not been used hard enough.

  The Kani harnesses bound the analysis inside an unwind limit. This covers the
  sets whose recurrence takes more iterations than any such limit admits, which
  is most of them: 67.9% take more than two, with an observed maximum of 126.

### Attribution

- **Lehoczky and Tindell were not named.** The recurrence has been credited to
  Joseph and Pandya since 0.1.0, and the priority assignment to Audsley. The
  busy-period analysis that makes a deadline past the period sound is
  Lehoczky's, RTSS 1990; the jitter terms are Tindell, Burns and Wellings,
  1994. Neither appeared anywhere in the crate, in the release whose headline
  feature is their result. Both are now named in `README.md`, in the doc
  comment on the function that implements them, and in a formal `references`
  block in `CITATION.cff` alongside Joseph & Pandya, Audsley, and Sha et al.
  for the blocking term.

### CI

- **The step named "No floating point anywhere" checked one file.** It read
  `src/lib.rs` and nothing else, so a float in a test or a harness would have
  passed a gate whose name says otherwise. It now reads `src`, `tests` and
  `kani`.
- **The Kani job rebuilt the model checker on every run.** `kani-verifier` is
  now cached against its pinned version, which returns most of that job's
  thirty-minute budget to the proofs it is supposed to be running.

### Housekeeping in this release

- **The fixed-scope audit is $3,000.** It was quoted at $2,400 here and at
  $3,000 in the published writing, and a price stated twice at two figures is
  the same defect as a number stated twice at two values. `AUDIT.md`, the
  README and the landing page now agree, and the page checks the repository on
  every publish rather than trusting that they do.
- **One signature on everything.** Every source file opens with the same three
  lines, and every document closes with the same one: DY Research, Denis
  Yermakou, one address.
- **`audit.sh` stopped reporting absence as failure.** A cross-compilation
  target that is not installed says something about the machine and nothing
  about the repository, and on a toolchain without rustup it is not even
  actionable. It is a note there and a warning only where it can be acted on.


The analysis was unsound for sets whose response runs past their own period,
and had been since the deadline-beyond-period case was first permitted. This
release fixes that, and fixes four things found while checking whether anything
else was wrong.

### The defect

A task whose deadline exceeds its period can have a response time longer than
its period, and its next job is then released before the current one finishes.
The single-job recurrence charges only higher-priority interference and is
sound exactly while `R ≤ T`. This crate permitted `D > T`, advertised it, and
never checked `R ≤ T`.

```text
task 0:  C = 5  T = 10  D = 10
task 1:  C = 2  T = 5   D = 10  B = 2        U = 0.9

1.2.1:  response_of(1) = Bounded(9)  ·  is_schedulable = true  ·  slack = Some(1)
2.0.0:  response_of(1) = ExceedsDeadline(11)  ·  is_schedulable = false  ·  slack = None
```

Job two of task 1 is released at t = 5, starts at 9, is preempted at 10 by task
0's second activation, and finishes at 16 — eleven microseconds after its own
release, against a ten microsecond deadline. 1.2.1 reported one microsecond of
slack on a set that misses by one.

Wrong in the flattering direction, which is the one direction this crate exists
to refuse. Across 400,000 generated sets in the admitted region, 14,185
under-reported and 557 of those returned `Bounded` on a set that misses.

### Why nothing here caught it

`tests/properties.rs` generates deadlines up to twice the period, so roughly
half of its twenty-eight thousand sets were in the region. The bug was
generated thousands of times and asserted past, because every property checks a
lower bound, a monotonicity, or whether the implementation agrees with itself.
The Kani harness proving `Bounded(r) → r ≤ d` is true of the buggy code at any
unwind: 9 ≤ 10. `docs/CROSSCHECK.md` compares against an implementation by the
same author of the same recurrence, which the document already says is not
independence.

Nothing in this repository compared the implementation to a different
algorithm. `tests/oracle.rs` is the first thing that does.

### Changed

- **`response_of` solves the level-i busy period.** The busy-period length is
  found first as a single bounded fixed point, the job count derived from it as
  `⌈(L + J)/T⌉`, and the worst `R(q) = w(q) − q·T + J` returned. Joseph and
  Pandya's form is the `q = 0` line of this. Answers change only for sets whose
  response passed their own period; all forty-nine tests from 1.2.1 pass with
  no expected value edited.
- **`passes_utilisation_bound` checks its own preconditions.** Liu and
  Layland's bound holds for implicit deadlines under rate-monotonic priorities
  with no blocking or jitter. None of that was checked. A rate-monotonic set at
  a fifth of the bound with one constrained deadline was told it was
  schedulable while missing by five microseconds. Sets outside the theorem now
  return `false`.
- **The Liu and Layland table is floored, not rounded.** Six of sixteen entries
  sat above the true value — five by one part per million, `n = 11` by five —
  which admits a set the theorem does not cover. Four more (`n = 13…16`) were
  simply wrong numbers, drifting up to sixty-nine parts per million in the
  conservative direction. All sixteen are now `⌊n·(2^(1/n) − 1)·10⁶⌋`, with a
  test against values computed to forty digits.
- **A lone task with `C > T` is refused rather than bounded.** Utilisation is a
  floor, so for periods above 10⁶ µs a task that can never keep up could still
  report exactly full utilisation. 1.2.1 returned a bound for it.
- **`BUSY_PERIOD_CAP`**, new, at 1024. A busy period holding more jobs of one
  task than that is refused before any per-job work, for the same reason
  `MAX_TASKS` is sixteen.

### Fixed in the documentation, which described code that no longer existed

- The Kani unwind justification claimed the recurrence settles in two
  iterations on every set measured, and that a bound of four covered twice
  that. `telemetry_tx` in this repository's own cross-check fixture takes
  three; across 271,442 generated converging sets 67.9% take more than two,
  with a maximum of 126; and no harness ever used four — they use five and
  three. The proofs were sound within their declared unwind. The sentence
  justifying that unwind was not, and it is now replaced by the measurements.
- `response_of`'s doc comment stated the single-job recurrence as the whole
  analysis.
- `audit.sh` reported on `Response::Unschedulable`, removed in 1.2.1, and used
  `grep -P`, which is GNU-only and fails silently on macOS and BSD, taking the
  tab check with it.

### Found by the new tests, and fixed

`R(q) = w(q) − q·T + J` was grouped as `(w − q·T) + J`. A job of a task that
carries release jitter is released at `q·T − J` from the start of the busy
period, so `w(q) − q·T` is negative whenever that job completes before its
nominal offset. `u64` has no negative, `checked_sub` refused, and the analysis
returned `Overflow` on sets whose answer is an ordinary positive number.

```text
task 0:  C = 1  T = 10
task 1:  C = 2  T = 10  D = 100  J = 8

before:  response_of(1) = Overflow
after:   response_of(1) = Bounded(11)
```

Across 120,053 generated task and index pairs the regrouping changes 3,116 of
them, 2.6%, and every one is `Overflow` giving way to a real answer. Not one
number that the analysis already produced moves. `is_schedulable` is unchanged
throughout, because both outcomes fail `is_bounded`; what was wrong is the
diagnosis. A set told the arithmetic had been refused was in most cases a set
that misses its deadline by a stated amount, and `response_time` returned
`None` where it should have carried that amount.

`tests/oracle.rs` grouped it the same way and returned `None` at the same
point, where the harness counts a skip rather than a disagreement. So the
153,365 agreements it reports were 153,365 agreements about the region where
the two implementations were both right. A second opinion that shares the
first one's arithmetic is not a second opinion, and this is the second time in
one release that the point has had to be made in this file.

The case that surfaced it is `a_tasks_own_jitter_can_add_a_job_to_its_own_busy_period`
in `tests/boundaries.rs`, derived by hand, two tasks, five lines.

### Added

- `tests/oracle.rs` — the busy period computed a second way, growing the job
  count rather than deriving it. 153,365 exact agreements.
- `tests/differential.rs` — the 1.2.1 recurrence kept as a comparison target.
  101,105 comparisons: never smaller, and bit-identical wherever the old answer
  stayed inside the task's period.
- `tests/precheck.rs` — the utilisation bound held to its preconditions, and
  every table entry against its true floor.
- `tests/boundaries.rs` — thirteen cases on the surface around the recurrence:
  `first_failure`, `utilisation_through`, `get` and `iter`, the split between
  `bound` and `response_time`, a full set of sixteen, the busy-period cap, and
  the utilisation table past its last entry. Every expected value is derived in
  the comment above it.
- Two Kani harnesses where there were five: the termination proof rewritten for
  the two-loop structure, and a new one for the invariant the busy-period form
  has to preserve.
- `[lints.rust] check-cfg = ['cfg(kani)']` in `Cargo.toml`. The `#[cfg(kani)]`
  declaration above is a cfg rustc does not know, so every build emitted
  `unexpected_cfgs` and CI's `-D warnings` would have rejected the release
  that introduced it.
- **The harnesses are now wired into the crate.** Until 2.0.0 there was no
  `#[cfg(kani)]` module declaration, so `kani/response_bounds.rs` shipped in
  the published archive as dead code and `cargo kani` found nothing from a
  clean clone, while the README said to run it. CI now runs the proofs on every
  push; it did not before.

### Migration

Match arms are unchanged; `Unbounded` still carries the same four reasons. What
changes is which answer you get:

- A set with `D ≤ T` throughout gets the same numbers as 1.2.1.
- A set whose response passed its own period gets a larger, correct number, and
  may move from `Bounded` to `ExceedsDeadline`. If a set became unschedulable
  on upgrade, it was unschedulable before.
- `passes_utilisation_bound` now returns `false` for sets outside Liu and
  Layland's preconditions. It was never safe to act on those answers.

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
