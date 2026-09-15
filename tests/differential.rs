// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
// DY Research — https://dyresearch.github.io

//! Differential check of the busy-window analysis against the single-job
//! recurrence it replaces.
//!
//! Two things must hold for the change to be safe to ship:
//!
//!   1. Where the old recurrence was sound — a response at or inside the
//!      task's own period — the two must agree bit for bit.
//!   2. Nowhere may the new answer be smaller than the old one. A smaller
//!      answer is the flattering direction, and the whole crate exists to
//!      refuse it.

use dy_wcet::{Response, Task, TaskSet, FULL_UTILISATION_PPM, ITERATION_CAP};

/// The v1.2.1 recurrence, kept here as the thing under comparison.
fn old_response_of(s: &TaskSet, index: usize) -> Option<u64> {
    let task = s.get(index).copied()?;
    match s.utilisation_through(index) {
        None => return None,
        Some(u) if u > FULL_UTILISATION_PPM => return None,
        Some(_) => {}
    }
    let base = task.wcet_us.checked_add(task.blocking_us)?;
    let mut w = base;
    for _ in 0..ITERATION_CAP {
        let mut next = base;
        for i in 0..index {
            let h = s.get(i).copied()?;
            let window = w.checked_add(h.jitter_us)?;
            let jobs = window / h.period_us + u64::from(window % h.period_us != 0);
            next = next.checked_add(jobs.checked_mul(h.wcet_us)?)?;
        }
        if next == w {
            return w.checked_add(task.jitter_us);
        }
        w = next;
    }
    None
}

struct Lcg(u64);
impl Lcg {
    fn bits(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn between(&mut self, lo: u64, hi: u64) -> u64 {
        if hi <= lo {
            lo
        } else {
            lo + self.bits() % (hi - lo + 1)
        }
    }
}

#[test]
fn the_busy_window_agrees_where_the_old_recurrence_was_sound_and_never_flatters_elsewhere() {
    let mut rng = Lcg(0x5eed_1234);
    let mut compared = 0u32;
    let mut diverged = 0u32;

    for _ in 0..40_000 {
        let n = 2 + (rng.bits() % 3) as usize;
        let mut s = TaskSet::new();
        for _ in 0..n {
            let period = rng.between(4, 120);
            let wcet = rng.between(1, period / 2 + 1);
            // deadlines deliberately allowed past the period: the region the
            // old recurrence claimed to support and did not
            let deadline = rng.between(wcet, period.saturating_mul(4));
            let blocking = if rng.bits() % 4 == 0 {
                rng.between(1, 30)
            } else {
                0
            };
            let jitter = if rng.bits() % 4 == 0 {
                rng.between(1, 20)
            } else {
                0
            };
            let _ = s.push(
                Task::new(wcet, period)
                    .deadline(deadline)
                    .blocking(blocking)
                    .jitter(jitter),
            );
        }

        for i in 0..s.iter().count() {
            let task = *s.get(i).unwrap();
            let old = old_response_of(&s, i);
            let new = match s.response_of(i) {
                Response::Bounded(r) => Some(r),
                Response::Refused(dy_wcet::AnalysisFailure::ExceedsDeadline(r)) => Some(r),
                _ => None,
            };

            let (o, nw) = match (old, new) {
                (Some(a), Some(b)) => (a, b),
                _ => continue,
            };
            compared += 1;

            // 2. never smaller than the old answer
            assert!(
                nw >= o,
                "busy window returned {nw}, below the old {o}, which is the flattering direction"
            );

            // 1. identical wherever the old answer stayed inside the period
            if o <= task.period_us {
                assert_eq!(
                    nw, o,
                    "old answer {o} was inside the period and must be preserved, got {nw}"
                );
            } else if nw != o {
                diverged += 1;
            }
        }
    }

    assert!(compared > 10_000, "only {compared} sets compared");
    assert!(
        diverged > 0,
        "no set exercised the self-interference region; the generator is not reaching it"
    );
    std::eprintln!("compared {compared}, diverged {diverged} (all upward, all past the period)");
}

/// The witness from the audit, kept as a fixed case.
///
/// ```text
/// hi: C=5  T=10 D=10
/// lo: C=2  T=5  D=10 B=2      U = 0.9
///
/// single job:   w = 2+2 = 4 → 4 + ⌈4/10⌉·5  = 9 → 9 + ⌈9/10⌉·5 = 9   R = 9  ≤ 10
/// busy window:  q=1 → w = 2·2+2 = 6 → 6 + ⌈11/10⌉·5 = 16           R = 16 − 5 = 11 > 10
/// ```
///
/// Job two of the low-priority task is released at t = 5, starts at 9, is
/// preempted at 10 by the high-priority task's second activation, and finishes
/// at 16. Eleven microseconds after its own release, against a deadline of ten.
#[test]
fn a_response_past_its_own_period_pays_for_its_own_next_job() {
    let mut s = TaskSet::new();
    s.push(Task::new(5, 10).deadline(10)).unwrap();
    s.push(Task::new(2, 5).deadline(10).blocking(2)).unwrap();

    assert_eq!(old_response_of(&s, 1), Some(9));
    assert_eq!(
        s.response_of(1),
        Response::Refused(dy_wcet::AnalysisFailure::ExceedsDeadline(11))
    );
    assert!(!s.is_schedulable());
    assert_eq!(s.slack_of(1), None);
}
