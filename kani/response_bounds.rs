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
#[kani::proof]
#[kani::unwind(5)]
fn a_bounded_response_never_exceeds_its_deadline() {
    let c: u64 = kani::any();
    let t: u64 = kani::any();
    let d: u64 = kani::any();
    let b: u64 = kani::any();
    let j: u64 = kani::any();
    kani::assume(t > 0 && t < 1_000_000);
    kani::assume(c <= d && d < 1_000_000);
    kani::assume(b < 1_000_000 && j < 1_000_000);

    let mut s = TaskSet::new();
    if s.push(task(c, t, d, b, j)).is_ok() {
        if let Response::Bounded(r) = s.response_of(0) {
            assert!(r <= d);
        }
    }
}

/// Every `AnalysisFailure` variant fails every deadline comparison. A caller that
/// forgets to match on the reason still gets the safe answer.
#[kani::proof]
fn every_unbounded_variant_fails_every_deadline() {
    let d: u64 = kani::any();
    let v: u64 = kani::any();
    assert!(!Response::Refused(AnalysisFailure::NonConvergent).meets(d));
    assert!(!Response::Refused(AnalysisFailure::Overflow).meets(d));
    assert!(!Response::Refused(AnalysisFailure::NoSuchTask).meets(d));
    assert!(!Response::Refused(AnalysisFailure::ExceedsDeadline(v)).meets(d));
}

/// `response_of` terminates and never panics. Overflow is refused rather than
/// wrapped, so no arithmetic in either loop can abort.
///
/// The bound covers the busy-period solve and the per-job solve together. It
/// does not cover every admissible set — see the note at the head of this file
/// — and the input scope is narrowed here so that what it does cover is
/// stated rather than implied.
#[kani::proof]
#[kani::unwind(6)]
fn the_recurrence_terminates_without_panicking() {
    let c0: u64 = kani::any();
    let t0: u64 = kani::any();
    let c1: u64 = kani::any();
    let t1: u64 = kani::any();
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
#[kani::unwind(6)]
fn a_bounded_answer_is_never_below_its_own_work() {
    let c: u64 = kani::any();
    let t: u64 = kani::any();
    let b: u64 = kani::any();
    let j: u64 = kani::any();
    kani::assume(t > 0 && t < 1_000);
    kani::assume(c <= t && b < 1_000 && j < 1_000);

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
#[kani::unwind(3)]
fn a_lone_task_pays_only_for_itself() {
    let c: u64 = kani::any();
    let t: u64 = kani::any();
    let b: u64 = kani::any();
    let j: u64 = kani::any();
    kani::assume(t > 0 && t < 100_000);
    kani::assume(c < 100_000 && b < 100_000 && j < 100_000);

    let mut s = TaskSet::new();
    if s.push(task(c, t, u64::MAX, b, j)).is_ok() {
        if let Response::Bounded(r) = s.response_of(0) {
            assert!(r == c + b + j);
        }
    }
}

/// An index past the end is named rather than guessed, and never reported as
/// a bound.
#[kani::proof]
fn an_index_past_the_end_is_named() {
    let i: usize = kani::any();
    kani::assume(i > 0);
    let s = TaskSet::new();
    assert!(s.response_of(i) == Response::Refused(AnalysisFailure::NoSuchTask));
}
