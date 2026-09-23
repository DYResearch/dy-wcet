// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
// DY Research — https://dyresearch.github.io

//! Every snippet and every figure the README shows, compiled and asserted.
//!
//! Until 4.1.3 the README carried a `match` over four refusals when the enum
//! had six, so the example did not compile, and it said "the 85 tests" when
//! there were 96. Nothing checked either. A README is a set of claims about the
//! code, and a claim nothing checks drifts; this file is what checks these.

use dy_wcet::{AnalysisFailure, PriorityAssignment, Response, Task, TaskSet};

fn quick_start() -> TaskSet {
    let mut set = TaskSet::new();
    set.push(Task::new(100, 400).named("sensor")).unwrap();
    set.push(
        Task::new(200, 1000)
            .blocking(20)
            .jitter(15)
            .named("control"),
    )
    .unwrap();
    set
}

/// The puzzle at the top of the README: 300, not the 400 most people give.
#[test]
fn the_puzzle_answers_three_hundred() {
    let mut s = TaskSet::new();
    s.push(Task::new(100, 400)).unwrap();
    s.push(Task::new(200, 1000)).unwrap();
    assert_eq!(s.response_of(1), Response::Bounded(300));
}

/// The quick start, the exhaustive match, and the priority search, as written.
#[test]
fn the_quick_start_and_both_matches_compile_and_answer_as_shown() {
    let set = quick_start();
    assert_eq!(set.response_of(1), Response::Bounded(335));

    // Exhaustive over all six refusals, with no wildcard. If a seventh is
    // ever added this stops compiling, which is the point.
    let said = match set.response_of(1) {
        Response::Bounded(_) => "meets",
        Response::Refused(AnalysisFailure::ExceedsDeadline(_)) => "misses",
        Response::Refused(AnalysisFailure::NonConvergent) => "impossible",
        Response::Refused(AnalysisFailure::IterationLimit) => "iteration",
        Response::Refused(AnalysisFailure::BusyPeriodLimit) => "busy",
        Response::Refused(AnalysisFailure::Overflow) => "overflow",
        Response::Refused(AnalysisFailure::NoSuchTask) => "none",
    };
    assert_eq!(said, "meets");

    match set.optimal_priority_order() {
        PriorityAssignment::Found(_) => {}
        other => panic!("the README shows an order being found, got {other:?}"),
    }
}

/// The two sensitivity figures, and that 465 is the last value that fits.
#[test]
fn the_sensitivity_figures_are_the_ones_shown() {
    let set = quick_start();
    assert_eq!(set.slack_of(1), Some(665));
    assert_eq!(set.max_provable_wcet_increase(1), Some(465));

    let at = |c: u64| {
        let mut s = TaskSet::new();
        s.push(Task::new(100, 400)).unwrap();
        s.push(Task::new(c, 1000).blocking(20).jitter(15)).unwrap();
        s.response_of(1)
    };
    assert_eq!(
        at(200 + 465),
        Response::Bounded(1000),
        "465 more lands on the deadline"
    );
    assert_eq!(
        at(200 + 466),
        Response::Refused(AnalysisFailure::ExceedsDeadline(1001)),
        "466 more is one past it"
    );
}

/// The "What it prints" column of the refusal table, word for word.
#[test]
fn every_refusal_prints_what_the_readme_table_says() {
    let table = include_str!("../README.md");
    for f in [
        AnalysisFailure::ExceedsDeadline(412),
        AnalysisFailure::NonConvergent,
        AnalysisFailure::IterationLimit,
        AnalysisFailure::BusyPeriodLimit,
        AnalysisFailure::Overflow,
        AnalysisFailure::NoSuchTask,
    ] {
        let printed = f.to_string();
        assert!(
            table.contains(&printed),
            "{f:?} prints {printed:?}, and the README table does not say that"
        );
    }
}
