// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
// DY Research — https://dyresearch.github.io

//! Independent busy-window oracle.
//!
//! `response_of` derives the job count from the busy-period length. This
//! computes it the other way — growing q while the work through job q still
//! spills past the q+1 periods that released it — and the two must agree.
//! If the derived count is ever short, the analysis under-reports, which is
//! the failure this whole exercise exists to refuse.

use dy_wcet::{AnalysisFailure, Response, Task, TaskSet, FULL_UTILISATION_PPM, ITERATION_CAP};

fn solve_w(s: &TaskSet, index: usize, base: u64) -> Option<u64> {
    let mut w = base;
    for _ in 0..ITERATION_CAP {
        let mut next = base;
        for i in 0..index {
            let h = *s.get(i)?;
            let window = w.checked_add(h.jitter_us)?;
            let jobs = window / h.period_us + u64::from(window % h.period_us != 0);
            next = next.checked_add(jobs.checked_mul(h.wcet_us)?)?;
        }
        if next == w {
            return Some(w);
        }
        w = next;
    }
    None
}

/// Textbook incremental form: q grows while w(q) > (q+1)·T.
fn oracle(s: &TaskSet, index: usize) -> Option<u64> {
    let task = *s.get(index)?;
    match s.utilisation_through(index) {
        Some(u) if u <= FULL_UTILISATION_PPM => {}
        _ => return None,
    }
    let mut worst = 0u64;
    for q in 0..100_000u64 {
        let base = q
            .checked_add(1)?
            .checked_mul(task.wcet_us)?
            .checked_add(task.blocking_us)?;
        let w = solve_w(s, index, base)?;
        // Jitter in before the shift out, for the reason given in `src/lib.rs`:
        // w(q) − q·T is negative for a jittered job that completes before its
        // nominal offset, and this oracle refused those sets in exactly the
        // same place the implementation did. A second opinion that shares the
        // first one's arithmetic is not a second opinion.
        let r = w
            .checked_add(task.jitter_us)?
            .checked_sub(q.checked_mul(task.period_us)?)?;
        worst = worst.max(r);
        if w <= q.checked_add(1)?.checked_mul(task.period_us)? {
            return Some(worst);
        }
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
fn the_derived_job_count_never_stops_short_of_the_busy_period() {
    let mut rng = Lcg(0xC0FF_EE99);
    let (mut agreed, mut skipped) = (0u32, 0u32);

    for _ in 0..60_000 {
        let n = 2 + (rng.bits() % 3) as usize;
        let mut s = TaskSet::new();
        for _ in 0..n {
            let period = rng.between(3, 200);
            let wcet = rng.between(1, period / 2 + 1);
            let deadline = rng.between(wcet, period.saturating_mul(6));
            let blocking = if rng.bits() % 3 == 0 {
                rng.between(1, 40)
            } else {
                0
            };
            let jitter = if rng.bits() % 3 == 0 {
                rng.between(1, 25)
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
            let got = match s.response_of(i) {
                Response::Bounded(r) => r,
                Response::Refused(AnalysisFailure::ExceedsDeadline(r)) => r,
                _ => {
                    skipped += 1;
                    continue;
                }
            };
            let Some(want) = oracle(&s, i) else {
                skipped += 1;
                continue;
            };
            assert!(
                got >= want,
                "response_of returned {got}, below the oracle's {want}: a job was missed"
            );
            assert_eq!(got, want, "response_of {got} vs oracle {want}");
            agreed += 1;
        }
    }

    assert!(agreed > 20_000, "only {agreed} agreements");
    std::eprintln!("oracle agreed on {agreed}, skipped {skipped}");
}
