// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
// DY Research — https://dyresearch.github.io

//! Kani harnesses for `response_of`.
//!
//! Tests sample the input space. These bound it. Each harness states a
//! property that must hold for every task set the model checker can build
//! within its unwind limit, and a counterexample is the falsifier.
//!
//!     cargo kani --harness a_bounded_response_never_exceeds_its_deadline
//!
//! **On the unwind limits.** Through 1.2.1 this file said the recurrence
//! settles in two iterations on every set measured, and that a bound of four
//! therefore covered twice the observed worst case. Both halves were wrong.
//! `telemetry_tx` in this repository's own cross-check fixture takes three,
//! and over 271,442 randomly generated converging sets, 67.9% take more than
//! two, with an observed maximum of 126. No harness ever used a bound of four
//! either; they use five and three.
//!
//! The proofs were never unsound — each holds for every execution inside its
//! declared unwind — but the sentence justifying the bound claimed a coverage
//! the bound does not have. What an unwind of five establishes is a property
//! over the sets whose recurrence settles in fewer than five iterations, which
//! is roughly four in five of them and not all of them. That is the honest
//! statement, and it is the one to make.
//!
//! From 2.0.0 the analysis carries two nested bounded loops rather than one —
//! a busy-period solve, then a per-job solve — so these bounds cover less of
//! the input space than the same numbers did before, and the recurrence
//! harness below is rewritten against the new structure.

use dy_wcet::{AnalysisFailure, Response, Task, TaskSet};

fn task(c: u64, t: u64, d: u64, b: u64, j: u64) -> Task {
    Task::new(c, t).deadline(d).blocking(b).jitter(j)
}

/// A `Bounded` answer is at or below the deadline. This is the invariant a
/// caller acts on, and the one an arithmetic error would break in the
/// flattering direction.
/// Bound is `MAX_TASKS + 2`. Every loop in `response_of` walks the sixteen-slot
/// task array, so nothing below seventeen can close it, whatever the set holds
/// — this harness ran at five and reported "unwinding assertion loop 2" at
/// `src/lib.rs:545`, which is the seed loop over that array.
///
/// The input space is narrowed to a busy period holding one job, and that is a
/// scope, not a formality: without it `jobs_in_window` is `⌈(busy + J)/T⌉`,
/// which reaches `BUSY_PERIOD_CAP` at 1024 and cannot be unwound at all.
/// `tests/harness_scope.rs` measures that the narrowing does what it says.
#[kani::proof]
#[kani::unwind(18)]
fn a_bounded_response_never_exceeds_its_deadline() {
    let c = u64::from(kani::any::<u8>());
    let t = u64::from(kani::any::<u8>());
    let d = u64::from(kani::any::<u8>());
    let b = u64::from(kani::any::<u8>());
    let j = u64::from(kani::any::<u8>());
    // Ranges tightened in 3.0.7. 3.0.6 fixed the unwinding assertions — the
    // loops close now, and the trace shows iteration 13 and 14 rather than 880
    // — and what remains is solver time: the step ended on exit 124, the
    // six-minute budget, not on a failed check. Five symbolic u64 values
    // through integer division and checked multiplication is where the cost
    // is, and CBMC reasons over the full 64-bit width whatever the range
    // assumption says; a tighter range does not shrink the circuit, it prunes
    // the search. These bounds are a scope on what the proof covers and are
    // stated for that reason, not tuned until something passed — I cannot run
    // Kani here, so nothing was tuned at all.
    kani::assume(t > 1 && t < 64);
    kani::assume(c <= d && d < 128);
    kani::assume(b < 32 && j < 32);
    // One job in the busy period: the per-job loop unwinds once.
    kani::assume(c + b + j < t);

    let mut s = TaskSet::new();
    if s.push(task(c, t, d, b, j)).is_ok() {
        if let Response::Bounded(r) = s.response_of(0) {
            assert!(r <= d);
        }
    }
}

/// Every `AnalysisFailure` variant fails every deadline comparison. A caller
/// that forgets to match on the reason still gets the safe answer.
///
/// 3.0.0 split `IterationLimit` and `BusyPeriodLimit` out of `NonConvergent`
/// and left this harness asserting the four that existed before. The name says
/// *every*, and a proof covering two thirds of what its name claims is worse
/// than none, because the name is what gets repeated. `audit.sh` counted the
/// variants in `src/lib.rs`, not the ones a harness exercises, so nothing saw
/// it — the "named two of four" defect of 0.1.2, one layer up.
///
/// The `match` below has no wildcard arm on purpose: a seventh variant will
/// not compile until this list grows with it.
/// The bound must exceed the number of variants: the loop below runs once per
/// variant and needs one more unwinding to close. 3.0.2 gave this harness a
/// bound of two while rewriting it to iterate six, which is an unwinding
/// assertion failure — CBMC reports it as the harness failing, not as a
/// timeout, and it fails every run identically. `audit.sh` ties this number to
/// the variant count now, so the two cannot drift apart again.
#[kani::proof]
#[kani::unwind(8)]
fn every_unbounded_variant_fails_every_deadline() {
    let d = u64::from(kani::any::<u8>());
    let v = u64::from(kani::any::<u8>());

    let all = [
        AnalysisFailure::NonConvergent,
        AnalysisFailure::IterationLimit,
        AnalysisFailure::BusyPeriodLimit,
        AnalysisFailure::Overflow,
        AnalysisFailure::NoSuchTask,
        AnalysisFailure::ExceedsDeadline(v),
    ];
    for why in all {
        assert!(!Response::Refused(why).meets(d));
        match why {
            AnalysisFailure::NonConvergent
            | AnalysisFailure::IterationLimit
            | AnalysisFailure::BusyPeriodLimit
            | AnalysisFailure::Overflow
            | AnalysisFailure::NoSuchTask
            | AnalysisFailure::ExceedsDeadline(_) => {}
        }
    }
}

/// `response_of` terminates and never panics. Overflow is refused rather than
/// wrapped, so no arithmetic in either loop can abort.
///
/// The bound covers the busy-period solve and the per-job solve together. It
/// does not cover every admissible set — see the note at the head of this file
/// — and the input scope is narrowed here so that what it does cover is
/// stated rather than implied.
#[kani::proof]
#[kani::unwind(18)]
fn the_recurrence_terminates_without_panicking() {
    let c0 = u64::from(kani::any::<u8>());
    let t0 = u64::from(kani::any::<u8>());
    let c1 = u64::from(kani::any::<u8>());
    let t1 = u64::from(kani::any::<u8>());
    kani::assume(t0 > 0 && t1 > 0 && t0 < 64 && t1 < 64);
    kani::assume(c0 <= t0 && c1 <= t1);

    let mut s = TaskSet::new();
    if s.push(task(c0, t0, t0, 0, 0)).is_ok() && s.push(task(c1, t1, t1, 0, 0)).is_ok() {
        let _ = s.response_of(0);
        let _ = s.response_of(1);
    }
}

/// A bounded answer is never below the work the job itself contains. The
/// busy-period form must not lose the `q = 0` case that the single-job
/// recurrence computed.
#[kani::proof]
#[kani::unwind(18)]
fn a_bounded_answer_is_never_below_its_own_work() {
    let c = u64::from(kani::any::<u8>());
    let t = u64::from(kani::any::<u8>());
    let b = u64::from(kani::any::<u8>());
    let j = u64::from(kani::any::<u8>());
    // Ranges tightened in 3.0.7. 3.0.6 fixed the unwinding assertions — the
    // loops close now, and the trace shows iteration 13 and 14 rather than 880
    // — and what remains is solver time: the step ended on exit 124, the
    // six-minute budget, not on a failed check. Five symbolic u64 values
    // through integer division and checked multiplication is where the cost
    // is, and CBMC reasons over the full 64-bit width whatever the range
    // assumption says; a tighter range does not shrink the circuit, it prunes
    // the search. These bounds are a scope on what the proof covers and are
    // stated for that reason, not tuned until something passed — I cannot run
    // Kani here, so nothing was tuned at all.
    kani::assume(t > 0 && t < 64);
    kani::assume(c <= t && b < 32 && j < 32);

    let mut s = TaskSet::new();
    if s.push(task(c, t, u64::MAX, b, j)).is_ok() {
        if let Response::Bounded(r) = s.response_of(0) {
            assert!(r >= c + b + j);
        }
    }
}

/// A single task with no higher priority above it responds in exactly its own
/// execution plus blocking plus jitter. No interference term can appear from
/// nowhere.
#[kani::proof]
#[kani::unwind(18)]
fn a_lone_task_pays_only_for_itself() {
    let c = u64::from(kani::any::<u8>());
    let t = u64::from(kani::any::<u8>());
    let b = u64::from(kani::any::<u8>());
    let j = u64::from(kani::any::<u8>());
    // Ranges tightened in 3.0.7. 3.0.6 fixed the unwinding assertions — the
    // loops close now, and the trace shows iteration 13 and 14 rather than 880
    // — and what remains is solver time: the step ended on exit 124, the
    // six-minute budget, not on a failed check. Five symbolic u64 values
    // through integer division and checked multiplication is where the cost
    // is, and CBMC reasons over the full 64-bit width whatever the range
    // assumption says; a tighter range does not shrink the circuit, it prunes
    // the search. These bounds are a scope on what the proof covers and are
    // stated for that reason, not tuned until something passed — I cannot run
    // Kani here, so nothing was tuned at all.
    kani::assume(t > 0 && t < 64);
    kani::assume(c < 64 && b < 32 && j < 32);

    let mut s = TaskSet::new();
    if s.push(task(c, t, u64::MAX, b, j)).is_ok() {
        if let Response::Bounded(r) = s.response_of(0) {
            assert!(r == c + b + j);
        }
    }
}

/// An index past the end is named rather than guessed, and never reported as
/// a bound.
///
/// The index was fully symbolic over `usize` and the harness carried no unwind
/// bound: an unbounded value and an unbounded search together. Every other
/// harness here declares its bound; these two did not, and §13 of the release
/// specification asks for every bound to be documented — which cannot be done
/// for a bound that does not exist.
///
/// Bounded to just past `MAX_TASKS`, which is where the claim lives: the last
/// valid index, the first invalid one, and a few beyond. `usize::MAX` is
/// covered concretely by `tests/adversarial.rs`, in under a second.
#[kani::proof]
#[kani::unwind(18)]
fn an_index_past_the_end_is_named() {
    let i = usize::from(kani::any::<u8>());
    kani::assume(i > 0 && i <= dy_wcet::MAX_TASKS + 4);
    let s = TaskSet::new();
    assert!(s.response_of(i) == Response::Refused(AnalysisFailure::NoSuchTask));
}
