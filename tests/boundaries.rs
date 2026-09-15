// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
// DY Research — https://dyresearch.github.io

//
// The edges the other files do not reach.
//
// `on_paper.rs` proves the recurrence against hand derivations. `properties.rs`
// samples. `oracle.rs` and `differential.rs` check the 2.0.0 busy-period work
// against a second method and against the form it replaced. What none of them
// touch is the surface around the recurrence: the accessors that report a
// result, the admission rules, the refusals that are supposed to happen before
// any analysis starts, and the two constants that bound the whole thing.
//
// Every expected value below is derived in the comment above it. A reader who
// distrusts the implementation can settle each one with a pencil.

use dy_wcet::{AnalysisFailure, Rejected, Response, Task, TaskSet, BUSY_PERIOD_CAP, MAX_TASKS};

/// `first_failure` names the highest-priority task that misses, not the count
/// of tasks that do, and not the last one.
///
/// A(100, 400), B(100, 500), C(400, 1000) with D = 500.
///
///   R(A) = 100, inside 400.
///   R(B): w = 100 + ⌈w/400⌉·100. w = 100 → 200; w = 200 → 200. R = 200 ≤ 500.
///   R(C): w = 400 + ⌈w/400⌉·100 + ⌈w/500⌉·100.
///         w = 400 → 400 + 100 + 100 = 600
///         w = 600 → 400 + 200 + 200 = 800
///         w = 800 → 400 + 200 + 200 = 800   fixed point
///         R = 800, and the deadline is 500.
///
/// So index 2 is the first and only failure.
#[test]
fn first_failure_names_the_task_rather_than_counting_them() {
    let mut s = TaskSet::new();
    s.push(Task::new(100, 400).named("a")).unwrap();
    s.push(Task::new(100, 500).named("b")).unwrap();
    s.push(Task::new(400, 1000).deadline(500).named("c"))
        .unwrap();

    assert_eq!(s.response_of(0), Response::Bounded(100));
    assert_eq!(s.response_of(1), Response::Bounded(200));
    assert_eq!(
        s.response_of(2),
        Response::Refused(AnalysisFailure::ExceedsDeadline(800))
    );

    assert_eq!(s.first_failure(), Some(2));
    assert!(!s.is_schedulable());
}

/// A set where every task holds gives `None`, not `Some(0)`. Index 0 is a
/// truthful answer to a different question, and the two must not collide.
#[test]
fn first_failure_is_none_when_nothing_fails() {
    let mut s = TaskSet::new();
    s.push(Task::new(100, 400)).unwrap();
    s.push(Task::new(200, 1000)).unwrap();

    assert!(s.is_schedulable());
    assert_eq!(s.first_failure(), None);
}

/// The busy-period cap refuses rather than enumerating.
///
/// A(1, 1000), B(1, 1000) with B blocked for 2_000_000 and a deadline far
/// above it. Utilisation is 2000 ppm, so convergence is never in question:
/// this is not the over-utilised path.
///
///   L = B + ⌈L/1000⌉·1 + ⌈L/1000⌉·1 = 2_000_000 + 2·⌈L/1000⌉
///   L settles at 2_004_010, which holds ⌈2_004_010/1000⌉ = 2005 jobs of B.
///
/// 2005 is above BUSY_PERIOD_CAP, so the answer is a refusal. Halve the
/// blocking and the same set settles at 501_004 with 502 jobs, under the cap,
/// and produces a number.
#[test]
fn a_busy_period_past_the_cap_is_refused_and_not_enumerated() {
    let mut over = TaskSet::new();
    over.push(Task::new(1, 1000)).unwrap();
    over.push(Task::new(1, 1000).blocking(2_000_000).deadline(10_000_000))
        .unwrap();

    // Under one percent utilisation, and until 3.0.0 this asserted
    // `NonConvergent`, whose Display reads "utilisation exceeds one; no fixed
    // point exists". The assertion two lines above says it does not. The busy
    // period here closes; what the analysis declined to do is enumerate the
    // jobs inside it, which is what `BusyPeriodLimit` now says.
    assert!(over.utilisation_ppm().unwrap() < 10_000);
    assert_eq!(
        over.response_of(1),
        Response::Refused(AnalysisFailure::BusyPeriodLimit)
    );

    let mut under = TaskSet::new();
    under.push(Task::new(1, 1000)).unwrap();
    under
        .push(Task::new(1, 1000).blocking(500_000).deadline(10_000_000))
        .unwrap();

    assert_eq!(under.response_of(1), Response::Bounded(500_502));
    assert_eq!(BUSY_PERIOD_CAP, 1_024);
}

/// `bound` and `response_time` answer different questions, and the difference
/// is the whole reason `ExceedsDeadline` carries a number.
///
/// A(100, 400), B(300, 2000) with D = 350.
///
///   w = 300 + ⌈w/400⌉·100. w = 300 → 400; w = 400 → 400. R = 400 > 350.
///
/// `bound` is None: there is no bound at or below the deadline. `response_time`
/// is Some(400): the recurrence did converge and 400 is what it converged to.
/// A caller sizing a margin needs the second; a caller deciding whether to ship
/// needs the first.
#[test]
fn a_bound_and_a_response_time_are_not_the_same_question() {
    let mut s = TaskSet::new();
    s.push(Task::new(100, 400)).unwrap();
    s.push(Task::new(300, 2000).deadline(350)).unwrap();

    let r = s.response_of(1);
    assert_eq!(r, Response::Refused(AnalysisFailure::ExceedsDeadline(400)));
    assert_eq!(r.bound(), None);
    assert_eq!(r.response_time(), Some(400));
    assert_eq!(r.reason(), Some(AnalysisFailure::ExceedsDeadline(400)));
    assert!(!r.is_bounded());
    assert!(!r.meets(350));
    assert!(!r.meets(400));

    // A converged answer that fits reports the same number through both.
    let ok = s.response_of(0);
    assert_eq!(ok.bound(), Some(100));
    assert_eq!(ok.response_time(), Some(100));
    assert_eq!(ok.reason(), None);

    // The refusals carry no number at all, and must not invent one.
    let none = Response::Refused(AnalysisFailure::NonConvergent);
    assert_eq!(none.bound(), None);
    assert_eq!(none.response_time(), None);
}

/// A full set of sixteen is analysed, not refused. MAX_TASKS is a limit on
/// admission and nothing else, and an off-by-one there would either lose the
/// sixteenth task or admit a seventeenth into an array of sixteen.
///
/// Sixteen tasks of C = 1 at periods 1000, 2000 … 16000. Total utilisation is
/// 3375 ppm. The lowest-priority task is preempted at most once by each of the
/// fifteen above it inside its own response, so R(15) = 1 + 15 = 16.
#[test]
fn a_full_set_of_sixteen_is_analysed_rather_than_refused() {
    let mut s = TaskSet::new();
    for i in 1..=16u64 {
        s.push(Task::new(1, 1000 * i)).unwrap();
    }

    assert_eq!(s.len(), MAX_TASKS);
    assert!(!s.is_empty());
    assert_eq!(s.utilisation_ppm(), Some(3_375));
    assert_eq!(s.response_of(15), Response::Bounded(16));
    assert!(s.is_schedulable());

    assert_eq!(s.push(Task::new(1, 100_000)), Err(Rejected::Full));
    assert_eq!(s.len(), MAX_TASKS);
}

/// `utilisation_through` counts this priority level and above, and stops.
///
/// Three tasks of C = 100, 200, 300 all at T = 1000: 100_000, then 300_000,
/// then 600_000 ppm. The whole-set figure is the last of these, which is what
/// `utilisation_ppm` returns.
#[test]
fn utilisation_through_counts_this_level_and_above_and_stops() {
    let mut s = TaskSet::new();
    s.push(Task::new(100, 1000)).unwrap();
    s.push(Task::new(200, 1000)).unwrap();
    s.push(Task::new(300, 1000)).unwrap();

    assert_eq!(s.utilisation_through(0), Some(100_000));
    assert_eq!(s.utilisation_through(1), Some(300_000));
    assert_eq!(s.utilisation_through(2), Some(600_000));
    assert_eq!(s.utilisation_ppm(), s.utilisation_through(2));
}

/// Sensitivity on a set that already misses is `None`, not a number.
///
/// The set from the accessor test above misses at index 1. Asking how much
/// more execution time it could absorb has no honest answer, and zero is not
/// one: zero would say the set is exactly at its limit when it is past it.
#[test]
fn sensitivity_refuses_a_set_that_already_misses() {
    let mut miss = TaskSet::new();
    miss.push(Task::new(100, 400)).unwrap();
    miss.push(Task::new(300, 2000).deadline(350)).unwrap();

    assert!(!miss.is_schedulable());
    assert_eq!(miss.max_wcet_increase(0), None);
    assert_eq!(miss.max_wcet_increase(1), None);
    assert_eq!(miss.slack_of(1), None);

    // The same call on a set that holds returns the last value that still fits.
    // A(100, 400), B(200, 1000): at C = 700, w = 700 + ⌈w/400⌉·100 settles at
    // 1000, exactly the deadline. At C = 701 it settles at 1001 and misses.
    let mut ok = TaskSet::new();
    ok.push(Task::new(100, 400)).unwrap();
    ok.push(Task::new(200, 1000)).unwrap();
    assert_eq!(ok.max_wcet_increase(1), Some(500));
    assert_eq!(ok.slack_of(1), Some(700));
}

/// A higher-priority task's blocking term never reaches a lower-priority task.
///
/// Blocking under the priority-ceiling protocol is the one lower-priority
/// section that can hold a task up, and it is charged to the task that suffers
/// it. Summing it into the interference a lower task sees would charge it
/// twice, in the flattering direction for nobody and the alarming direction
/// for everybody.
///
/// A(100, 400) blocked for 200: R(A) = 300, inside 400.
/// B(200, 1000) below it: w = 200 + ⌈w/400⌉·100 settles at 300, unchanged from
/// the same set with A unblocked.
#[test]
fn a_higher_priority_blocking_term_never_reaches_a_lower_task() {
    let mut blocked = TaskSet::new();
    blocked.push(Task::new(100, 400).blocking(200)).unwrap();
    blocked.push(Task::new(200, 1000)).unwrap();

    let mut plain = TaskSet::new();
    plain.push(Task::new(100, 400)).unwrap();
    plain.push(Task::new(200, 1000)).unwrap();

    assert_eq!(blocked.response_of(0), Response::Bounded(300));
    assert_eq!(plain.response_of(0), Response::Bounded(100));

    assert_eq!(blocked.response_of(1), Response::Bounded(300));
    assert_eq!(blocked.response_of(1), plain.response_of(1));
}

/// `get` and `iter` report the set in the order it was pushed, which is the
/// order that decides priority. A container that reordered here would make
/// every index in every other answer mean something else.
#[test]
fn get_and_iter_report_the_priority_order_that_was_pushed() {
    let mut s = TaskSet::new();
    s.push(Task::new(1, 4).named("high")).unwrap();
    s.push(Task::new(2, 6).named("mid")).unwrap();
    s.push(Task::new(2, 20).named("low")).unwrap();

    assert_eq!(s.get(0).unwrap().name, "high");
    assert_eq!(s.get(2).unwrap().name, "low");
    assert!(s.get(3).is_none());

    let names: Vec<&str> = s.iter().map(|t| t.name).collect();
    assert_eq!(names, ["high", "mid", "low"]);
    assert_eq!(s.iter().count(), 3);
    assert_eq!(s.iter().count(), s.len());
}

/// The Liu and Layland table is a table, and past its end it falls back to the
/// limit the series converges on rather than reading off the end.
///
/// n·(2^(1/n) − 1) decreases in n toward ln 2, which is 0.693147… and 693_147
/// ppm floored. The table holds n = 0 through 16, matching MAX_TASKS, and a
/// larger n gets the limit. An empty set and a single task are both 1_000_000:
/// one task at full utilisation misses nothing.
#[test]
fn the_utilisation_table_runs_out_into_ln_two_rather_than_off_its_end() {
    assert_eq!(TaskSet::liu_layland_bound_ppm(0), 1_000_000);
    assert_eq!(TaskSet::liu_layland_bound_ppm(1), 1_000_000);
    assert_eq!(TaskSet::liu_layland_bound_ppm(2), 828_427);
    assert_eq!(TaskSet::liu_layland_bound_ppm(3), 779_763);
    assert_eq!(TaskSet::liu_layland_bound_ppm(MAX_TASKS), 708_380);

    for n in (MAX_TASKS + 1)..64 {
        assert_eq!(TaskSet::liu_layland_bound_ppm(n), 693_147);
    }

    // Monotone down across the table, with no step back up.
    for n in 2..MAX_TASKS {
        assert!(
            TaskSet::liu_layland_bound_ppm(n) > TaskSet::liu_layland_bound_ppm(n + 1),
            "bound must decrease at n = {n}"
        );
    }
}

/// A task's own jitter can add a job to its own busy period, and the job it
/// adds is not the worst one.
///
/// A(1, 10), B(2, 10) with D = 100.
///
/// Without jitter: L = 1 + 2 = 3, holding ⌈3/10⌉ = 1 job of B.
///   q = 0: w = 2 + ⌈w/10⌉·1 settles at 3. R = 3 − 0 + 0 = 3.
///
/// With J = 8 on B: L = ⌈L/10⌉·1 + ⌈(L+8)/10⌉·2 settles at 5, and the window
/// that counts for B is L + J = 13, holding ⌈13/10⌉ = 2 jobs.
///   q = 0: base = 2, w settles at 3. R = 3 − 0 + 8 = 11.
///   q = 1: base = 4, w settles at 5. R = 5 − 10 + 8 = 3.
///   worst = 11.
///
/// The second job is the one the extra iteration exists for, and it is the
/// smaller of the two. An implementation that stopped at q = 0 would get 11
/// here and be right by luck; `differential.rs` holds the case where it is not.
#[test]
fn a_tasks_own_jitter_can_add_a_job_to_its_own_busy_period() {
    let mut plain = TaskSet::new();
    plain.push(Task::new(1, 10)).unwrap();
    plain.push(Task::new(2, 10).deadline(100)).unwrap();
    assert_eq!(plain.response_of(1), Response::Bounded(3));

    let mut jittered = TaskSet::new();
    jittered.push(Task::new(1, 10)).unwrap();
    jittered
        .push(Task::new(2, 10).jitter(8).deadline(100))
        .unwrap();
    assert_eq!(jittered.response_of(1), Response::Bounded(11));

    // Jitter is added once, at the end, and not per iteration: 3 + 8 = 11.
    let widened = jittered.response_of(1).bound().unwrap();
    let plainly = plain.response_of(1).bound().unwrap();
    assert_eq!(widened - plainly, 8);
}

/// Audsley returns an ordering that works, which is not always the ordering the
/// reader expected, and never the one it silently applied.
///
/// A(1, 4), B(2, 6), C(2, 20) is already rate-monotonic and already holds. The
/// search still fills the lowest priority first and takes the first candidate
/// that fits there, so it returns [2, 0, 1] rather than the identity: task 0 at
/// the bottom, then task 1, leaving task 2 on top.
///
/// That is not a defect, and asserting the identity here would encode a wish
/// rather than the theorem. What Audsley proves is that if any ordering meets
/// every deadline, the search finds one. So the test applies what came back and
/// checks it holds, and separately pins the exact array, because the same input
/// must keep giving the same bits.
#[test]
fn audsley_returns_an_ordering_that_works_and_not_the_one_expected() {
    let mut s = TaskSet::new();
    s.push(Task::new(1, 4)).unwrap();
    s.push(Task::new(2, 6)).unwrap();
    s.push(Task::new(2, 20)).unwrap();

    assert!(s.is_schedulable(), "the set already holds as pushed");

    let order = s.optimal_priority_order().expect("an ordering exists");
    assert_eq!(order[..3], [2, 0, 1], "deterministic, and not the identity");

    let mut applied = TaskSet::new();
    for &i in &order[..3] {
        applied.push(*s.get(i).unwrap()).unwrap();
    }
    assert!(
        applied.is_schedulable(),
        "the ordering Audsley returned must actually hold"
    );

    assert_eq!(s.optimal_priority_order(), Some(order));
}

/// An index past the end is named at every entry point, and none of them
/// substitutes a number for the absence of a task.
#[test]
fn an_absent_task_is_named_rather_than_defaulted_at_every_entry_point() {
    let mut s = TaskSet::new();
    s.push(Task::new(100, 400)).unwrap();

    assert_eq!(
        s.response_of(9),
        Response::Refused(AnalysisFailure::NoSuchTask)
    );
    assert_eq!(s.slack_of(9), None);
    assert_eq!(s.max_wcet_increase(9), None);
    assert_eq!(s.get(9), None);
    assert_eq!(s.utilisation_through(9), s.utilisation_ppm());

    let empty = TaskSet::new();
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.first_failure(), None);
    assert_eq!(
        empty.response_of(0),
        Response::Refused(AnalysisFailure::NoSuchTask)
    );
}

/// `utilisation_through(usize::MAX)` panicked in debug and wrapped in release.
///
/// The wrap is the part worth a test. `index + 1` overflowed to zero, `take(0)`
/// counted nothing, and the function returned `Some(0)` — no utilisation at all
/// — for a set that was plainly busy. A caller pre-checking a set with that
/// number would be told the processor was idle. The panic was the loud half of
/// the same defect and the only half anyone would have noticed.
#[test]
fn utilisation_through_saturates_instead_of_wrapping() {
    let mut s = TaskSet::new();
    s.push(Task::new(1, 10)).unwrap();
    s.push(Task::new(2, 100)).unwrap();

    let all = s.utilisation_ppm().unwrap();
    assert_eq!(all, 120_000);

    // Deeper than any level that exists is still every task, not none of them.
    assert_eq!(s.utilisation_through(usize::MAX), Some(all));
    assert_eq!(s.utilisation_through(MAX_TASKS), Some(all));
    assert_eq!(s.utilisation_through(usize::MAX - 1), Some(all));
}

/// The three refusals that used to arrive as `NonConvergent` are now distinct.
///
/// `NonConvergent` makes a claim about the mathematics: no fixed point exists.
/// The other two say the implementation stopped — one at its iteration cap, one
/// at its job-enumeration cap — and in both of those the fixed point may exist.
/// A caller tuning a system needs to know which happened: one is answered by
/// changing the task set, the other by raising a cap.
#[test]
fn an_implementation_limit_is_not_a_mathematical_claim() {
    // Over-utilised: genuinely no fixed point.
    let mut over = TaskSet::new();
    over.push(Task::new(600, 1000)).unwrap();
    over.push(Task::new(600, 1000)).unwrap();
    assert_eq!(
        over.response_of(1),
        Response::Refused(AnalysisFailure::NonConvergent)
    );

    // Barely-loaded, huge blocking: the busy period closes, the job count is
    // finite and known, and the analysis declines to enumerate it.
    let mut capped = TaskSet::new();
    capped.push(Task::new(1, 1000)).unwrap();
    capped
        .push(Task::new(1, 1000).blocking(2_000_000).deadline(10_000_000))
        .unwrap();
    assert!(capped.utilisation_ppm().unwrap() < 10_000);
    assert_eq!(
        capped.response_of(1),
        Response::Refused(AnalysisFailure::BusyPeriodLimit)
    );

    // And the two do not compare equal, which is the whole point.
    assert_ne!(over.response_of(1), capped.response_of(1));
}
