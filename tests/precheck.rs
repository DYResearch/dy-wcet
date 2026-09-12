// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
// DY Research — https://dyresearch.github.io

//! The Liu and Layland pre-check, and the preconditions it depends on.

use dy_wcet::{Response, Task, TaskSet, Unbounded};

/// ```text
/// hi (10, 100) D = T          lo (20, 200) D = 25
/// U = 0.1 + 0.1 = 0.2, against the two-task bound of 0.828427
/// R(1): w = 20 → 20 + ⌈20/100⌉·10 = 30 → 30 + ⌈30/100⌉·10 = 30   R = 30 > 25
/// ```
/// Rate-monotonic, a fifth of the bound, and it misses. The only Liu and
/// Layland condition broken is `D = T`, which the bound also requires.
#[test]
fn a_constrained_deadline_is_outside_the_bound_and_the_precheck_says_so() {
    let mut s = TaskSet::new();
    s.push(Task::new(10, 100)).unwrap();
    s.push(Task::new(20, 200).deadline(25)).unwrap();

    assert_eq!(s.utilisation_ppm(), Some(200_000));
    assert!(s.utilisation_ppm().unwrap() < TaskSet::liu_layland_bound_ppm(2));
    assert_eq!(
        s.response_of(1),
        Response::Unbounded(Unbounded::ExceedsDeadline(30))
    );
    assert!(!s.is_schedulable());
    assert!(
        !s.passes_utilisation_bound(),
        "the bound must not claim a set it does not cover"
    );
}

/// Priorities the caller got backwards are outside the theorem too.
#[test]
fn a_set_that_is_not_rate_monotonic_is_outside_the_bound() {
    let mut s = TaskSet::new();
    s.push(Task::new(40, 200)).unwrap();
    s.push(Task::new(10, 100)).unwrap();
    assert!(!s.passes_utilisation_bound());
}

/// Blocking and jitter are outside it as well: neither appears in the theorem.
#[test]
fn blocking_and_jitter_are_outside_the_bound() {
    let mut s = TaskSet::new();
    s.push(Task::new(10, 100).blocking(5)).unwrap();
    s.push(Task::new(10, 200)).unwrap();
    assert!(!s.passes_utilisation_bound());
}

/// What the bound does cover still passes.
#[test]
fn an_implicit_deadline_rate_monotonic_set_under_the_bound_passes() {
    let mut s = TaskSet::new();
    s.push(Task::new(10, 100)).unwrap();
    s.push(Task::new(20, 200)).unwrap();
    assert!(s.passes_utilisation_bound());
    assert!(s.is_schedulable());
}

/// Every entry is the floor of `n·(2^(1/n) − 1)`, never the rounding of it.
/// A bound rounded up admits a set the theorem does not cover, which is the
/// flattering direction. Six of the sixteen entries were rounded up until
/// 2.0.0, the worst by five parts per million at n = 11.
#[test]
fn the_bound_table_is_floored_and_never_rounded_up() {
    // floor(n·(2^(1/n) − 1)·10⁶), computed to forty digits offline
    const TRUE_FLOOR: [u64; 17] = [
        1_000_000, 1_000_000, 828_427, 779_763, 756_828, 743_491, 734_772, 728_626, 724_061,
        720_537, 717_734, 715_451, 713_557, 711_958, 710_592, 709_411, 708_380,
    ];
    for (n, &floor) in TRUE_FLOOR.iter().enumerate() {
        assert_eq!(
            TaskSet::liu_layland_bound_ppm(n),
            floor,
            "bound for n = {n} is not the floor of the true value"
        );
    }
    // past the table the fallback is ln 2, floored
    assert_eq!(TaskSet::liu_layland_bound_ppm(17), 693_147);
}
