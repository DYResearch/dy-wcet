// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! Kani harnesses for `response_of`.
//!
//! Tests sample the input space. These bound it. Each harness states a
//! property that must hold for every task set the model checker can build
//! within its unwind limit, and a counterexample is the falsifier.
//!
//!     cargo kani --harness a_bounded_response_never_exceeds_its_deadline
//!
//! The unwind limits are small on purpose: the recurrence settles in two
//! iterations on every set measured, so a bound of four covers twice the
//! observed worst case while keeping the proof tractable.

use dy_wcet::{Response, Task, TaskSet, Unbounded};

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

/// Every `Unbounded` variant fails every deadline comparison. A caller that
/// forgets to match on the reason still gets the safe answer.
#[kani::proof]
fn every_unbounded_variant_fails_every_deadline() {
    let d: u64 = kani::any();
    let v: u64 = kani::any();
    assert!(!Response::Unbounded(Unbounded::NonConvergent).meets(d));
    assert!(!Response::Unbounded(Unbounded::Overflow).meets(d));
    assert!(!Response::Unbounded(Unbounded::NoSuchTask).meets(d));
    assert!(!Response::Unbounded(Unbounded::ExceedsDeadline(v)).meets(d));
}

/// `response_of` terminates and never panics. Overflow is refused rather than
/// wrapped, so no arithmetic in the loop can abort.
#[kani::proof]
#[kani::unwind(5)]
fn the_recurrence_terminates_without_panicking() {
    let c0: u64 = kani::any();
    let t0: u64 = kani::any();
    let c1: u64 = kani::any();
    let t1: u64 = kani::any();
    kani::assume(t0 > 0 && t1 > 0);
    kani::assume(c0 <= t0 && c1 <= t1);

    let mut s = TaskSet::new();
    if s.push(task(c0, t0, t0, 0, 0)).is_ok() && s.push(task(c1, t1, t1, 0, 0)).is_ok() {
        let _ = s.response_of(0);
        let _ = s.response_of(1);
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
    assert!(s.response_of(i) == Response::Unbounded(Unbounded::NoSuchTask));
}
