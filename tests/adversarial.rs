// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
// DY Research — https://dyresearch.github.io

//! Every public entry point, at the edges of the integer domain.
//!
//! The rule this crate states about itself is that it refuses rather than
//! rounds. A refusal is a `Response::Refused`, a `None` or an `Err` — it is not
//! a panic, and it is certainly not a wrapped number. Both of those had
//! happened: `utilisation_through(usize::MAX)` panicked in debug and returned
//! `Some(0)` in release for a busy set, which is the same defect wearing two
//! faces, and only the loud face would ever have been noticed.
//!
//! So this file calls everything public with `0`, `1`, `u64::MAX - 1` and
//! `u64::MAX`, and with the index boundaries around `MAX_TASKS`. Running it
//! under the debug profile is what catches a wrap: debug builds panic on
//! overflow, so a test that merely "does not panic" in release proves nothing
//! about arithmetic. The gate runs both.

use dy_wcet::{AnalysisFailure, Rejected, Response, Task, TaskSet, MAX_TASKS};

const EDGES: [u64; 6] = [0, 1, 2, 1_000, u64::MAX - 1, u64::MAX];

fn admitted(t: Task) -> Option<TaskSet> {
    let mut s = TaskSet::new();
    s.push(t).ok()?;
    Some(s)
}

/// No combination of edge values makes construction do anything but accept or
/// reject, and the rejection always names a reason.
#[test]
fn construction_accepts_or_names_its_refusal() {
    for &c in &EDGES {
        for &t in &EDGES {
            for &d in &EDGES {
                for &j in &[0u64, 1, u64::MAX] {
                    for &b in &[0u64, 1, u64::MAX] {
                        let task = Task::new(c, t).deadline(d).jitter(j).blocking(b);
                        let mut s = TaskSet::new();
                        match s.push(task) {
                            Ok(()) => assert_eq!(s.len(), 1),
                            Err(Rejected::ZeroPeriod) => assert_eq!(t, 0),
                            Err(Rejected::ExecutionExceedsDeadline) => assert!(c > d),
                            Err(Rejected::Full) => panic!("a one-task set cannot be full"),
                        }
                    }
                }
            }
        }
    }
}

/// `utilisation_ppm` on a single task: a number or `None`, never a wrap.
#[test]
fn task_utilisation_refuses_rather_than_wrapping() {
    for &c in &EDGES {
        for &t in &EDGES {
            let u = Task::new(c, t).utilisation_ppm();
            match u {
                None => assert!(t == 0 || c.checked_mul(1_000_000).is_none()),
                Some(v) => {
                    // The only honest check that does not repeat the formula:
                    // utilisation is zero exactly when the task does no work.
                    assert_eq!(v == 0, c == 0 || c * 1_000_000 / t == 0);
                }
            }
        }
    }
}

/// Every index, including the ones that used to overflow.
#[test]
fn every_index_is_answerable() {
    let mut s = TaskSet::new();
    s.push(Task::new(1, 10)).unwrap();
    s.push(Task::new(2, 100)).unwrap();

    let whole = s.utilisation_ppm().unwrap();
    for &i in &[
        0usize,
        1,
        2,
        MAX_TASKS - 1,
        MAX_TASKS,
        usize::MAX - 1,
        usize::MAX,
    ] {
        // Deeper than the set is still the whole set, never zero and never a panic.
        let u = s.utilisation_through(i).expect("no overflow at any index");
        assert!(u <= whole);
        if i >= 1 {
            assert_eq!(u, whole);
        }

        match s.response_of(i) {
            Response::Bounded(_) => assert!(i < 2),
            Response::Refused(AnalysisFailure::NoSuchTask) => assert!(i >= 2),
            other => panic!("index {i} produced {other:?}"),
        }

        assert_eq!(s.slack_of(i).is_some(), i < 2);
        assert_eq!(s.get(i).is_some(), i < 2);
    }
}

/// The analysis at the top of the integer range refuses; it does not wrap.
#[test]
fn extreme_parameters_refuse_and_say_why() {
    // Execution so large that utilisation cannot be represented at all.
    let huge = Task::new(u64::MAX, u64::MAX).deadline(u64::MAX);
    let s = admitted(huge).expect("admissible: wcet equals deadline");
    assert_eq!(
        s.response_of(0),
        Response::Refused(AnalysisFailure::Overflow),
        "an unrepresentable utilisation must be refused, not truncated"
    );

    // Jitter at the top of the range, with real work underneath it.
    let mut s = TaskSet::new();
    s.push(Task::new(1, 10)).unwrap();
    s.push(Task::new(1, 10).deadline(u64::MAX).jitter(u64::MAX - 1))
        .unwrap();
    // No assertion on which it is — the model admits either here. What is
    // being checked is that it answers at all rather than panicking, which is
    // the whole point of running this file under the debug profile.
    let _ = s.response_of(1);

    // Blocking at the top of the range.
    let mut s = TaskSet::new();
    s.push(Task::new(1, 1_000)).unwrap();
    s.push(
        Task::new(1, 1_000)
            .deadline(u64::MAX)
            .blocking(u64::MAX - 1),
    )
    .unwrap();
    assert!(
        matches!(s.response_of(1), Response::Refused(_)),
        "an unrepresentable busy period must be refused"
    );
}

/// A full set, then one more.
#[test]
fn the_set_fills_and_then_refuses() {
    let mut s = TaskSet::new();
    for k in 0..MAX_TASKS {
        s.push(Task::new(1, 1_000_000 + k as u64)).unwrap();
    }
    assert_eq!(s.len(), MAX_TASKS);
    assert_eq!(s.push(Task::new(1, 10)), Err(Rejected::Full));
    assert_eq!(s.len(), MAX_TASKS, "a refused push must not change the set");

    // And the whole set still answers at every level.
    for i in 0..MAX_TASKS {
        assert!(matches!(s.response_of(i), Response::Bounded(_)));
    }
    assert_eq!(s.utilisation_through(usize::MAX), s.utilisation_ppm());
}

/// Sensitivity and priority assignment on degenerate sets.
#[test]
fn the_derived_answers_hold_at_the_edges() {
    // A task that exactly fills its deadline has no headroom, and says zero
    // rather than refusing: zero is an answer.
    let mut s = TaskSet::new();
    s.push(Task::new(10, 10)).unwrap();
    assert_eq!(s.max_wcet_increase(0), Some(0));
    assert_eq!(s.slack_of(0), Some(0));

    // An empty set has nothing to say, and says so.
    let empty = TaskSet::new();
    assert!(empty.is_empty());
    assert_eq!(empty.max_wcet_increase(0), None);
    assert_eq!(empty.slack_of(0), None);
    assert_eq!(empty.first_failure(), None);
    assert!(
        empty.is_schedulable(),
        "a set with no deadlines misses none"
    );
    assert_eq!(empty.utilisation_through(usize::MAX), Some(0));

    // An unschedulable set refuses to report headroom rather than inventing it.
    let mut over = TaskSet::new();
    over.push(Task::new(600, 1_000)).unwrap();
    over.push(Task::new(600, 1_000)).unwrap();
    assert!(!over.is_schedulable());
    assert_eq!(over.max_wcet_increase(0), None);
    assert_eq!(over.first_failure(), Some(1));
}
