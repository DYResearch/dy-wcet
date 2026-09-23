// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
// DY Research — https://dyresearch.github.io

//! Exact saturation, at the one boundary where it decides anything.
//!
//! `response_of` refuses any level whose floored utilisation exceeds one
//! million parts per million before it ever asks whether the level is
//! saturated. So `level_is_saturated` decides only the band where the floor
//! lands on exactly 1,000,000 while the true utilisation is at or above one:
//! U in [1.0, 1.000001).
//!
//! Until 4.1.0 it reduced the fraction with a gcd on `u128`, carried every
//! task through after the sum had already reached one, and overflowed on a
//! set with enough coprime periods. It then reported a limit reached where
//! the level is provably impossible.

use dy_wcet::{AnalysisFailure, Response, Task, TaskSet, MAX_TASKS};

/// The fourteen smallest primes above one million. Each contributes
/// `⌊10⁶/p⌋ = 0` parts per million, so the floored gate sees exactly the one
/// million the first two tasks bring and lets the level through. Coprime to
/// each other and to 2, so an exact common denominator is their product:
/// 2 · (10⁶)¹⁴ ≈ 2·10⁸⁴, far past `u128::MAX` ≈ 3.4·10³⁸.
const WIDE: [u64; 14] = [
    1_000_003, 1_000_033, 1_000_037, 1_000_039, 1_000_081, 1_000_099, 1_000_117, 1_000_121,
    1_000_133, 1_000_151, 1_000_159, 1_000_171, 1_000_183, 1_000_187,
];

fn exactly_full_then_wide() -> TaskSet {
    let mut s = TaskSet::new();
    // 1/2 + 1/2 = 1 exactly after two tasks. The jitter is the disturbance
    // that makes a saturated level non-convergent rather than merely full.
    s.push(Task::new(1, 2).jitter(1)).unwrap();
    s.push(Task::new(1, 2)).unwrap();
    for &p in &WIDE {
        s.push(Task::new(1, p)).unwrap();
    }
    assert_eq!(s.len(), MAX_TASKS);
    s
}

#[test]
fn the_floored_gate_lets_this_level_through() {
    // The premise of every test below: the cheap check must not be what
    // decides these. If it did, they would pass for the wrong reason.
    let s = exactly_full_then_wide();
    assert_eq!(s.utilisation_through(MAX_TASKS - 1), Some(1_000_000));
}

#[test]
fn a_level_full_at_the_second_task_is_proved_impossible() {
    // True utilisation 1 + Σ 1/p ≈ 1.000014, so no fixed point exists, and the
    // jitter means the busy period never closes. That is a proof of
    // impossibility and the answer has to say so rather than report a limit.
    let s = exactly_full_then_wide();
    assert_eq!(
        s.response_of(MAX_TASKS - 1),
        Response::Refused(AnalysisFailure::NonConvergent),
    );
}

#[test]
fn every_level_from_the_saturating_one_down_is_impossible() {
    let s = exactly_full_then_wide();
    for i in 1..MAX_TASKS {
        assert_eq!(
            s.response_of(i),
            Response::Refused(AnalysisFailure::NonConvergent),
            "level {i} is at or above one from the second task on",
        );
    }
}

#[test]
fn a_level_below_one_is_never_called_saturated() {
    // The wide tasks alone sum to about 1.4·10⁻⁵.
    let mut s = TaskSet::new();
    for &p in &WIDE {
        s.push(Task::new(1, p).jitter(1)).unwrap();
    }
    for i in 0..s.len() {
        assert_ne!(
            s.response_of(i),
            Response::Refused(AnalysisFailure::NonConvergent),
            "level {i} is at about 10⁻⁵ utilisation",
        );
    }
}
