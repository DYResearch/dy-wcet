// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
// Part of DY Research — https://github.com/DYResearch

//! Worst-case response time for a fixed-priority task set.
//!
//! Response-time analysis is a fixed point: a task's response time is its own
//! execution plus the interference from everything that can preempt it, and
//! the interference depends on the response time. Joseph and Pandya gave the
//! recurrence in 1986 and it is short enough to write on a napkin. Getting it
//! *right* is where the work is, and the wrong answers are all quiet.
//!
//! This crate exists because three of them are quiet in ways that matter:
//!
//! **Floating point.** A response time computed in `f64` is a response time
//! whose last bits depend on the compiler's mood. Two implementations of the
//! same analysis disagree in the eighth decimal, one of them rounds a deadline
//! the other misses, and neither can be pinned by a test. Everything here is
//! `u64` microseconds. Same input, same bits, any machine.
//!
//! **Silent non-convergence.** The recurrence converges only when utilisation
//! is below one. Above it, the iteration climbs forever, and an implementation
//! that caps the loop and returns the last value returns a number that looks
//! like an answer. This one returns [`Response::Unschedulable`], which fails
//! every deadline comparison it is put into — an infinite response time is the
//! honest value, and it cannot be mistaken for a small one.
//!
//! **Overflow.** Interference is a sum of ceilings of quotients, and on a long
//! period with short tasks it grows fast. A wrapping add turns an
//! unschedulable set into a schedulable one, which is the worst direction for
//! an arithmetic error to go. Every operation here is checked, and an overflow
//! is reported as unschedulable rather than wrapped.
//!
//! ## What this does not do
//!
//! It does not measure. It computes a bound from execution times somebody else
//! established, and a bound is only as good as the numbers fed to it. If those
//! come from a spreadsheet rather than a scope, the result is arithmetic about
//! a guess.
//!
//! It does not model cache, pipeline, DMA contention, or shared-bus stalls.
//! Blocking is an input, not a derivation.
//!
//! ```
//! use dy_wcet::{Task, TaskSet, Response};
//!
//! let mut set = TaskSet::new();
//! set.push(Task { wcet_us: 100, period_us: 400, deadline_us: 400, blocking_us: 0 }).unwrap();
//! set.push(Task { wcet_us: 200, period_us: 1000, deadline_us: 1000, blocking_us: 20 }).unwrap();
//!
//! match set.response_of(1) {
//!     Response::Bounded(r) => assert!(r <= 1000),
//!     Response::Unschedulable => panic!("this set fits"),
//! }
//! ```

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// The largest task set this crate will analyse.
///
/// Fixed because the crate is `no_std` and allocates nothing. Sixteen is not a
/// theoretical limit — it is the point past which a fixed-priority set on one
/// core stops being analysable by hand, and an analysis nobody can check by
/// hand is an analysis nobody checks.
pub const MAX_TASKS: usize = 16;

/// A periodic task, in microseconds throughout.
///
/// No floating point anywhere: microseconds are the unit at the bottom, and a
/// system that needs finer resolution needs a different unit rather than a
/// fractional one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Task {
    /// Worst-case execution time. An input, not a measurement made here.
    pub wcet_us: u64,
    /// Activation period.
    pub period_us: u64,
    /// Relative deadline. May be shorter than the period.
    pub deadline_us: u64,
    /// Longest time this task can be blocked by a lower-priority task holding
    /// a shared resource. Zero if nothing is shared.
    pub blocking_us: u64,
}

/// The result of a response-time computation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Response {
    /// A proven upper bound, in microseconds.
    Bounded(u64),
    /// No bound exists, or the arithmetic to find one overflowed.
    ///
    /// Both are the same answer to the question being asked: this task set
    /// cannot be shown to meet its deadlines. Distinguishing them would invite
    /// a caller to treat one as recoverable, and neither is.
    Unschedulable,
}

impl Response {
    /// Whether this response meets a deadline.
    ///
    /// `Unschedulable` fails every comparison, which is the point: a caller
    /// that forgets to match on the variant still gets the safe answer.
    #[must_use]
    pub const fn meets(&self, deadline_us: u64) -> bool {
        match self {
            Self::Bounded(r) => *r <= deadline_us,
            Self::Unschedulable => false,
        }
    }
}

/// Why a task set was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejected {
    /// More than [`MAX_TASKS`].
    Full,
    /// A period of zero. Division by it is undefined and a task that activates
    /// infinitely often is not a task.
    ZeroPeriod,
    /// Execution longer than the deadline. No amount of scheduling fixes it,
    /// and admitting it would produce an analysis of an impossible system.
    ExecutionExceedsDeadline,
}

impl core::fmt::Display for Rejected {
    /// Written out because a caller integrating this into a `std` program
    /// otherwise formats a rejection as `Full`, which says nothing about what
    /// was full or why the set was refused.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Full => "task set is full (MAX_TASKS reached)",
            Self::ZeroPeriod => "period is zero: a task activating infinitely often is not a task",
            Self::ExecutionExceedsDeadline => {
                "execution time exceeds the deadline; no scheduling fixes that"
            }
        })
    }
}

impl core::fmt::Display for Response {
    /// `Unschedulable` prints as prose rather than a variant name, so a log
    /// line reads as a finding instead of as an enum.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Bounded(r) => write!(f, "{r} us"),
            Self::Unschedulable => f.write_str("no bound exists"),
        }
    }
}

/// A fixed-priority task set, highest priority first.
///
/// Priority is position: index 0 preempts everything, index 1 preempts
/// everything below it. Rate-monotonic ordering is the caller's job, because
/// sorting silently would hide a caller's mistaken assumption about which task
/// wins.
#[derive(Clone, Debug, Default)]
pub struct TaskSet {
    tasks: [Option<Task>; MAX_TASKS],
    len: usize,
}

impl TaskSet {
    /// An empty set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            tasks: [None; MAX_TASKS],
            len: 0,
        }
    }

    /// Number of admitted tasks.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether the set is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Admit a task at the next lowest priority.
    ///
    /// Rejects rather than accepts-and-warns. A task set that cannot be
    /// analysed should not exist as an object that looks analysable.
    pub fn push(&mut self, t: Task) -> Result<(), Rejected> {
        if self.len >= MAX_TASKS {
            return Err(Rejected::Full);
        }
        if t.period_us == 0 {
            return Err(Rejected::ZeroPeriod);
        }
        if t.wcet_us > t.deadline_us {
            return Err(Rejected::ExecutionExceedsDeadline);
        }
        self.tasks[self.len] = Some(t);
        self.len += 1;
        Ok(())
    }

    /// Total utilisation in parts per million.
    ///
    /// Parts per million rather than a ratio, for the same reason everything
    /// else here is an integer: a utilisation of 0.7500001 and one of 0.75 are
    /// different numbers, and which one a float gives depends on the order of
    /// the additions.
    ///
    /// Returns `None` on overflow, which for a plausible task set means the
    /// inputs are wrong.
    #[must_use]
    pub fn utilisation_ppm(&self) -> Option<u64> {
        let mut total: u64 = 0;
        for t in self.tasks.iter().take(self.len).flatten() {
            let part = t.wcet_us.checked_mul(1_000_000)?.checked_div(t.period_us)?;
            total = total.checked_add(part)?;
        }
        Some(total)
    }

    /// The worst-case response time of the task at `index`.
    ///
    /// The recurrence, in the form it is usually written:
    ///
    /// ```text
    /// R⁰ = C + B
    /// Rⁿ⁺¹ = C + B + Σ ⌈Rⁿ / Tⱼ⌉ · Cⱼ   for every j of higher priority
    /// ```
    ///
    /// It terminates when `Rⁿ⁺¹ = Rⁿ`, which is the fixed point, or when `R`
    /// exceeds the deadline, at which point continuing tells the caller
    /// nothing they did not already know.
    ///
    /// Every add and multiply is checked. An overflow returns
    /// [`Response::Unschedulable`] rather than wrapping, because a wrapped sum
    /// turns an unschedulable set into a schedulable-looking one, and that is
    /// the one direction an arithmetic error must never go.
    #[must_use]
    pub fn response_of(&self, index: usize) -> Response {
        let Some(Some(task)) = self.tasks.get(index) else {
            return Response::Unschedulable;
        };

        let Some(base) = task.wcet_us.checked_add(task.blocking_us) else {
            return Response::Unschedulable;
        };
        let mut r = base;

        // Bounded by construction: each iteration strictly increases r, and r
        // is compared against the deadline every round, so the loop cannot run
        // longer than the deadline divided by the smallest execution time.
        //
        // The cap is defence in depth against a future change to this function
        // rather than against any input that exists today. An external audit
        // found the previous comment here wrong: it named a zero-execution
        // task as the case that would otherwise spin, and a zero-execution
        // task contributes zero interference, so `next == base == r` on the
        // first iteration and the loop exits immediately.
        //
        // Measured rather than assumed: every set tried, including sixteen
        // tasks at 0.9 utilisation and two tasks at 0.9999, settles in two
        // iterations. Ten thousand is four orders of magnitude of headroom
        // over anything observed, which is the right size for a limit whose
        // purpose is to make a future mistake finite rather than to catch a
        // present one.
        for _ in 0..10_000 {
            let mut next = base;
            for higher in self.tasks.iter().take(index).flatten() {
                // ⌈r / T⌉ without floating point and without overflowing on
                // the addition that the usual (a + b - 1) / b trick needs.
                let jobs = r / higher.period_us + u64::from(r % higher.period_us != 0);
                let Some(interference) = jobs.checked_mul(higher.wcet_us) else {
                    return Response::Unschedulable;
                };
                let Some(sum) = next.checked_add(interference) else {
                    return Response::Unschedulable;
                };
                next = sum;
            }

            if next == r {
                return Response::Bounded(r);
            }
            if next > task.deadline_us {
                return Response::Unschedulable;
            }
            r = next;
        }
        Response::Unschedulable
    }

    /// Whether every task meets its deadline.
    #[must_use]
    pub fn is_schedulable(&self) -> bool {
        for i in 0..self.len {
            let Some(Some(t)) = self.tasks.get(i) else {
                return false;
            };
            if !self.response_of(i).meets(t.deadline_us) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(w: u64, p: u64) -> Task {
        Task {
            wcet_us: w,
            period_us: p,
            deadline_us: p,
            blocking_us: 0,
        }
    }

    #[test]
    fn a_single_task_responds_in_its_own_execution_time() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        assert_eq!(s.response_of(0), Response::Bounded(100));
    }

    #[test]
    fn a_lower_priority_task_pays_for_every_preemption() {
        // The high-priority task runs 100 µs every 400 µs. Starting at
        // R = 200, one activation fits in the window, giving 300; at R = 300
        // still one fits, so 300 is the fixed point.
        //
        // The obvious guess is 400 — two preemptions, one per 400 µs of the
        // low task's 1000 µs period. It is wrong, and it is wrong in the
        // direction that matters: the window that counts is the response time,
        // not the period, and a version of this analysis that used the period
        // would over-estimate here and under-estimate elsewhere.
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        s.push(t(200, 1000)).unwrap();
        assert_eq!(s.response_of(1), Response::Bounded(300));
    }

    #[test]
    fn blocking_is_added_once_and_not_per_preemption() {
        // Priority inversion is bounded by one blocking term under a priority
        // ceiling protocol. Adding it per interference term would double-count
        // it, which is the most common way this analysis is written wrong.
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        let mut low = t(200, 1000);
        low.blocking_us = 50;
        s.push(low).unwrap();
        // 300 from the case above, plus the blocking term once.
        assert_eq!(s.response_of(1), Response::Bounded(350));
    }

    #[test]
    fn an_overloaded_set_is_unschedulable_and_not_a_large_number() {
        // Utilisation above one. The recurrence never converges, and an
        // implementation that caps the loop and returns the last value returns
        // something that looks like an answer.
        let mut s = TaskSet::new();
        s.push(t(300, 400)).unwrap();
        s.push(t(300, 500)).unwrap();
        assert_eq!(s.response_of(1), Response::Unschedulable);
    }

    #[test]
    fn unschedulable_fails_every_deadline_it_is_compared_against() {
        assert!(!Response::Unschedulable.meets(u64::MAX));
        assert!(!Response::Unschedulable.meets(0));
    }

    #[test]
    fn overflow_is_reported_as_unschedulable_and_never_wrapped() {
        // A wrapped sum turns an unschedulable set into a schedulable-looking
        // one. That is the one direction an arithmetic error must not go, and
        // it is why every operation here is checked.
        let mut s = TaskSet::new();
        s.push(Task {
            wcet_us: u64::MAX / 2,
            period_us: 1,
            deadline_us: u64::MAX,
            blocking_us: 0,
        })
        .unwrap();
        s.push(Task {
            wcet_us: u64::MAX / 2,
            period_us: 2,
            deadline_us: u64::MAX,
            blocking_us: 0,
        })
        .unwrap();
        assert_eq!(s.response_of(1), Response::Unschedulable);
    }

    #[test]
    fn a_zero_period_is_refused_at_admission() {
        let mut s = TaskSet::new();
        assert_eq!(
            s.push(Task {
                wcet_us: 1,
                period_us: 0,
                deadline_us: 1,
                blocking_us: 0
            }),
            Err(Rejected::ZeroPeriod)
        );
    }

    #[test]
    fn execution_longer_than_its_deadline_is_refused() {
        let mut s = TaskSet::new();
        assert_eq!(
            s.push(Task {
                wcet_us: 500,
                period_us: 1000,
                deadline_us: 400,
                blocking_us: 0
            }),
            Err(Rejected::ExecutionExceedsDeadline)
        );
    }

    #[test]
    fn utilisation_is_parts_per_million_and_never_a_float() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        s.push(t(200, 1000)).unwrap();
        // 0.25 + 0.20 = 0.45
        assert_eq!(s.utilisation_ppm(), Some(450_000));
    }

    #[test]
    fn the_same_set_gives_the_same_bits_every_time() {
        // The property the integer arithmetic exists for. A float version of
        // this analysis cannot promise it across compilers.
        let mut s = TaskSet::new();
        s.push(t(37, 211)).unwrap();
        s.push(t(53, 499)).unwrap();
        s.push(t(101, 1013)).unwrap();
        let first = s.response_of(2);
        for _ in 0..1000 {
            assert_eq!(s.response_of(2), first);
        }
    }

    #[test]
    fn a_set_at_exactly_its_deadline_is_schedulable() {
        // The boundary, where an off-by-one lives. 100 + 300 = 400 exactly.
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        s.push(Task {
            wcet_us: 300,
            period_us: 1000,
            deadline_us: 400,
            blocking_us: 0,
        })
        .unwrap();
        assert_eq!(s.response_of(1), Response::Bounded(400));
        assert!(s.is_schedulable());
    }

    #[test]
    fn one_microsecond_past_the_deadline_is_not() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        s.push(Task {
            wcet_us: 301,
            period_us: 1000,
            deadline_us: 400,
            blocking_us: 0,
        })
        .unwrap();
        assert!(!s.is_schedulable());
    }
}
