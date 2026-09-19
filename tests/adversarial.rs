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

use dy_wcet::{AnalysisFailure, PriorityAssignment, Rejected, Response, Task, TaskSet, MAX_TASKS};

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
    assert_eq!(s.max_provable_wcet_increase(0), Some(0));
    assert_eq!(s.slack_of(0), Some(0));

    // An empty set has nothing to say, and says so.
    let empty = TaskSet::new();
    assert!(empty.is_empty());
    assert_eq!(empty.max_provable_wcet_increase(0), None);
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
    assert_eq!(over.max_provable_wcet_increase(0), None);
    assert_eq!(over.first_failure(), Some(1));
}

// ── 4.0.0: the findings an external audit raised, and what each turned out
//    to be. Kept together because the pairing is the point: two were real and
//    the loudest one was not.

/// The audit's headline counterexample, and the reason it is not a bug.
///
/// `C = T = 10` with `J = 1` was offered as "obviously feasible" and therefore
/// proof that the analysis was broken. Work it through: releases compress to
/// 0, 9, 19, 29 … so from the second job onward the response is 11 against a
/// deadline of 10. The set misses. Refusing is right.
///
/// What the refusal must not say is `IterationLimit`, which means *this
/// implementation stopped counting*. At U = 1 with a disturbance,
/// f(L) ≥ L + B + Σ Jⱼ·Cⱼ/Tⱼ exceeds L for every L, so no fixed point exists
/// and the honest word is `NonConvergent`.
#[test]
fn saturated_with_jitter_has_no_fixed_point_and_says_so() {
    let mut s = TaskSet::new();
    s.push(Task::new(10, 10).jitter(1)).unwrap();
    assert_eq!(
        s.response_of(0),
        Response::Refused(AnalysisFailure::NonConvergent),
        "a saturated level with jitter has no fixed point, and the refusal \
         should name that rather than report an exhausted loop"
    );
}

/// The same set without the jitter converges, which is why the condition is
/// saturation *and* a disturbance rather than saturation alone. A gate that
/// rejected every U = 1 set would lose this answer.
#[test]
fn saturated_without_disturbance_still_converges() {
    let mut s = TaskSet::new();
    s.push(Task::new(10, 10)).unwrap();
    assert_eq!(s.response_of(0), Response::Bounded(10));
}

/// Blocking is a disturbance too, by the same inequality.
#[test]
fn saturated_with_blocking_has_no_fixed_point() {
    let mut s = TaskSet::new();
    s.push(Task::new(10, 10).blocking(3)).unwrap();
    assert_eq!(
        s.response_of(0),
        Response::Refused(AnalysisFailure::NonConvergent)
    );
}

/// The real defect the audit found, reduced to the set that exposes it.
///
/// `passes_utilisation_bound` summed per-task utilisations that were each
/// already floored, then compared the total against the Liu and Layland
/// bound. C=2/T=7 with C=108/T=199 has a true utilisation of 828427.85 ppm
/// against a two-task bound of 828427 — above it — and the floored sum
/// reported exactly 828427. The function answered *schedulable without
/// further analysis* for a set that does not pass.
#[test]
fn the_liu_layland_precheck_is_not_decided_by_rounding() {
    let mut s = TaskSet::new();
    s.push(Task::new(2, 7)).unwrap();
    s.push(Task::new(108, 199)).unwrap();
    assert!(
        !s.passes_utilisation_bound(),
        "true utilisation is 828427.85 ppm against a bound of 828427; a \
         sufficient condition must not be granted by a floor"
    );

    // A second set from the same search, to show the first was not a one-off.
    let mut t = TaskSet::new();
    t.push(Task::new(8, 13)).unwrap();
    t.push(Task::new(49, 230)).unwrap();
    assert!(!t.passes_utilisation_bound());

    // And a set comfortably under the bound still passes, so the fix did not
    // simply make the pre-check useless.
    let mut u = TaskSet::new();
    u.push(Task::new(1, 4)).unwrap();
    u.push(Task::new(1, 4)).unwrap();
    assert!(u.passes_utilisation_bound());
}

/// A priority search that ran out of candidates because the analysis declined
/// them has not shown that no ordering exists, and must not say so.
///
/// Getting this test right took two attempts, and the first attempt is the
/// more instructive one. It used two saturated tasks carrying jitter, on the
/// reasoning that a refusal is a refusal. It is not: that set refuses with
/// `NonConvergent`, which *proves* no bound exists, and a level whose every
/// candidate is proven unbounded genuinely has no ordering. `NoOrdering` was
/// the right answer there and the test asserting otherwise was wrong.
///
/// A truly undecided level needs a refusal that establishes nothing. Both
/// tasks below carry a blocking term large enough that the level-i busy
/// period reaches 12 600 µs, which is 6 300 jobs of the two-microsecond task
/// and 4 200 of the three — well past `BUSY_PERIOD_CAP`. Utilisation is 0.833,
/// so nothing here is overloaded; the analysis simply declines to walk that
/// many jobs, and declining is not a finding.
#[test]
fn a_refused_candidate_leaves_the_search_inconclusive() {
    let mut s = TaskSet::new();
    s.push(Task::new(1, 2).blocking(2100)).unwrap();
    s.push(Task::new(1, 3).blocking(2100)).unwrap();

    match s.optimal_priority_order() {
        PriorityAssignment::Inconclusive(AnalysisFailure::BusyPeriodLimit) => {}
        other => panic!(
            "expected Inconclusive(BusyPeriodLimit) when every candidate was \
             capped rather than analysed, got {other:?}"
        ),
    }
    assert!(
        !s.optimal_priority_order().is_proven_impossible(),
        "a cap is not a proof that no ordering works"
    );
}

/// The other half of the same distinction. An overloaded set refuses with
/// `NonConvergent` at every level, and that is a proof rather than a limit:
/// no ordering of a set demanding one and a half processors works.
#[test]
fn an_overloaded_set_is_proven_impossible_not_inconclusive() {
    let mut s = TaskSet::new();
    s.push(Task::new(300, 400)).unwrap();
    s.push(Task::new(300, 400)).unwrap();
    assert_eq!(s.optimal_priority_order(), PriorityAssignment::NoOrdering);
    assert!(s.optimal_priority_order().is_proven_impossible());
}

/// Every refusal is classified, and the classification is the thing
/// `PriorityAssignment` reads. Listed one by one so that a new variant added
/// without a decision about it fails here.
#[test]
fn every_refusal_declares_whether_it_establishes_anything() {
    assert!(AnalysisFailure::ExceedsDeadline(9).is_determinate());
    assert!(AnalysisFailure::NonConvergent.is_determinate());
    assert!(!AnalysisFailure::IterationLimit.is_determinate());
    assert!(!AnalysisFailure::BusyPeriodLimit.is_determinate());
    assert!(!AnalysisFailure::Overflow.is_determinate());
    assert!(!AnalysisFailure::NoSuchTask.is_determinate());
}
