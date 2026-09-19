// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
// DY Research — https://dyresearch.github.io

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
//! is at most one. Above it, the iteration climbs forever, and an
//! implementation that caps the loop and returns the last value returns a
//! number that looks like an answer. This one returns
//! [`AnalysisFailure::NonConvergent`], which fails every deadline comparison it is
//! put into.
//!
//! **Overflow.** Interference is a sum of ceilings of quotients, and on a long
//! period with short tasks it grows fast. A wrapping add turns an
//! unschedulable set into a schedulable one, which is the worst direction for
//! an arithmetic error to go. Every operation here is checked, and an overflow
//! is reported as [`AnalysisFailure::Overflow`] rather than wrapped.
//!
//! # The model
//!
//! Periodic or sporadic tasks on one core, fixed priority, priority by
//! position. Each task carries a worst-case execution time, a period, a
//! relative deadline that may be shorter or longer than the period, a blocking
//! term, and a release jitter. The recurrence solved is the jitter-extended
//! form:
//!
//! ```text
//! w⁰   = C + B
//! wⁿ⁺¹ = C + B + Σ ⌈(wⁿ + Jⱼ) / Tⱼ⌉ · Cⱼ     for every j of higher priority
//! R    = w + J
//! ```
//!
//! With every `J` zero this reduces to Joseph and Pandya exactly, which is
//! checked by a test rather than asserted here.
//!
//! # What this does not do
//!
//! It does not measure. It computes a bound from execution times somebody else
//! established, and a bound is only as good as the numbers fed to it. If those
//! come from a spreadsheet rather than a scope, the result is arithmetic about
//! a guess.
//!
//! It does not model cache, pipeline, DMA contention, or shared-bus stalls.
//! Blocking is an input, not a derivation.
//!
//! It assumes a priority-ceiling protocol, under which a task is blocked at
//! most once per release. Without one the blocking term is not a single number
//! and this analysis does not apply.
//!
//! ```
//! use dy_wcet::{Task, TaskSet, Response};
//!
//! let mut set = TaskSet::new();
//! set.push(Task::new(100, 400).named("sensor")).unwrap();
//! set.push(Task::new(200, 1000).blocking(20).named("control")).unwrap();
//!
//! match set.response_of(1) {
//!     Response::Bounded(r) => assert_eq!(r, 320),
//!     Response::Refused(why) => panic!("this set fits: {why}"),
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
#[cfg(not(kani))]
pub const MAX_TASKS: usize = 16;

/// Four under `cfg(kani)`. Every loop in [`TaskSet::response_of`] walks this
/// array, and each walk sits inside the two bounded loops above, so the array
/// length multiplies everything else. Four keeps a set with a genuine
/// higher-priority interference term — which is the thing the proofs are
/// about — at a size a solver finishes.
#[cfg(kani)]
pub const MAX_TASKS: usize = 4;

/// One hundred per cent utilisation, expressed in parts per million.
///
/// The unit everything here reports utilisation in, for the same reason
/// everything else is an integer: 0.7500001 and 0.75 are different numbers,
/// and which one a float gives you depends on the order of the additions.
pub const FULL_UTILISATION_PPM: u64 = 1_000_000;

/// The iteration ceiling on the fixed-point search.
///
/// Not a correctness mechanism. Convergence is decided before the loop by
/// [`TaskSet::utilisation_through`], and each iteration strictly increases the
/// window, so the loop terminates on its own. The cap is defence against a
/// future change to that function, and its size is measured rather than
/// assumed: every set tried, including sixteen tasks at 0.9 utilisation and
/// two at 0.9999, settles in two iterations.
#[cfg(not(kani))]
pub const ITERATION_CAP: u32 = 10_000;

/// Six under `cfg(kani)`, and this is the reason the proofs could not run.
///
/// CBMC has to unwind a loop to its largest possible trip count before it can
/// assert anything past it. `for _ in 0..ITERATION_CAP` has a constant bound
/// of ten thousand, and the loop over `jobs_in_window` a constant bound of
/// [`BUSY_PERIOD_CAP`]; the second is nested inside the first. No value of
/// `#[kani::unwind]` reaches that, and raising the number from five to
/// eighteen — which is what 3.0.5 did — moved the failure not at all. The two
/// unwinding assertions the runner reported were at `src/lib.rs:545` and
/// `src/lib.rs:600`, which are exactly these two loops and not, as 3.0.5
/// claimed, the walk over the task array.
///
/// Reducing the caps under `cfg(kani)` is what makes them reachable, and it
/// narrows what the proofs establish: the same code with a smaller refusal
/// threshold. The arithmetic, the recurrence and the refusal paths are
/// identical; what shrinks is how far the search may run before the analysis
/// gives up. That is a scope, so it is written here rather than left in a
/// build flag.
#[cfg(kani)]
pub const ITERATION_CAP: u32 = 6;

/// How many jobs of one task a level-i busy period may contain before the
/// analysis refuses it. A busy period longer than this is a set no reader
/// could check by hand, which is the same reason MAX_TASKS is sixteen.
#[cfg(not(kani))]
pub const BUSY_PERIOD_CAP: u64 = 1_024;

/// Four under `cfg(kani)`. See [`ITERATION_CAP`] for why the shipped value
/// cannot be unwound; this is the loop at `src/lib.rs:600` that reported the
/// second unwinding assertion.
#[cfg(kani)]
pub const BUSY_PERIOD_CAP: u64 = 4;

/// A periodic or sporadic task, in microseconds throughout.
///
/// No floating point anywhere: microseconds are the unit at the bottom, and a
/// system that needs finer resolution needs a different unit rather than a
/// fractional one.
///
/// Construct with [`Task::new`] and the chaining setters, which give a task
/// whose unset terms are zero rather than whatever was in the caller's head:
///
/// ```
/// use dy_wcet::Task;
/// let t = Task::new(200, 1000).deadline(800).blocking(20).jitter(15).named("control");
/// assert_eq!(t.deadline_us, 800);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Task {
    /// Worst-case execution time. An input, not a measurement made here.
    pub wcet_us: u64,
    /// Activation period, or minimum inter-arrival time for a sporadic task.
    pub period_us: u64,
    /// Relative deadline. May be shorter or longer than the period.
    pub deadline_us: u64,
    /// Longest time this task can be blocked by a lower-priority task holding
    /// a shared resource. Zero if nothing is shared.
    pub blocking_us: u64,
    /// Release jitter: the longest delay between the task's nominal release
    /// and its actual release.
    ///
    /// Widens the interference window that lower-priority tasks see, and adds
    /// directly to this task's own response time. Zero for a task released by
    /// a hardware timer with no queueing in front of it.
    pub jitter_us: u64,
    /// A label carried into `Display` output and analysis reports.
    ///
    /// `&'static str` rather than an owned string, because this crate
    /// allocates nothing.
    pub name: &'static str,
}

impl Task {
    /// A task with the given execution time and period.
    ///
    /// Deadline equals the period; blocking and jitter are zero; the name is
    /// empty. Refine with the setters below.
    #[must_use]
    pub const fn new(wcet_us: u64, period_us: u64) -> Self {
        Self {
            wcet_us,
            period_us,
            deadline_us: period_us,
            blocking_us: 0,
            jitter_us: 0,
            name: "",
        }
    }

    /// Set the relative deadline.
    #[must_use]
    pub const fn deadline(mut self, deadline_us: u64) -> Self {
        self.deadline_us = deadline_us;
        self
    }

    /// Set the blocking term.
    #[must_use]
    pub const fn blocking(mut self, blocking_us: u64) -> Self {
        self.blocking_us = blocking_us;
        self
    }

    /// Set the release jitter.
    #[must_use]
    pub const fn jitter(mut self, jitter_us: u64) -> Self {
        self.jitter_us = jitter_us;
        self
    }

    /// Set the label.
    #[must_use]
    pub const fn named(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    /// This task's utilisation in parts per million, or `None` if the period
    /// is zero or the arithmetic overflows.
    #[must_use]
    pub fn utilisation_ppm(&self) -> Option<u64> {
        if self.period_us == 0 {
            return None;
        }
        self.wcet_us
            .checked_mul(FULL_UTILISATION_PPM)
            .map(|n| n / self.period_us)
    }
}

/// The outcome of a priority search.
///
/// Through 3.0.9 this was an `Option`, and `None` carried two different
/// statements: *no ordering of this set meets every deadline*, and *at least
/// one candidate ordering could not be analysed, so nothing is known about
/// it*. The first is a result. The second is a refusal, and a caller acting on
/// it as though it were the first would be reordering a system on the strength
/// of an answer that was never given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityAssignment {
    /// Original indices in priority order, highest first. Every task in this
    /// ordering was shown to meet its deadline at the level it holds.
    Found([usize; MAX_TASKS]),
    /// Every candidate at some level was analysed and every one of them missed
    /// its deadline there. Audsley's result then says no ordering works.
    NoOrdering,
    /// A level had no placeable candidate, and at least one candidate was
    /// declined by the analysis rather than shown to miss. An ordering may
    /// exist; this search cannot say. Carries the first such refusal.
    Inconclusive(AnalysisFailure),
}

impl PriorityAssignment {
    /// The ordering, if one was established.
    #[must_use]
    pub fn found(self) -> Option<[usize; MAX_TASKS]> {
        match self {
            Self::Found(order) => Some(order),
            _ => None,
        }
    }

    /// Whether the search established that no ordering works. False for
    /// `Inconclusive`, which is the distinction this type exists for.
    #[must_use]
    pub fn is_proven_impossible(self) -> bool {
        matches!(self, Self::NoOrdering)
    }
}

/// Why the analysis returned no usable bound.
///
/// The distinction this enum exists to hold is between *the mathematics has no
/// answer* and *this implementation declined to keep going*. Until 3.0.0 both
/// arrived as `NonConvergent`, whose `Display` said "utilisation exceeds one;
/// no fixed point exists" — a specific mathematical claim, printed in three
/// situations where it had not been established. A reader tuning a system needs
/// to know which of the two happened, because one of them is answered by
/// changing the task set and the other by raising a cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisFailure {
    /// Utilisation through this priority level exceeds one, so the recurrence
    /// has no fixed point. No amount of iterating produces a number.
    ///
    /// This is the only variant that makes a claim about the mathematics.
    NonConvergent,
    /// The recurrence did not settle within [`ITERATION_CAP`].
    ///
    /// Not a proof of non-convergence: the fixed point may exist just beyond
    /// the cap. It says the implementation stopped, and where.
    IterationLimit,
    /// The busy period closed, its length is known and finite, and it holds
    /// more jobs of this task than [`BUSY_PERIOD_CAP`].
    ///
    /// The analysis refused to enumerate them. Nothing here is unbounded — the
    /// bound was not computed because enumerating it was declined.
    BusyPeriodLimit,
    /// The recurrence converged, and it converged above the deadline. The
    /// value is the true response time: a real bound, on a task that misses.
    ///
    /// This is the case worth acting on. It says by how much.
    ExceedsDeadline(u64),
    /// Checked arithmetic refused rather than wrapping.
    ///
    /// A wrapped sum turns an unschedulable set into a schedulable-looking
    /// one, which is the one direction an arithmetic error must never go.
    Overflow,
    /// The index addresses no admitted task.
    NoSuchTask,
}

impl AnalysisFailure {
    /// Whether this refusal establishes something about the task set, or only
    /// reports that the analysis stopped.
    ///
    /// The distinction decides what a caller may conclude. `ExceedsDeadline`
    /// and `NonConvergent` are results: the first proves a miss, the second
    /// proves that no fixed point exists, which is as final as a miss and for
    /// a stronger reason. The rest are limits of this implementation —
    /// iteration caps, job caps, arithmetic declined — and a set behind one of
    /// them has not been shown to be anything.
    ///
    /// [`PriorityAssignment`] turns on exactly this: a level whose candidates
    /// were all refused determinately has no ordering, and a level with an
    /// indeterminate refusal among them has not been decided.
    #[must_use]
    pub fn is_determinate(self) -> bool {
        match self {
            // Proven: the response converges, past the deadline.
            Self::ExceedsDeadline(_) => true,
            // Proven: f(L) > L for every L, so no bound exists at all. A
            // utilisation above one lands here, and no priority ordering
            // rescues a set that asks for more processor than exists.
            Self::NonConvergent => true,
            // Stopped, capped, or declined. Says nothing about the set.
            Self::IterationLimit | Self::BusyPeriodLimit => false,
            Self::Overflow => false,
            // A caller error rather than a property of any task set.
            Self::NoSuchTask => false,
        }
    }
}

impl core::fmt::Display for AnalysisFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonConvergent => f.write_str("utilisation exceeds one; no fixed point exists"),
            Self::IterationLimit => {
                f.write_str("the recurrence did not settle within the iteration cap")
            }
            Self::BusyPeriodLimit => {
                f.write_str("the busy period is bounded but holds more jobs than the cap admits")
            }
            Self::ExceedsDeadline(r) => write!(f, "converges at {r} us, past the deadline"),
            Self::Overflow => f.write_str("the arithmetic overflowed and was refused"),
            Self::NoSuchTask => f.write_str("no task at that index"),
        }
    }
}

/// The result of a response-time computation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Response {
    /// A proven upper bound, in microseconds, at or below the deadline.
    Bounded(u64),
    /// No usable bound, and why. See [`AnalysisFailure`].
    Refused(AnalysisFailure),
}

impl Response {
    /// Whether this response meets a deadline.
    ///
    /// Every [`Response::Refused`] fails every comparison, which is the
    /// point: a caller that forgets to match on the variant still gets the
    /// safe answer.
    #[must_use]
    pub const fn meets(&self, deadline_us: u64) -> bool {
        match self {
            Self::Bounded(r) => *r <= deadline_us,
            Self::Refused(_) => false,
        }
    }

    /// The bound, if there is one at or below the deadline.
    #[must_use]
    pub const fn bound(&self) -> Option<u64> {
        match self {
            Self::Bounded(r) => Some(*r),
            Self::Refused(_) => None,
        }
    }

    /// The response time whether or not it meets the deadline.
    ///
    /// [`AnalysisFailure::ExceedsDeadline`] carries a real number, and a caller
    /// asking "how badly" wants it. Non-convergence and overflow still have
    /// nothing to give.
    #[must_use]
    pub const fn response_time(&self) -> Option<u64> {
        match self {
            Self::Bounded(r) => Some(*r),
            Self::Refused(AnalysisFailure::ExceedsDeadline(r)) => Some(*r),
            Self::Refused(_) => None,
        }
    }

    /// Whether a bound at or below the deadline was found.
    #[must_use]
    pub const fn is_bounded(&self) -> bool {
        matches!(self, Self::Bounded(_))
    }

    /// Why there is no bound, if there is none.
    #[must_use]
    pub const fn reason(&self) -> Option<AnalysisFailure> {
        match self {
            Self::Refused(u) => Some(*u),
            Self::Bounded(_) => None,
        }
    }
}

impl core::fmt::Display for Response {
    /// Prints as prose rather than as a variant name, so a log line reads as a
    /// finding instead of as an enum.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Bounded(r) => write!(f, "{r} us"),
            Self::Refused(why) => write!(f, "{why}"),
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
            Self::Full => "the task set is full",
            Self::ZeroPeriod => "a period of zero is not a period",
            Self::ExecutionExceedsDeadline => {
                "execution time exceeds the deadline; no scheduling fixes that"
            }
        })
    }
}

/// A fixed-priority task set, highest priority first.
///
/// Priority is position: index 0 preempts everything, index 1 preempts
/// everything below it. Rate-monotonic ordering is the caller's job, because
/// sorting silently would hide a caller's mistaken assumption about which task
/// wins. [`TaskSet::optimal_priority_order`] will search for an ordering that
/// works, and it says so rather than applying one behind your back.
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

    /// The task at a priority position, if there is one.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Task> {
        self.tasks.get(index).and_then(|t| t.as_ref())
    }

    /// Every admitted task, highest priority first.
    pub fn iter(&self) -> impl Iterator<Item = &Task> {
        self.tasks.iter().take(self.len).flatten()
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
    #[must_use]
    pub fn utilisation_ppm(&self) -> Option<u64> {
        self.utilisation_through(self.len.saturating_sub(1))
    }

    /// Utilisation through a priority level, inclusive, in parts per million.
    ///
    /// This is the quantity that decides whether the recurrence converges at
    /// that level. Integer division truncates, so the figure is a floor: a set
    /// fractionally above one can report exactly [`FULL_UTILISATION_PPM`]. The
    /// iteration cap catches those, which is why the cap exists at all.
    #[must_use]
    pub fn utilisation_through(&self, index: usize) -> Option<u64> {
        // `index + 1` unchecked was a panic in debug and, worse, a wrap in
        // release: `utilisation_through(usize::MAX)` returned `Some(0)` for a
        // set that was plainly busy — a wrong answer in the flattering
        // direction, which is the one direction this crate exists to refuse.
        //
        // Clamped, not saturated. A saturating add would read the same here,
        // and `audit.sh` would be right to object: saturation on a quantity
        // yields a plausible wrong number instead of a refusal, and that gate
        // cannot tell that this one is an array index rather than a time.
        // Clamping to the array length says what is meant — a level deeper
        // than the set is the whole set — without borrowing a habit that is
        // wrong everywhere else in this file.
        let depth = if index >= MAX_TASKS {
            MAX_TASKS
        } else {
            index + 1
        };
        // A floor, and deliberately so. Each summand is already floored, so
        // the total under-reports the true utilisation by under one part per
        // million per task. That direction is the safe one *here*: this value
        // gates convergence, and under-reporting means the gate declines to
        // refuse early and lets the busy-period solve decide instead. Over-
        // reporting would refuse a set that converges, which is the direction
        // that loses answers.
        //
        // It is not the safe direction everywhere, and `passes_utilisation_bound`
        // is where that mattered. See the note there.
        let mut total: u64 = 0;
        for t in self.tasks.iter().take(depth).flatten() {
            total = total.checked_add(t.utilisation_ppm()?)?;
        }
        Some(total)
    }

    /// The worst-case response time of the task at `index`.
    ///
    /// Solves the jitter-extended recurrence over the level-*i* busy period —
    /// the stretch during which the processor is continuously occupied by this
    /// priority or higher:
    ///
    /// ```text
    /// L    = B + Σ ⌈(L + Jⱼ)/Tⱼ⌉ · Cⱼ                 this level and above
    /// w(q) = (q+1)·C + B + Σ ⌈(w(q) + Jⱼ)/Tⱼ⌉ · Cⱼ    every j above
    /// R(q) = w(q) − q·T + J
    /// R    = max R(q)   over q = 0 … ⌈(L + J)/T⌉ − 1
    /// ```
    ///
    /// The busy-period form is Lehoczky's (RTSS 1990); the jitter terms are
    /// Tindell, Burns and Wellings (Real-Time Systems, 1994).
    /// Joseph and Pandya's 1986 form is the `q = 0` line of this without `J`,
    /// and it is sound exactly while the answer stays inside the task's own
    /// period. Past that the task's next job is released before the current one
    /// finishes, and the single-job form omits that self-interference. Until
    /// 2.0.0 this computed `q = 0` alone while accepting deadlines beyond the
    /// period, and under-reported on every set that reached past one.
    ///
    /// The busy-period length is solved first rather than discovered by growing
    /// `q`, so a set that cannot close one is refused after a single bounded
    /// solve and the job count is known before any per-job work begins. A count
    /// above [`BUSY_PERIOD_CAP`] is refused rather than enumerated.
    ///
    /// Convergence is decided before iterating, from the utilisation through
    /// this priority level. When the recurrence does converge, the search runs
    /// to the fixed point even if it passes the deadline, so that
    /// [`AnalysisFailure::ExceedsDeadline`] can report by how much. Earlier versions
    /// stopped at the deadline and could not.
    ///
    /// Every add, multiply and subtract is checked. An overflow returns
    /// [`AnalysisFailure::Overflow`] rather than wrapping, because a wrapped sum
    /// turns an unschedulable set into a schedulable-looking one, and that is
    /// the one direction an arithmetic error must never go.
    #[must_use]
    pub fn response_of(&self, index: usize) -> Response {
        let task = match self.tasks.get(index) {
            Some(Some(t)) => *t,
            _ => return Response::Refused(AnalysisFailure::NoSuchTask),
        };

        match self.utilisation_through(index) {
            None => return Response::Refused(AnalysisFailure::Overflow),
            Some(u) if u > FULL_UTILISATION_PPM => {
                return Response::Refused(AnalysisFailure::NonConvergent)
            }
            Some(_) => {}
        }

        // A saturated level with any jitter or blocking in it has no fixed
        // point, and that is provable rather than observed:
        //
        //     f(L) = B + Σ ⌈(L + Jⱼ)/Tⱼ⌉·Cⱼ  ≥  B + L·U + Σ Jⱼ·Cⱼ/Tⱼ
        //
        // so at U = 1, f(L) ≥ L + B + Σ Jⱼ·Cⱼ/Tⱼ, which exceeds L for every L
        // as soon as one of those terms is non-zero. The recurrence below
        // would climb for ten thousand iterations and then report
        // `IterationLimit`, which says *this implementation stopped*. The
        // truth is `NonConvergent`: the mathematics has no answer. Two
        // different statements, and the enum exists to keep them apart.
        //
        // At U = 1 with no jitter and no blocking a fixed point does exist —
        // L settles on a common multiple of the periods — so the condition is
        // saturation *and* a disturbance, never saturation alone.
        if self.level_is_saturated(index) && self.level_has_disturbance(index) {
            return Response::Refused(AnalysisFailure::NonConvergent);
        }

        // Length of the level-i busy period: the stretch during which the
        // processor is continuously occupied by this priority or higher.
        //
        //     L = B + Σ ⌈(L + Jⱼ)/Tⱼ⌉ · Cⱼ     for every j at this level or above
        //
        // Solving this first, rather than growing the job count one at a time,
        // is what keeps the cost of refusing an over-utilised set to a single
        // bounded solve. A set that cannot close its busy period is refused
        // here, before any per-job work is attempted.
        let mut busy = task.blocking_us;
        for t in self.tasks.iter().take(index + 1).flatten() {
            busy = match busy.checked_add(t.wcet_us) {
                Some(x) => x,
                None => return Response::Refused(AnalysisFailure::Overflow),
            };
        }
        let mut busy_settled = false;
        for _ in 0..ITERATION_CAP {
            let mut next = task.blocking_us;
            for t in self.tasks.iter().take(index + 1).flatten() {
                let window = match busy.checked_add(t.jitter_us) {
                    Some(x) => x,
                    None => return Response::Refused(AnalysisFailure::Overflow),
                };
                let jobs = window / t.period_us + u64::from(window % t.period_us != 0);
                let demand = match jobs.checked_mul(t.wcet_us) {
                    Some(x) => x,
                    None => return Response::Refused(AnalysisFailure::Overflow),
                };
                next = match next.checked_add(demand) {
                    Some(x) => x,
                    None => return Response::Refused(AnalysisFailure::Overflow),
                };
            }
            if next == busy {
                busy_settled = true;
                break;
            }
            busy = next;
        }
        if !busy_settled {
            // The busy-period recurrence ran out of iterations. Whether a fixed
            // point exists past the cap is not known here, and saying
            // NonConvergent would be asserting that it does not.
            return Response::Refused(AnalysisFailure::IterationLimit);
        }

        // How many jobs of this task the busy period contains. Derived, not
        // capped: the analysis knows the count before it starts enumerating.
        let released = match busy.checked_add(task.jitter_us) {
            Some(x) => x,
            None => return Response::Refused(AnalysisFailure::Overflow),
        };
        let jobs_in_window = released / task.period_us + u64::from(released % task.period_us != 0);
        let jobs_in_window = jobs_in_window.max(1);

        // A busy period holding more jobs of one task than this is a set no
        // reader could check by hand, and the point of the sixteen-task limit
        // is that every answer here stays checkable. Refused rather than
        // enumerated.
        if jobs_in_window > BUSY_PERIOD_CAP {
            // The busy period closed. Its length is known, the job count is
            // known, and both are finite. This is a refusal to enumerate, not
            // an absence of a bound.
            return Response::Refused(AnalysisFailure::BusyPeriodLimit);
        }

        // Job q of this task inside that busy period, counting from zero.
        // While the busy period is still running, job q + 1 is released before
        // job q finishes, and the work already queued is charged to it.
        let mut worst: u64 = 0;

        for q in 0..jobs_in_window {
            // (q+1)·C + B: every job released so far in this busy period, plus
            // the one blocking term the priority-ceiling protocol allows.
            let base = match q
                .checked_add(1)
                .and_then(|n| n.checked_mul(task.wcet_us))
                .and_then(|x| x.checked_add(task.blocking_us))
            {
                Some(b) => b,
                None => return Response::Refused(AnalysisFailure::Overflow),
            };
            let mut w = base;
            let mut settled = false;

            for _ in 0..ITERATION_CAP {
                let mut next = base;
                for higher in self.tasks.iter().take(index).flatten() {
                    let window = match w.checked_add(higher.jitter_us) {
                        Some(x) => x,
                        None => return Response::Refused(AnalysisFailure::Overflow),
                    };
                    // ⌈window / T⌉ without floating point and without the overflow
                    // the usual (a + b - 1) / b trick invites.
                    let jobs =
                        window / higher.period_us + u64::from(window % higher.period_us != 0);
                    let interference = match jobs.checked_mul(higher.wcet_us) {
                        Some(x) => x,
                        None => return Response::Refused(AnalysisFailure::Overflow),
                    };
                    next = match next.checked_add(interference) {
                        Some(x) => x,
                        None => return Response::Refused(AnalysisFailure::Overflow),
                    };
                }

                if next == w {
                    settled = true;
                    break;
                }
                w = next;
            }

            if !settled {
                // Same reasoning as the busy-period solve above: the cap was
                // reached, which is a statement about this implementation.
                return Response::Refused(AnalysisFailure::IterationLimit);
            }

            // R(q) = w(q) + J − q·T. The completion is measured from this
            // job's own release, not from the start of the busy period.
            //
            // The jitter is grouped in before the shift is taken out, and the
            // order is not cosmetic. Job q is released at q·T − J from the
            // start of the busy period, so w(q) − q·T is negative whenever the
            // job completes before that nominal offset, which a jittered task
            // does routinely. `u64` has no negative, and taking the shift out
            // first made `checked_sub` refuse a set whose answer is an ordinary
            // positive number. Grouped this way every intermediate stays in
            // range, because w(q) + J ≥ q·T holds for the same reason: a job
            // cannot complete before it is released.
            let shift = match q.checked_mul(task.period_us) {
                Some(x) => x,
                None => return Response::Refused(AnalysisFailure::Overflow),
            };
            let r = match w
                .checked_add(task.jitter_us)
                .and_then(|x| x.checked_sub(shift))
            {
                Some(x) => x,
                None => return Response::Refused(AnalysisFailure::Overflow),
            };
            if r > worst {
                worst = r;
            }
        }

        if worst > task.deadline_us {
            Response::Refused(AnalysisFailure::ExceedsDeadline(worst))
        } else {
            Response::Bounded(worst)
        }
    }

    /// Whether every task meets its deadline.
    #[must_use]
    pub fn is_schedulable(&self) -> bool {
        for i in 0..self.len {
            if !self.response_of(i).is_bounded() {
                return false;
            }
        }
        true
    }

    /// The index of the first task that misses, if any misses.
    #[must_use]
    pub fn first_failure(&self) -> Option<usize> {
        (0..self.len).find(|&i| !self.response_of(i).is_bounded())
    }

    /// Time between a task's response and its deadline, when it meets one.
    ///
    /// `None` when there is no bound at or below the deadline, because a
    /// negative slack reported as a small positive number is the shape of
    /// error this crate exists to refuse.
    #[must_use]
    pub fn slack_of(&self, index: usize) -> Option<u64> {
        let task = self.get(index)?;
        let r = self.response_of(index).bound()?;
        Some(task.deadline_us - r)
    }

    /// **Provable, not maximal, and the name says which.** The search asks
    /// `is_schedulable` at each step, and that answers false both for a set
    /// that misses a deadline and for one the analysis declined to examine. A
    /// candidate refused for [`AnalysisFailure::BusyPeriodLimit`] or
    /// [`AnalysisFailure::IterationLimit`] stops the search as surely as a
    /// missed deadline would, so the figure returned is the largest increase
    /// this analysis could *establish*, which may be smaller than the largest
    /// the system could actually absorb.
    ///
    /// Through 3.0.9 this was called `max_provable_wcet_increase`, which claimed the
    /// second thing while computing the first.
    ///
    /// How much execution time this task could gain before the set stops
    /// being schedulable.
    ///
    /// Binary search over the whole set, not over this task alone: raising one
    /// execution time can push a *lower*-priority task past its deadline, and
    /// an answer that only checked the task being changed would be wrong in
    /// the flattering direction.
    ///
    /// `None` if the set does not currently hold together, or if the index
    /// addresses no task.
    #[must_use]
    pub fn max_provable_wcet_increase(&self, index: usize) -> Option<u64> {
        let task = *self.get(index)?;
        if !self.is_schedulable() {
            return None;
        }
        let feasible = |extra: u64| -> bool {
            let mut trial = self.clone();
            match task.wcet_us.checked_add(extra) {
                Some(c) => {
                    trial.tasks[index] = Some(Task { wcet_us: c, ..task });
                    c <= task.deadline_us && trial.is_schedulable()
                }
                None => false,
            }
        };
        // The deadline is a hard ceiling on any single execution time, so the
        // search space is bounded without probing for it.
        let mut lo = 0u64;
        let mut hi = task.deadline_us - task.wcet_us;
        while lo < hi {
            let mid = lo + (hi - lo).div_ceil(2);
            if feasible(mid) {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        Some(lo)
    }

    /// The Liu and Layland sufficient bound for rate-monotonic priorities,
    /// in parts per million: `n · (2^(1/n) − 1)`.
    ///
    /// A pre-check and nothing more. A set under this bound is schedulable
    /// under rate-monotonic ordering; a set over it may still be, and
    /// [`TaskSet::response_of`] is what decides. The values are a table rather
    /// than a computation because the expression needs a root, and a root
    /// needs a float.
    #[must_use]
    pub fn liu_layland_bound_ppm(n: usize) -> u64 {
        const BOUND: [u64; 17] = [
            1_000_000, 1_000_000, 828_427, 779_763, 756_828, 743_491, 734_772, 728_626, 724_061,
            720_537, 717_734, 715_451, 713_557, 711_958, 710_592, 709_411, 708_380,
        ];
        BOUND.get(n).copied().unwrap_or(693_147)
    }

    /// Whether utilisation at this level reaches or exceeds one, decided
    /// exactly.
    ///
    /// `utilisation_through` floors, which is the right direction for the
    /// convergence gate and the wrong one for this question: a level whose
    /// true utilisation is 1.0000004 floors to 1.0, and the difference decides
    /// whether a fixed point can exist at all.
    fn level_is_saturated(&self, index: usize) -> bool {
        fn gcd(mut a: u128, mut b: u128) -> u128 {
            while b != 0 {
                let t = a % b;
                a = b;
                b = t;
            }
            a
        }
        let depth = if index >= MAX_TASKS {
            MAX_TASKS
        } else {
            index + 1
        };
        let mut num: u128 = 0;
        let mut den: u128 = 1;
        for t in self.tasks.iter().take(depth).flatten() {
            let c = u128::from(t.wcet_us);
            let tp = u128::from(t.period_us);
            let g = gcd(den, tp);
            let (d, n) = match (den.checked_mul(tp / g), num.checked_mul(tp / g)) {
                (Some(d), Some(n)) => (d, n),
                // Undecidable here. Returning false hands the question to the
                // recurrence, which refuses on its own terms.
                _ => return false,
            };
            let add = match c.checked_mul(d / tp) {
                Some(x) => x,
                None => return false,
            };
            num = match n.checked_add(add) {
                Some(x) => x,
                None => return false,
            };
            den = d;
            let g2 = gcd(num, den);
            if g2 > 1 {
                num /= g2;
                den /= g2;
            }
        }
        den != 0 && num >= den
    }

    /// Whether anything at this level pushes the busy period past its own
    /// demand: a blocking term on the task under analysis, or jitter on any
    /// task that actually executes.
    fn level_has_disturbance(&self, index: usize) -> bool {
        let depth = if index >= MAX_TASKS {
            MAX_TASKS
        } else {
            index + 1
        };
        if let Some(Some(t)) = self.tasks.get(index) {
            if t.blocking_us > 0 {
                return true;
            }
        }
        self.tasks
            .iter()
            .take(depth)
            .flatten()
            .any(|t| t.jitter_us > 0 && t.wcet_us > 0)
    }

    /// Whether the set passes the Liu and Layland pre-check.
    ///
    /// True means schedulable without further analysis. False means nothing at
    /// all, and is why the bound is a pre-check rather than a test.
    ///
    /// The theorem holds only for its own preconditions, so this checks them
    /// rather than assuming the caller did:
    ///
    /// * every deadline equals its period, and
    /// * priorities are rate-monotonic — shorter period, higher priority.
    ///
    /// A set breaking either gets `false`. Until 2.0.0 neither was checked, and
    /// a rate-monotonic set at a fifth of the bound with one constrained
    /// deadline was told it was schedulable while missing by five microseconds.
    /// The utilisation bound saying *safe* about a set that misses is the exact
    /// failure this crate was written about.
    #[must_use]
    pub fn passes_utilisation_bound(&self) -> bool {
        let mut previous: Option<u64> = None;
        for t in self.tasks.iter().take(self.len).flatten() {
            if t.deadline_us != t.period_us {
                return false;
            }
            if t.blocking_us != 0 || t.jitter_us != 0 {
                return false;
            }
            if let Some(p) = previous {
                if t.period_us < p {
                    return false;
                }
            }
            previous = Some(t.period_us);
        }
        // Compared exactly, as a ratio, because this function asserts a
        // sufficient condition and `utilisation_ppm` is a floor.
        //
        // Through 3.0.9 this read `utilisation_ppm() <= bound`. The summands
        // are floored per task, so a set could sit above the bound and report
        // as sitting on it: C=2/T=7 with C=108/T=199 has a true utilisation of
        // 828427.85 ppm against a two-task bound of 828427, and the floored
        // sum reported 828427 exactly. This returned true — *schedulable
        // without further analysis* — for a set that does not pass Liu and
        // Layland. A sufficient condition answered from a number rounded in
        // the flattering direction is the one failure this crate is built not
        // to have.
        //
        // Σ(Cⱼ/Tⱼ) ≤ B/10⁶ is decided by accumulating the sum as a reduced
        // fraction. The denominator is kept small by dividing out the common
        // factor at every step; when it would still overflow, this returns
        // false, which its own contract already defines as meaning nothing.
        fn gcd(mut a: u128, mut b: u128) -> u128 {
            while b != 0 {
                let t = a % b;
                a = b;
                b = t;
            }
            a
        }
        let mut num: u128 = 0;
        let mut den: u128 = 1;
        for t in self.tasks.iter().take(self.len).flatten() {
            let c = u128::from(t.wcet_us);
            let tp = u128::from(t.period_us);
            let g = gcd(den, tp);
            let (lhs, rhs) = match (den.checked_mul(tp / g), num.checked_mul(tp / g)) {
                (Some(d), Some(n)) => (d, n),
                _ => return false,
            };
            let add = match c.checked_mul(lhs / tp) {
                Some(x) => x,
                None => return false,
            };
            num = match rhs.checked_add(add) {
                Some(x) => x,
                None => return false,
            };
            den = lhs;
            let g2 = gcd(num, den);
            if g2 > 1 {
                num /= g2;
                den /= g2;
            }
        }
        if den == 0 {
            return false;
        }
        let bound = u128::from(Self::liu_layland_bound_ppm(self.len));
        match (
            num.checked_mul(u128::from(FULL_UTILISATION_PPM)),
            bound.checked_mul(den),
        ) {
            (Some(lhs), Some(rhs)) => lhs <= rhs,
            _ => false,
        }
    }

    /// Audsley's optimal priority assignment.
    ///
    /// Returns the original indices in priority order, highest first.
    ///
    /// Optimal in the sense Audsley proved, with one qualification this crate
    /// has to make and a textbook does not. The proof assumes an exact
    /// schedulability test. This analysis refuses some inputs rather than
    /// answering them, and a candidate that is refused has not been shown to
    /// miss its deadline — only left unexamined. So the guarantee here is:
    /// **if any ordering works and every candidate along the way could be
    /// analysed, this finds one.** When a level runs out of candidates with a
    /// refusal among them, the result is [`PriorityAssignment::Inconclusive`]
    /// rather than a claim that no ordering exists.
    ///
    /// The algorithm fills the lowest priority first. At each level it looks
    /// for a task that meets its deadline with every still-unassigned task
    /// above it, which is the worst case that task can face at that level.
    /// Finding one fixes it there and never revisits the choice.
    ///
    /// Ordering is returned rather than applied. A set that silently reordered
    /// itself would hide the assumption the caller came in with.
    #[must_use]
    pub fn optimal_priority_order(&self) -> PriorityAssignment {
        let n = self.len;
        let mut assigned = [false; MAX_TASKS];
        let mut order = [0usize; MAX_TASKS];

        for level in (0..n).rev() {
            let mut placed = false;
            // The first refusal seen while looking at this level. A level that
            // ends with no placement and one of these in hand is inconclusive,
            // not impossible.
            let mut refused: Option<AnalysisFailure> = None;
            for candidate in 0..n {
                if assigned[candidate] {
                    continue;
                }
                // Everything unassigned other than the candidate sits above it.
                let mut trial = TaskSet::new();
                for (other, other_done) in assigned.iter().enumerate().take(n) {
                    if other != candidate && !*other_done {
                        if let Some(Some(t)) = self.tasks.get(other) {
                            if trial.push(*t).is_err() {
                                return PriorityAssignment::Inconclusive(AnalysisFailure::Overflow);
                            }
                        }
                    }
                }
                if let Some(Some(t)) = self.tasks.get(candidate) {
                    if trial.push(*t).is_err() {
                        return PriorityAssignment::Inconclusive(AnalysisFailure::Overflow);
                    }
                } else {
                    return PriorityAssignment::Inconclusive(AnalysisFailure::NoSuchTask);
                }
                let last = trial.len() - 1;
                match trial.response_of(last) {
                    Response::Bounded(_) => {
                        assigned[candidate] = true;
                        order[level] = candidate;
                        placed = true;
                        break;
                    }
                    // A determinate refusal settles this candidate at this
                    // level: it misses, or it has no bound at all. Either way
                    // the search has learned something and moves on.
                    Response::Refused(why) if why.is_determinate() => {}
                    // An indeterminate one settles nothing. A level that runs
                    // out of candidates with one of these among them has not
                    // been shown impossible, only left undecided.
                    Response::Refused(why) => {
                        if refused.is_none() {
                            refused = Some(why);
                        }
                    }
                }
            }
            if !placed {
                return match refused {
                    Some(why) => PriorityAssignment::Inconclusive(why),
                    None => PriorityAssignment::NoOrdering,
                };
            }
        }
        PriorityAssignment::Found(order)
    }
}

// The harnesses live outside `src/` so that an ordinary build never walks
// them, and are declared here so that `cargo kani` finds them from a clean
// clone. Until 2.0.0 this declaration did not exist: `kani/response_bounds.rs`
// shipped in the published archive and was dead code, and the README's
// instruction to run `cargo kani` did nothing.
//
// The `cfg(kani)` gate is the idiom Kani documents, and it has a cost worth
// naming: the module is invisible to rustc, rust-analyzer, and every tool
// built on them, so no code graph or coverage tool can see which production
// code the proofs reach. Making them visible would mean a stub dependency,
// and this crate has none. The gate is the lesser of the two.
#[cfg(kani)]
extern crate self as dy_wcet;

#[cfg(kani)]
#[path = "../kani/response_bounds.rs"]
mod proofs;

#[cfg(test)]
mod tests {
    use super::*;

    fn t(w: u64, p: u64) -> Task {
        Task::new(w, p)
    }

    #[test]
    fn a_single_task_responds_in_its_own_execution_time() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        assert_eq!(s.response_of(0), Response::Bounded(100));
    }

    #[test]
    fn a_lower_priority_task_pays_for_every_preemption() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        s.push(t(200, 1000)).unwrap();
        // R = 200 → ⌈200/400⌉ = 1 → 300; R = 300 → still 1 → 300, fixed point.
        assert_eq!(s.response_of(1), Response::Bounded(300));
    }

    #[test]
    fn blocking_is_added_once_and_not_per_preemption() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        s.push(Task::new(200, 1000).blocking(20)).unwrap();
        assert_eq!(s.response_of(1), Response::Bounded(320));
    }

    #[test]
    fn jitter_widens_the_window_and_adds_to_the_answer() {
        let mut s = TaskSet::new();
        s.push(Task::new(100, 400).jitter(50)).unwrap();
        s.push(Task::new(200, 1000)).unwrap();
        // w = 200 → ⌈(200+50)/400⌉ = 1 → 300
        // w = 300 → ⌈(300+50)/400⌉ = 1 → 300, fixed point. R = 300 + 0.
        assert_eq!(s.response_of(1), Response::Bounded(300));
    }

    #[test]
    fn a_tasks_own_jitter_lands_in_its_own_response() {
        let mut s = TaskSet::new();
        s.push(Task::new(100, 400)).unwrap();
        s.push(Task::new(200, 1000).jitter(40)).unwrap();
        // w settles at 300 as above; R = w + J = 340.
        assert_eq!(s.response_of(1), Response::Bounded(340));
    }

    #[test]
    fn zero_jitter_reduces_to_joseph_and_pandya() {
        let mut a = TaskSet::new();
        a.push(t(100, 400)).unwrap();
        a.push(t(200, 1000)).unwrap();
        let mut b = TaskSet::new();
        b.push(Task::new(100, 400).jitter(0)).unwrap();
        b.push(Task::new(200, 1000).jitter(0)).unwrap();
        assert_eq!(a.response_of(1), b.response_of(1));
    }

    #[test]
    fn an_overloaded_set_does_not_converge_and_says_so() {
        let mut s = TaskSet::new();
        s.push(t(300, 400)).unwrap();
        s.push(t(300, 400)).unwrap();
        assert_eq!(
            s.response_of(1),
            Response::Refused(AnalysisFailure::NonConvergent)
        );
    }

    #[test]
    fn a_bound_above_the_deadline_is_reported_with_its_value() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        s.push(Task::new(200, 1000).deadline(250)).unwrap();
        // Converges at 300, which is past 250. The number is the point.
        match s.response_of(1) {
            Response::Refused(AnalysisFailure::ExceedsDeadline(r)) => assert_eq!(r, 300),
            other => panic!("expected ExceedsDeadline, got {other:?}"),
        }
    }

    #[test]
    fn every_unbounded_variant_fails_every_deadline() {
        for why in [
            AnalysisFailure::NonConvergent,
            AnalysisFailure::ExceedsDeadline(1),
            AnalysisFailure::Overflow,
            AnalysisFailure::NoSuchTask,
        ] {
            assert!(!Response::Refused(why).meets(u64::MAX));
            assert_eq!(Response::Refused(why).bound(), None);
        }
    }

    #[test]
    fn an_index_past_the_end_is_named_rather_than_guessed() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        assert_eq!(
            s.response_of(9),
            Response::Refused(AnalysisFailure::NoSuchTask)
        );
    }

    #[test]
    fn overflow_is_reported_and_never_wrapped() {
        let mut s = TaskSet::new();
        s.push(Task::new(u64::MAX / 2, u64::MAX).deadline(u64::MAX))
            .unwrap();
        s.push(Task::new(u64::MAX / 2, u64::MAX).deadline(u64::MAX))
            .unwrap();
        assert!(matches!(s.response_of(1), Response::Refused(_)));
        assert_eq!(s.response_of(1).bound(), None);
    }

    #[test]
    fn a_zero_period_is_refused_at_admission() {
        let mut s = TaskSet::new();
        assert_eq!(s.push(t(10, 0)), Err(Rejected::ZeroPeriod));
    }

    #[test]
    fn execution_longer_than_its_deadline_is_refused() {
        let mut s = TaskSet::new();
        assert_eq!(
            s.push(Task::new(500, 1000).deadline(400)),
            Err(Rejected::ExecutionExceedsDeadline)
        );
    }

    #[test]
    fn the_task_limit_is_enforced_rather_than_overrun() {
        let mut s = TaskSet::new();
        for _ in 0..MAX_TASKS {
            s.push(t(1, 1_000_000)).unwrap();
        }
        assert_eq!(s.push(t(1, 1_000_000)), Err(Rejected::Full));
    }

    #[test]
    fn utilisation_is_parts_per_million_and_never_a_float() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        s.push(t(200, 1000)).unwrap();
        assert_eq!(s.utilisation_ppm(), Some(450_000));
    }

    #[test]
    fn the_same_set_gives_the_same_bits_every_time() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        s.push(Task::new(200, 1000).blocking(20).jitter(7)).unwrap();
        let first = s.response_of(1);
        for _ in 0..1000 {
            assert_eq!(s.response_of(1), first);
        }
    }

    #[test]
    fn a_set_at_exactly_its_deadline_is_schedulable() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        s.push(Task::new(200, 1000).deadline(300)).unwrap();
        assert_eq!(s.response_of(1), Response::Bounded(300));
    }

    #[test]
    fn one_microsecond_past_the_deadline_is_not() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        s.push(Task::new(200, 1000).deadline(299)).unwrap();
        assert!(!s.response_of(1).is_bounded());
    }

    #[test]
    fn slack_is_none_when_the_task_misses() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        s.push(Task::new(200, 1000).deadline(299)).unwrap();
        assert_eq!(s.slack_of(1), None);
        assert_eq!(s.first_failure(), Some(1));
    }

    #[test]
    fn slack_is_the_distance_to_the_deadline() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        s.push(t(200, 1000)).unwrap();
        assert_eq!(s.slack_of(1), Some(700));
    }

    #[test]
    fn sensitivity_finds_the_last_execution_time_that_still_fits() {
        let mut s = TaskSet::new();
        s.push(t(100, 400)).unwrap();
        s.push(Task::new(200, 1000).deadline(400)).unwrap();
        let extra = s.max_provable_wcet_increase(1).unwrap();
        let mut grown = s.clone();
        grown.tasks[1] = Some(Task::new(200 + extra, 1000).deadline(400));
        assert!(grown.is_schedulable());
        let mut past = s.clone();
        past.tasks[1] = Some(Task::new(200 + extra + 1, 1000).deadline(400));
        assert!(!past.is_schedulable());
    }

    #[test]
    fn the_utilisation_bound_is_a_precheck_and_says_so() {
        assert_eq!(TaskSet::liu_layland_bound_ppm(1), 1_000_000);
        assert_eq!(TaskSet::liu_layland_bound_ppm(2), 828_427);
        assert!(TaskSet::liu_layland_bound_ppm(16) > 693_147);
        assert_eq!(TaskSet::liu_layland_bound_ppm(99), 693_147);
    }

    #[test]
    fn audsley_finds_an_ordering_the_caller_had_backwards() {
        // Given in the wrong order: the short-deadline task sits at the bottom.
        let mut s = TaskSet::new();
        s.push(Task::new(200, 1000).named("long")).unwrap();
        s.push(Task::new(100, 400).deadline(150).named("tight"))
            .unwrap();
        assert!(!s.is_schedulable());
        let order = s
            .optimal_priority_order()
            .found()
            .expect("an ordering exists");
        assert_eq!(order[0], 1, "the tight task belongs at the top");
        assert_eq!(order[1], 0);
    }

    #[test]
    fn audsley_proves_impossible_when_no_ordering_works() {
        let mut s = TaskSet::new();
        s.push(t(300, 400)).unwrap();
        s.push(t(300, 400)).unwrap();
        assert_eq!(s.optimal_priority_order(), PriorityAssignment::NoOrdering);
    }

    #[test]
    fn rejections_and_responses_describe_themselves() {
        assert!(Rejected::Full.to_string().contains("full"));
        assert!(Response::Bounded(42).to_string().contains("42"));
        assert!(Response::Refused(AnalysisFailure::NonConvergent)
            .to_string()
            .contains("utilisation"));
        assert!(Response::Refused(AnalysisFailure::ExceedsDeadline(9))
            .to_string()
            .contains('9'));
    }

    #[test]
    fn a_task_builder_leaves_nothing_implicit() {
        let t = Task::new(200, 1000)
            .deadline(800)
            .blocking(20)
            .jitter(15)
            .named("control");
        assert_eq!(t.wcet_us, 200);
        assert_eq!(t.period_us, 1000);
        assert_eq!(t.deadline_us, 800);
        assert_eq!(t.blocking_us, 20);
        assert_eq!(t.jitter_us, 15);
        assert_eq!(t.name, "control");
        assert_eq!(Task::new(1, 4).deadline_us, 4);
    }
}
