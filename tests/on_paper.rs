// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
// Part of DY Research — https://github.com/DYResearch

//! Task sets whose answers can be checked on paper.
//!
//! The unit tests in `src/lib.rs` check that the implementation does what its
//! author intended. These check something different and harder: that what its
//! author intended is what the analysis says.
//!
//! Every expected value here is derived in the comment above it, iteration by
//! iteration, so a reader who distrusts the code can settle the question with
//! a pencil instead of running anything.
//!
//! That distinction matters more than it sounds. While writing this file I
//! stated the answer to `three_tasks_settling_at_six` as 7 from memory, and
//! the arithmetic says 6. The recurrence is short enough that being confident
//! about it is easy and being right about it is not — which is the entire
//! reason this crate exists.

use dy_wcet::{Response, Task, TaskSet};

fn set(rows: &[(u64, u64, u64, u64)]) -> TaskSet {
    let mut s = TaskSet::new();
    for &(wcet_us, period_us, deadline_us, blocking_us) in rows {
        s.push(Task {
            wcet_us,
            period_us,
            deadline_us,
            blocking_us,
        })
        .expect("the fixtures in this file are all admissible");
    }
    s
}

/// Three tasks, priority by period, and the fixed point is 6.
///
/// ```text
/// T1  C=1  T=4      T2  C=2  T=6      T3  C=2  T=10
///
/// R(T3):
///   R = 2   ⌈2/4⌉·1 + ⌈2/6⌉·2 = 1 + 2  →  5
///   R = 5   ⌈5/4⌉·1 + ⌈5/6⌉·2 = 2 + 2  →  6
///   R = 6   ⌈6/4⌉·1 + ⌈6/6⌉·2 = 2 + 2  →  6   ← fixed point
/// ```
///
/// The step worth watching is the last one. At R = 6, T2 has had exactly one
/// activation — `⌈6/6⌉ = 1`, not 2 — because a task released at time 6 does
/// not interfere with a response that completes at 6. An implementation that
/// rounds up here instead of using a true ceiling reports 8, and 8 is a number
/// that will pass most tests somebody writes for it.
#[test]
fn three_tasks_settling_at_six() {
    let s = set(&[(1, 4, 4, 0), (2, 6, 6, 0), (2, 10, 10, 0)]);
    assert_eq!(s.response_of(0), Response::Bounded(1));
    assert_eq!(s.response_of(1), Response::Bounded(3));
    assert_eq!(s.response_of(2), Response::Bounded(6));
    assert!(s.is_schedulable());
}

/// A deadline shorter than the period, which is where rate-monotonic
/// reasoning stops applying.
///
/// ```text
/// T1  C=2  T=5  D=5        T2  C=3  T=8  D=4
///
/// R(T2):
///   R = 3   ⌈3/5⌉·2 = 2  →  5
///   5 > D = 4  →  unschedulable
/// ```
///
/// Utilisation is 0.4 + 0.375 = 0.775, comfortably below one, and a
/// utilisation-based test would call this schedulable. It is not. Response
/// time is the only thing that answers the question, and this is the smallest
/// set that shows why.
#[test]
fn a_deadline_shorter_than_the_period_can_fail_at_low_utilisation() {
    let s = set(&[(2, 5, 5, 0), (3, 8, 4, 0)]);
    assert_eq!(s.response_of(1), Response::Unschedulable);
    assert!(!s.is_schedulable());
    // The point of the case: utilisation says nothing useful here.
    assert_eq!(s.utilisation_ppm(), Some(775_000));
}

/// Utilisation of exactly one, which is schedulable and often assumed not to
/// be.
///
/// ```text
/// T1  C=1  T=2        T2  C=1  T=2
///
/// R(T2):
///   R = 1   ⌈1/2⌉·1 = 1  →  2
///   R = 2   ⌈2/2⌉·1 = 1  →  2   ← fixed point, and exactly the deadline
/// ```
///
/// The processor is busy every microsecond and no deadline is missed. An
/// implementation using a strict inequality against the deadline reports
/// failure here.
#[test]
fn full_utilisation_is_schedulable_when_it_lands_exactly() {
    let s = set(&[(1, 2, 2, 0), (1, 2, 2, 0)]);
    assert_eq!(s.response_of(1), Response::Bounded(2));
    assert!(s.is_schedulable());
    assert_eq!(s.utilisation_ppm(), Some(1_000_000));
}

/// Blocking longer than the deadline it applies to.
///
/// ```text
/// T1  C=1  T=10  D=10       T2  C=2  T=20  D=5  B=10
///
/// R(T2):
///   R⁰ = C + B = 2 + 10 = 12
///   12 > D = 5  →  unschedulable, before interference is even considered
/// ```
///
/// A task that can be held up for 10 µs cannot meet a 5 µs deadline whatever
/// else is true. The first iteration establishes it.
#[test]
fn blocking_alone_can_exceed_a_deadline() {
    let s = set(&[(1, 10, 10, 0), (2, 20, 5, 10)]);
    assert_eq!(s.response_of(1), Response::Unschedulable);
}

/// A zero-execution task at higher priority, which is admissible and must not
/// spin.
///
/// ```text
/// T1  C=0  T=10  D=10       T2  C=5  T=20  D=20
///
/// R(T2):
///   R = 5   ⌈5/10⌉·0 = 0   →  5   ← fixed point on the first iteration
/// ```
///
/// `push` accepts a zero-execution task: zero work in zero time is degenerate
/// but coherent, and refusing it would reject a valid model of a hardware
/// event that costs nothing. The recurrence must still terminate, and it does
/// — the interference term is zero, so the first value is already the fixed
/// point. An implementation that iterates until the value *changes* rather
/// than until it stops changing spins here forever.
#[test]
fn a_zero_execution_task_terminates_rather_than_spinning() {
    let s = set(&[(0, 10, 10, 0), (5, 20, 20, 0)]);
    assert_eq!(s.response_of(1), Response::Bounded(5));
}

/// The boundary, from both sides, one microsecond apart.
///
/// ```text
/// T1  C=100  T=400          T2  C=?  T=1000  D=400
///
/// C = 299:  R = 299   ⌈299/400⌉·100 = 100  →  399  ← fixed point, pass
/// C = 300:  R = 300   ⌈300/400⌉·100 = 100  →  400  ← fixed point, exactly
/// C = 301:  R = 301   ⌈301/400⌉·100 = 100  →  401
///           R = 401   ⌈401/400⌉·100 = 200  →  501  >  D = 400, fail
/// ```
///
/// At C = 300 the response lands on the deadline and the interference term is
/// still one activation, because `⌈400/400⌉ = 1`. One microsecond more and
/// both change at once: the sum passes the deadline and a second activation
/// appears. Off-by-one errors in this analysis hide precisely here.
#[test]
fn the_deadline_boundary_from_both_sides() {
    let below = set(&[(100, 400, 400, 0), (299, 1000, 400, 0)]);
    assert_eq!(below.response_of(1), Response::Bounded(399));

    let exact = set(&[(100, 400, 400, 0), (300, 1000, 400, 0)]);
    assert_eq!(exact.response_of(1), Response::Bounded(400));
    assert!(exact.is_schedulable());

    let above = set(&[(100, 400, 400, 0), (301, 1000, 400, 0)]);
    assert_eq!(above.response_of(1), Response::Unschedulable);
    assert!(!above.is_schedulable());
}

/// The same set, computed a thousand times, byte-identical every time.
///
/// ```text
/// T1  C=37  T=211      T2  C=53  T=499      T3  C=101  T=1013
///
/// R(T3) settles at 191, and settles there on every run:
///   R = 101   ⌈101/211⌉·37 + ⌈101/499⌉·53 = 37 + 53   →  191
///   R = 191   ⌈191/211⌉·37 + ⌈191/499⌉·53 = 37 + 53   →  191   ← fixed point
/// ```
///
/// This is the property the integer arithmetic exists for, and the one no
/// floating-point implementation can offer: two runs on two machines with two
/// compilers produce the same bits, or one of them is wrong. The periods are
/// prime so no ceiling divides evenly and every iteration exercises the
/// remainder.
#[test]
fn determinism_across_repeated_evaluation() {
    let s = set(&[(37, 211, 211, 0), (53, 499, 499, 0), (101, 1013, 1013, 0)]);
    let first = s.response_of(2);
    assert!(matches!(first, Response::Bounded(_)));
    for _ in 0..1000 {
        assert_eq!(s.response_of(2), first);
    }
}

/// A response time longer than the task's own period.
///
/// ```text
/// T1  C=3  T=10  D=10       T2  C=4  T=12  D=30
///
/// R(T2):
///   R = 4    ⌈4/10⌉·3 = 3   →  7
///   R = 7    ⌈7/10⌉·3 = 3   →  7   ← fixed point
/// ```
///
/// Here it settles below the period, but the deadline of 30 permits a response
/// beyond T = 12, and the analysis has to allow that. A version that assumed
/// `R ≤ T` and stopped early would be correct on most sets and silently wrong
/// on the ones where it matters — deadlines longer than periods are common in
/// exactly the low-rate, high-cost tasks that cause trouble.
#[test]
fn a_deadline_beyond_the_period_is_permitted() {
    let s = set(&[(3, 10, 10, 0), (4, 12, 30, 0)]);
    assert_eq!(s.response_of(1), Response::Bounded(7));
    assert!(s.is_schedulable());
}

/// Sixteen tasks, the maximum, and the seventeenth is refused.
///
/// ```text
/// T1…T16   C=1, T = 1000·i for i in 1…16
///
/// R(T16):
///   R = 1    ⌈1/T⌉ = 1 for all fifteen higher tasks  →  1 + 15 = 16
///   R = 16   ⌈16/T⌉ = 1 for all fifteen              →  16   ← fixed point
/// ```
///
/// Sixteen tasks, one microsecond each, against a 16 000 µs deadline.
///
/// The cap is not a theoretical limit. It is the point past which a
/// fixed-priority set on one core stops being checkable by hand, and an
/// analysis nobody can check by hand is an analysis nobody checks.
#[test]
fn the_task_limit_is_enforced_rather_than_overrun() {
    let mut s = TaskSet::new();
    for i in 1..=16u64 {
        s.push(Task {
            wcet_us: 1,
            period_us: 1000 * i,
            deadline_us: 1000 * i,
            blocking_us: 0,
        })
        .expect("sixteen fit");
    }
    assert_eq!(s.len(), 16);
    assert!(s
        .push(Task {
            wcet_us: 1,
            period_us: 99_000,
            deadline_us: 99_000,
            blocking_us: 0
        })
        .is_err());
    assert!(s.is_schedulable());
}
