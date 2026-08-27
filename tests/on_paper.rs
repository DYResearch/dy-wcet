// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! Cases whose expected value is derived above them, iteration by iteration.
//!
//! A test asserting a number without showing where the number came from pins
//! the implementation rather than the analysis, and if the implementation is
//! wrong such a test protects the bug. Every expectation here can be settled
//! with a pencil.

use dy_wcet::{Rejected, Response, Task, TaskSet, Unbounded, MAX_TASKS};

/// ```text
/// C = 2, B = 0, higher: (1, 4) and (2, 6)
/// R = 2   ⌈2/4⌉·1 + ⌈2/6⌉·2 = 1 + 2  →  5
/// R = 5   ⌈5/4⌉·1 + ⌈5/6⌉·2 = 2 + 2  →  6
/// R = 6   ⌈6/4⌉·1 + ⌈6/6⌉·2 = 2 + 2  →  6   ← fixed point
/// ```
/// Six, not seven. A task released at time 6 does not interfere with a
/// response that completes at 6, and the arithmetic says so even when memory
/// does not.
#[test]
fn three_tasks_settling_at_six() {
    let mut s = TaskSet::new();
    s.push(Task::new(1, 4)).unwrap();
    s.push(Task::new(2, 6)).unwrap();
    s.push(Task::new(2, 20)).unwrap();
    assert_eq!(s.response_of(2), Response::Bounded(6));
}

/// ```text
/// C = 300, D = 350, T = 2000, higher: (100, 400)
/// R = 300   ⌈300/400⌉·100 = 100  →  400
/// R = 400   ⌈400/400⌉·100 = 100  →  400   ← fixed point, and 400 > 350
/// ```
/// Utilisation is 100/400 + 300/2000 = 0.4, and the task still misses. Any
/// test that reads utilisation alone calls this set safe.
#[test]
fn a_deadline_shorter_than_the_period_can_fail_at_low_utilisation() {
    let mut s = TaskSet::new();
    s.push(Task::new(100, 400)).unwrap();
    s.push(Task::new(300, 2000).deadline(350)).unwrap();
    assert_eq!(
        s.response_of(1),
        Response::Unbounded(Unbounded::ExceedsDeadline(400))
    );
    assert_eq!(s.utilisation_ppm(), Some(400_000));
}

/// ```text
/// (2, 4) and (2, 4): U = 0.5 + 0.5 = 1.0 exactly
/// R = 2   ⌈2/4⌉·2 = 2  →  4
/// R = 4   ⌈4/4⌉·2 = 2  →  4   ← fixed point, and 4 = D
/// ```
/// Full utilisation converges and every deadline is met. An implementation
/// testing `U < 1` with a strict inequality reports failure here.
#[test]
fn full_utilisation_is_schedulable_when_it_lands_exactly() {
    let mut s = TaskSet::new();
    s.push(Task::new(2, 4)).unwrap();
    s.push(Task::new(2, 4)).unwrap();
    assert_eq!(s.utilisation_ppm(), Some(1_000_000));
    assert_eq!(s.response_of(1), Response::Bounded(4));
}

/// ```text
/// C = 10, B = 500, D = 400, no higher priority tasks
/// R = 10 + 500 = 510   no interference  →  510   ← fixed point, 510 > 400
/// ```
/// Blocking is not interference and it is not scaled by anything. It is added
/// once, and on its own it can sink a task with no preemption at all.
#[test]
fn blocking_alone_can_exceed_a_deadline() {
    let mut s = TaskSet::new();
    s.push(Task::new(10, 1000).deadline(400).blocking(500))
        .unwrap();
    assert_eq!(
        s.response_of(0),
        Response::Unbounded(Unbounded::ExceedsDeadline(510))
    );
}

/// ```text
/// higher: (0, 5) — zero execution time
/// R = 7   ⌈7/5⌉·0 = 0  →  7   ← fixed point on the first iteration
/// ```
/// A task with no execution contributes no interference, so the window never
/// grows and the loop exits immediately. The iteration cap is not what stops
/// this, which is worth pinning: the comment in `response_of` once claimed it
/// was, and the claim was wrong.
#[test]
fn a_zero_execution_task_terminates_rather_than_spinning() {
    let mut s = TaskSet::new();
    s.push(Task::new(0, 5)).unwrap();
    s.push(Task::new(7, 100)).unwrap();
    assert_eq!(s.response_of(1), Response::Bounded(7));
}

/// ```text
/// higher (100, 400), lower C = 200, T = 1000
/// R = 200   ⌈200/400⌉·100 = 100  →  300
/// R = 300   ⌈300/400⌉·100 = 100  →  300   ← fixed point
/// ```
/// At D = 300 the task meets its deadline; at D = 299 it does not, and the
/// same single microsecond changes both the sum and nothing else.
#[test]
fn the_deadline_boundary_from_both_sides() {
    let mut fits = TaskSet::new();
    fits.push(Task::new(100, 400)).unwrap();
    fits.push(Task::new(200, 1000).deadline(300)).unwrap();
    assert_eq!(fits.response_of(1), Response::Bounded(300));

    let mut misses = TaskSet::new();
    misses.push(Task::new(100, 400)).unwrap();
    misses.push(Task::new(200, 1000).deadline(299)).unwrap();
    assert_eq!(
        misses.response_of(1),
        Response::Unbounded(Unbounded::ExceedsDeadline(300))
    );
}

/// ```text
/// C = 200, T = 1000, D = 1500 — a deadline beyond the period
/// R = 200   ⌈200/400⌉·100 = 100  →  300
/// R = 300   ⌈300/400⌉·100 = 100  →  300   ← fixed point
/// ```
/// An implementation that assumes `R ≤ T`, or that clamps the deadline to the
/// period, is right on most sets and wrong on this one.
#[test]
fn a_deadline_beyond_the_period_is_permitted() {
    let mut s = TaskSet::new();
    s.push(Task::new(100, 400)).unwrap();
    s.push(Task::new(200, 1000).deadline(1500)).unwrap();
    assert_eq!(s.response_of(1), Response::Bounded(300));
}

/// ```text
/// higher (100, 400) with J = 300, lower C = 200, T = 2000, D = 2000
/// w = 200   ⌈(200+300)/400⌉·100 = 2·100 = 200  →  400
/// w = 400   ⌈(400+300)/400⌉·100 = 2·100 = 200  →  400   ← fixed point
/// ```
/// Without the jitter term the same set settles at 300. Three hundred
/// microseconds of release jitter on the high-priority task costs the low one
/// a whole extra preemption, and an analysis with no `J` reports the flattering
/// number.
#[test]
fn jitter_upstream_buys_a_whole_extra_preemption() {
    let mut with_jitter = TaskSet::new();
    with_jitter.push(Task::new(100, 400).jitter(300)).unwrap();
    with_jitter.push(Task::new(200, 2000)).unwrap();
    assert_eq!(with_jitter.response_of(1), Response::Bounded(400));

    let mut without = TaskSet::new();
    without.push(Task::new(100, 400)).unwrap();
    without.push(Task::new(200, 2000)).unwrap();
    assert_eq!(without.response_of(1), Response::Bounded(300));
}

/// ```text
/// C = 200, T = 1000, J = 40, higher (100, 400) with J = 0
/// w = 200   ⌈200/400⌉·100 = 100  →  300
/// w = 300   ⌈300/400⌉·100 = 100  →  300   ← fixed point
/// R = w + J = 300 + 40 = 340
/// ```
/// A task's own jitter does not widen the window it sees. It is added once, at
/// the end, to the answer.
#[test]
fn a_tasks_own_jitter_is_added_once_at_the_end() {
    let mut s = TaskSet::new();
    s.push(Task::new(100, 400)).unwrap();
    s.push(Task::new(200, 1000).jitter(40)).unwrap();
    assert_eq!(s.response_of(1), Response::Bounded(340));
}

/// ```text
/// (300, 400) and (300, 400): U = 0.75 + 0.75 = 1.5 > 1
/// R = 300   ⌈300/400⌉·300 = 300  →  600
/// R = 600   ⌈600/400⌉·300 = 600  →  900   climbing, and it never stops
/// ```
/// Reported before the loop runs, from utilisation, rather than after ten
/// thousand fruitless iterations.
#[test]
fn an_over_utilised_set_is_refused_by_arithmetic_not_by_exhaustion() {
    let mut s = TaskSet::new();
    s.push(Task::new(300, 400)).unwrap();
    s.push(Task::new(300, 400)).unwrap();
    assert_eq!(s.utilisation_ppm(), Some(1_500_000));
    assert_eq!(
        s.response_of(1),
        Response::Unbounded(Unbounded::NonConvergent)
    );
}

/// With a deadline of `u64::MAX` the deadline exit can never fire and the
/// iteration cap is four orders of magnitude away, so the question is whether
/// `checked_mul` gets there first. It does, on the very first interference
/// term: `⌈R/1⌉ · (u64::MAX / 2)` overflows immediately.
///
/// ```text
/// R = large   ⌈R/1⌉ · C overflows  →  refused, not wrapped
/// ```
#[test]
fn overflow_beats_the_iteration_cap_to_the_answer() {
    let mut s = TaskSet::new();
    s.push(Task::new(u64::MAX / 2, 1).deadline(u64::MAX))
        .unwrap();
    s.push(Task::new(1, u64::MAX).deadline(u64::MAX)).unwrap();
    assert!(!s.response_of(1).is_bounded());
    assert_eq!(s.response_of(1).bound(), None);
}

/// No derivation: this fixes an interface, not a number.
#[test]
fn determinism_across_repeated_evaluation() {
    let mut s = TaskSet::new();
    s.push(Task::new(100, 400).jitter(11)).unwrap();
    s.push(Task::new(200, 1000).blocking(20).jitter(7)).unwrap();
    let expected = s.response_of(1);
    for _ in 0..10_000 {
        assert_eq!(s.response_of(1), expected);
    }
}

/// No derivation: this fixes an interface, not a number.
#[test]
fn the_task_limit_is_enforced_rather_than_overrun() {
    let mut s = TaskSet::new();
    for _ in 0..MAX_TASKS {
        s.push(Task::new(1, 10_000_000)).unwrap();
    }
    assert_eq!(s.push(Task::new(1, 10_000_000)), Err(Rejected::Full));
    assert_eq!(s.len(), MAX_TASKS);
}

/// No derivation: this fixes an interface, not a number.
#[test]
fn rejections_and_responses_describe_themselves() {
    assert!(Rejected::ZeroPeriod.to_string().contains("period"));
    assert!(Response::Bounded(42).to_string().contains("42"));
    assert!(Response::Unbounded(Unbounded::NonConvergent)
        .to_string()
        .contains("utilisation"));
    assert!(Response::Unbounded(Unbounded::ExceedsDeadline(400))
        .to_string()
        .contains("400"));
    assert!(Response::Unbounded(Unbounded::NoSuchTask)
        .to_string()
        .contains("no task"));
}

/// No derivation: this fixes an interface, not a number.
#[test]
fn a_named_task_carries_its_name() {
    let mut s = TaskSet::new();
    s.push(Task::new(100, 400).named("sensor_poll")).unwrap();
    assert_eq!(s.get(0).unwrap().name, "sensor_poll");
    assert_eq!(s.iter().count(), 1);
}
