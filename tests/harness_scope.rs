// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
// DY Research — https://dyresearch.github.io

//! What the Kani harnesses assume, measured rather than asserted.
//!
//! A harness narrows its inputs with `kani::assume` so that the loops inside
//! `response_of` close within a declared unwind bound. That makes the bound a
//! claim about the narrowed space — and until 3.0.5 nothing checked the claim.
//! `a_bounded_response_never_exceeds_its_deadline` declared five, ran against a
//! space where `jobs_in_window` reaches `BUSY_PERIOD_CAP`, and reported
//! "unwinding assertion loop 2" at `src/lib.rs:545` after thirty-one seconds.
//! Every other harness that calls `response_of` declared six or three, all
//! below the seventeen the sixteen-slot task array needs, so none of them could
//! have closed either. Nobody knew, because the job ran all six as one command
//! and was cancelled before any of them reported.
//!
//! This walks the same narrowed spaces concretely and records the largest loop
//! counts they produce. It is not a proof — it is the measurement the bound is
//! chosen from, so the number in the attribute has a reason behind it that a
//! reader can re-run in under a second.

use dy_wcet::{Response, Task, TaskSet, BUSY_PERIOD_CAP, ITERATION_CAP, MAX_TASKS};

/// `⌈(busy + jitter) / period⌉`, the count the per-job loop runs to.
fn jobs_in_window(busy: u64, jitter: u64, period: u64) -> u64 {
    let released = busy + jitter;
    let n = released / period + u64::from(released % period != 0);
    n.max(1)
}

/// The narrowing on `a_bounded_response_never_exceeds_its_deadline`:
/// `c + b + j < t` holds the busy period to a single job.
#[test]
fn one_job_narrowing_holds_the_per_job_loop_to_one_iteration() {
    let mut worst = 0u64;
    let mut checked = 0u64;

    for t in 2..220u64 {
        for c in 0..t {
            for b in 0..t {
                for j in 0..t {
                    if c + b + j >= t {
                        continue;
                    }
                    // The seed of the busy-period recurrence for one task.
                    let busy = b + c;
                    let n = jobs_in_window(busy, j, t);
                    worst = worst.max(n);
                    checked += 1;
                }
            }
        }
    }

    assert!(checked > 100_000, "only {checked} points were checked");
    assert_eq!(
        worst, 1,
        "the narrowing is supposed to leave exactly one job in the busy period"
    );
}

/// Without that narrowing the same loop is unbounded in practice, which is why
/// the harness could not be fixed by raising its number alone.
#[test]
fn without_the_narrowing_the_per_job_loop_reaches_the_cap() {
    // A short period under a long blocking term: the original harness admitted
    // exactly this, with t as small as 1 and b up to a million.
    let n = jobs_in_window(1_000_000, 0, 2);
    assert!(
        n > BUSY_PERIOD_CAP,
        "expected the job count to exceed the cap, got {n}"
    );
}

/// What actually has to be unwound, and why no bound could ever reach it.
///
/// 3.0.5 said the floor was `MAX_TASKS + 1` — the walk over the task array —
/// and raised every bound to eighteen. That was the wrong cause, and the
/// evidence was already in the log: the two unwinding assertions were at
/// `src/lib.rs:545` and `src/lib.rs:600`, and those lines are
/// `for _ in 0..ITERATION_CAP` and `for q in 0..jobs_in_window`. Constant
/// bounds of ten thousand and one thousand and twenty-four, the second nested
/// inside the first. CBMC unwinds a loop to its largest possible trip count
/// before it will assert anything past it, so eighteen was as hopeless as five.
///
/// The fix is the caps, not the bounds: `cfg(kani)` reduces them, and the
/// declared unwind values have to clear the reduced ones.
#[test]
fn the_shipped_caps_are_past_any_unwind_bound_and_the_kani_caps_are_not() {
    // At the shipped configuration these are the trip counts a proof would
    // have to unwind. Stated as numbers so the impossibility is visible.
    assert_eq!(ITERATION_CAP, 10_000);
    assert_eq!(BUSY_PERIOD_CAP, 1_024);
    assert_eq!(MAX_TASKS, 16);

    let src = include_str!("../src/lib.rs");
    for name in ["ITERATION_CAP", "BUSY_PERIOD_CAP", "MAX_TASKS"] {
        let reduced = src
            .split("#[cfg(kani)]")
            .skip(1)
            .find_map(|chunk| {
                let line = chunk
                    .lines()
                    .find(|l| l.contains(&format!("pub const {name}")))?;
                line.rsplit('=')
                    .next()?
                    .trim()
                    .trim_end_matches(';')
                    .parse::<u64>()
                    .ok()
            })
            .unwrap_or_else(|| panic!("no cfg(kani) reduction for {name}"));
        assert!(
            reduced <= 8,
            "{name} is {reduced} under cfg(kani); a solver has to unwind it"
        );
    }

    // And every declared bound clears the reduced floor.
    let harness = include_str!("../kani/response_bounds.rs");
    let bounds: Vec<u64> = harness
        .match_indices("#[kani::unwind(")
        .map(|(i, _)| {
            harness[i + 15..]
                .split(')')
                .next()
                .unwrap()
                .parse()
                .expect("a number inside kani::unwind")
        })
        .collect();
    assert!(bounds.len() >= 6, "found only {} bounds", bounds.len());
    assert!(
        bounds.iter().filter(|&&b| b >= 8).count() >= 5,
        "bounds {bounds:?} do not clear the reduced caps"
    );
}

/// The narrowed space is not empty, and it still exercises the claim.
#[test]
fn the_narrowed_space_still_produces_bounded_answers() {
    let mut bounded = 0;
    for t in 2..200u64 {
        for c in 1..t {
            let (b, j) = (0, 0);
            if c + b + j >= t {
                continue;
            }
            let mut s = TaskSet::new();
            if s.push(Task::new(c, t).deadline(t).blocking(b).jitter(j))
                .is_ok()
            {
                if let Response::Bounded(r) = s.response_of(0) {
                    assert!(r <= t, "response {r} above deadline {t}");
                    bounded += 1;
                }
            }
        }
    }
    assert!(
        bounded > 1_000,
        "only {bounded} bounded answers — the narrowing has emptied the space"
    );
}
