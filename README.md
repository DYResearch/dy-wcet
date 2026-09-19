<div align="center">

# dy-wcet

### Worst-case response time, in arithmetic that refuses rather than rounds.

[![CI](https://img.shields.io/github/actions/workflow/status/DYResearch/dy-wcet/ci.yml?branch=main&style=flat-square&label=CI&labelColor=0e141d)](https://github.com/DYResearch/dy-wcet/actions)
[![no_std](https://img.shields.io/badge/no__std-yes-3ecf8e?style=flat-square&labelColor=0e141d)](src/lib.rs)
[![unsafe](https://img.shields.io/badge/unsafe-forbidden-3ecf8e?style=flat-square&labelColor=0e141d)](src/lib.rs)
[![deps](https://img.shields.io/badge/dependencies-0-3ecf8e?style=flat-square&labelColor=0e141d)](Cargo.toml)
[![tests](https://img.shields.io/badge/tests-92-3ecf8e?style=flat-square&labelColor=0e141d)](#verify-it-yourself)
[![proofs](https://img.shields.io/badge/Kani%20harnesses-6%20(advisory%2C%20not%20verifying)-d98b3a?style=flat-square&labelColor=0e141d)](kani/)
[![Licence](https://img.shields.io/badge/Apache--2.0%20OR%20MIT-475569?style=flat-square&labelColor=0e141d)](#licence)

[Symbols](#symbols-and-words) · [Two tasks, one number](#two-tasks-one-number) · [Use](#use) · [Verify](#verify-it-yourself) · [Limits](#what-it-does-not-do) · [Case study](#case-study) · [Timing audit](#timing-audit) · [Bounty](#the-bounty)

</div>

---

Response-time analysis is a fixed point. A task's response time is its own
execution plus the interference from everything that can preempt it, and the
interference depends on the response time:

```text
w⁰   = C + B
wⁿ⁺¹ = C + B + Σ ⌈(wⁿ + Jⱼ) / Tⱼ⌉ · Cⱼ     for every j of higher priority
R    = w + J
```

Joseph and Pandya published the form without `J` in 1986. It fits on a napkin,
and the wrong answers are all quiet.

---

## Symbols and words

Everything on this page and in the API uses these, and nothing here uses a
symbol it has not defined. Every quantity is an integer number of
**microseconds**; there is no floating point anywhere in this crate.

| | Means | In the API |
|:--|:--|:--|
| **C** | Execution time — the worst case, which is an input you supply, not something this crate measures | `Task::new(c, t)` |
| **T** | Period, or minimum inter-arrival time for a sporadic task | `Task::new(c, t)` |
| **D** | Relative deadline. Defaults to `T`; may be shorter (constrained) or longer (arbitrary) | `.deadline(d)` |
| **J** | Release jitter — how late a release may be relative to its nominal instant | `.jitter(j)` |
| **B** | Blocking — lower-priority work this task cannot preempt, from a shared resource | `.blocking(b)` |
| **R** | Response time: release to completion. The number this crate computes | `Response::Bounded(r)` |
| **w** | The fixed point of the recurrence above, before jitter is added back | internal |
| **L** | Level-*i* busy period: the stretch during which the processor runs only work at priority *i* or above | internal |
| **U** | Utilisation, `C/T`, summed over a priority level | `utilisation_ppm()` |

| Acronym | Expanded | Why it appears |
|:--|:--|:--|
| **WCET** | Worst-Case Execution Time | The `C` above. The crate's name; also the input it trusts you for |
| **RTA** | Response-Time Analysis | The method: solve the recurrence, do not estimate |
| **FPPS** | Fixed-Priority Preemptive Scheduling | The scheduling model this analysis is valid under |
| **DM / RM** | Deadline-Monotonic / Rate-Monotonic | Priority orderings that are optimal under stated conditions |
| **ppm** | Parts Per Million | How utilisation is reported, because `0.75` and `0.7500001` are different numbers and a float will not keep them apart |
| **MSRV** | Minimum Supported Rust Version | Declared in `Cargo.toml`, checked in CI |
| **CBMC** | C Bounded Model Checker | What Kani runs underneath; it unwinds loops to a bound and asks a solver |
| **SAT / SMT** | Boolean satisfiability / Satisfiability Modulo Theories | The two kinds of solver CBMC can hand the problem to. Division over 64-bit integers is where the choice starts to matter |
| **BRS** | Bounded Refusal Semantics | This crate's rule: when it cannot compute a sound bound it returns a named refusal, never a number it does not believe |

---

## Two tasks, one number

Task A runs 100 µs every 400 µs at higher priority. Task B runs 200 µs every
1000 µs. What is B's worst-case response time?

<details>
<summary><b>Work it out, then open this.</b></summary>

<br>

The common answer is **400 µs**: B's period is 1000 µs, two activations of A
fit inside it, so 200 + 2×100.

The correct answer is **300 µs**.

```text
R = 200   ⌈200/400⌉ = 1 activation   →  200 + 100 = 300
R = 300   ⌈300/400⌉ = 1 activation   →  200 + 100 = 300   ← fixed point
```

The window that counts is the **response time**, not the period. B finishes at
300 µs, and only one activation of A occurs before then.

Here the mistake is conservative — the system looks worse than it is. Change
the deadline to 350 µs and it inverts: the wrong method reports a failure that
does not happen. Change the periods and it inverts the other way, reporting a
deadline met that is missed on hardware.

Both answers are plausible, neither is flagged, and a test written by whoever
wrote the bug passes.

</details>

---

## Three ways this goes wrong elsewhere

**In floating point.** A response time in `f64` has last bits that depend on
the compiler and the optimisation level; two implementations disagree in the
eighth decimal, one rounds a deadline the other misses, and no test pins
either. Everything here is `u64` microseconds. Same input, same bits, any
machine.

**By capping the loop.** The recurrence converges only up to full utilisation.
Above it the iteration climbs forever, and an implementation that stops after
*n* rounds and returns the last value returns something that looks like an
answer. This decides convergence from utilisation *before* iterating and
returns `AnalysisFailure::NonConvergent`, which fails every deadline comparison it is
put into.

**By wrapping.** Interference is a sum of ceilings of quotients, and it grows
fast. A wrapping add turns an unschedulable set into a schedulable one: the
single worst direction an arithmetic error can go. Every operation is checked;
overflow is reported as unschedulable.

---

## Use

```toml
[dependencies]
dy-wcet = "4.0"
```

```rust
use dy_wcet::{Task, TaskSet, Response};

let mut set = TaskSet::new();
set.push(Task::new(100, 400).named("sensor"))?;
set.push(Task::new(200, 1000).blocking(20).jitter(15).named("control"))?;

match set.response_of(1) {
    Response::Bounded(r)   => println!("{r} µs"),        // 335
    Response::Refused(w) => println!("no bound: {w}"),
}
```

Unset terms are zero rather than whatever was in your head. Priority is
position: index 0 preempts everything.

An unbounded answer says which of four things happened, and the one that
matters carries a number:

```rust
use dy_wcet::{Response, AnalysisFailure};

match set.response_of(1) {
    Response::Bounded(r) => println!("{r} µs, and it fits"),
    Response::Refused(AnalysisFailure::ExceedsDeadline(r)) => println!("{r} µs, and it does not"),
    Response::Refused(AnalysisFailure::NonConvergent)      => println!("over-utilised; no bound exists"),
    Response::Refused(AnalysisFailure::Overflow)           => println!("the arithmetic was refused"),
    Response::Refused(AnalysisFailure::NoSuchTask)         => println!("no task at that index"),
}
```

Rate-monotonic ordering is the caller's job, because sorting silently would
hide a mistaken assumption about which task wins. If you would rather be told
whether *any* ordering works, ask:

```rust
use dy_wcet::PriorityAssignment;

match set.optimal_priority_order() {
    PriorityAssignment::Found(order) => println!("this order meets every deadline: {order:?}"),
    PriorityAssignment::NoOrdering   => println!("no fixed-priority ordering of this set does"),
    PriorityAssignment::Inconclusive(why) => println!("the search could not decide: {why:?}"),
}
```

Audsley's assignment. Optimal in his exact sense, with the qualification this
crate has to make and a textbook does not: his proof assumes an exact
schedulability test, and this analysis refuses some inputs rather than
answering them. A refused candidate has not been shown to miss — only left
unexamined. So the guarantee is *if an ordering exists and every candidate
along the way could be analysed, this finds one*, and a level that runs out of
candidates with a refusal among them returns `Inconclusive` rather than
claiming none exists. It returns the ordering rather than applying it.

Two more questions the analysis can answer once it holds:

```rust
set.slack_of(1);            // Some(665) — time between the response and the deadline
set.max_provable_wcet_increase(1);   // Some(n)   — execution time this task could gain
```

The sensitivity search re-analyses the whole set, not the task being changed.
Raising one execution time can sink a lower-priority task, and an answer that
checked only the one you touched would be wrong in the flattering direction.

## Verify it yourself

```sh
git clone https://github.com/DYResearch/dy-wcet && cd dy-wcet
cargo test          # the analysis
./audit.sh          # every number this repository states, against its source
```

26 unit tests in `src/lib.rs` check the implementation does what its author
intended. 15 integration tests in [`tests/on_paper.rs`](tests/on_paper.rs)
check something harder: that what its author intended is what the analysis
says.
Every expected value there is derived in the comment above it, iteration by
iteration, so a reader who distrusts the code can settle it with a pencil.

Three further files exist because none of the above could catch the defect
fixed in 2.0.0. [`tests/oracle.rs`](tests/oracle.rs) computes the busy period a
second way, growing the job count instead of deriving it, and agrees on 153,365
generated sets. [`tests/differential.rs`](tests/differential.rs) keeps the
recurrence this release replaced and checks the new answer is never smaller.
[`tests/precheck.rs`](tests/precheck.rs) holds the utilisation bound to its own
preconditions. Every other artifact here compares the implementation to itself.

A fifth file, [`tests/boundaries.rs`](tests/boundaries.rs), holds the surface
around the recurrence: what the accessors report, what admission refuses, and
the two constants that bound the analysis. `bound` and `response_time` answer
different questions about the same converged number, and a caller that reads
the wrong one sizes a margin against a deadline it already missed.

8 property tests in [`tests/properties.rs`](tests/properties.rs) check what no fixed
case can, across roughly twenty-eight thousand generated task sets: that a
response never falls below the work it contains, that jitter never shortens
one, that Audsley's answer actually holds when applied, and that the
sensitivity figure is the last value that fits rather than one past it. The
generator is thirty lines of seeded linear congruence and adds no dependency,
because a dependency tree pulled in to produce pseudo-random `u64` would cost
this crate the one thing it advertises.

Six Kani harnesses in [`kani/`](kani/) are written to bound what the tests
sample. **They do not currently verify, and this crate does not claim any
proved property.** CI runs them on every push as an advisory job: it reports
what happened and does not gate the build.

What is wrong with them is worth stating precisely, because "the proofs are
red" and "the proofs are slow" are different facts. Two loops in
`response_of` — the fixed-point search and the per-job walk — carry constant
bounds of ten thousand and one thousand and twenty-four, the second nested in
the first, and CBMC must unwind a loop to its largest possible trip count
before it asserts anything past it. No `#[kani::unwind]` value reaches that.
Since 3.0.6 `cfg(kani)` reduces those caps, which closed the unwinding
assertions; what remains is solver time, and a harness ends on its budget
rather than on a failed check.

So the evidence behind this crate is the 85 tests, the exhaustive small-state
checks, and the 220 000-case differential campaign against an independent
scheduler. The harnesses are work in progress and are labelled that way in the
badge above, in the job name, and in the run summary.

Four of those cases are worth reading even if you never use this crate:

| Case | Why it is there |
|:--|:--|
| [`a_deadline_shorter_than_the_period_can_fail_at_low_utilisation`](tests/on_paper.rs) | Utilisation 0.775 and it misses. A utilisation test calls this safe |
| [`full_utilisation_is_schedulable_when_it_lands_exactly`](tests/on_paper.rs) | U = 1.0 and every deadline is met. Strict inequality reports failure |
| [`the_deadline_boundary_from_both_sides`](tests/on_paper.rs) | One microsecond changes both the sum and the activation count at once |
| [`a_deadline_beyond_the_period_is_permitted`](tests/on_paper.rs) | An implementation assuming `R ≤ T` is right on most sets |

## Checked against a second implementation

The AxonOS kernel carries its own response-time analysis, written months
earlier for a different reason. On a shared task set the two agree — and the
first attempt at that comparison is the useful part.

It used a set whose execution times summed to less than the shortest period.
Every ceiling in the recurrence was 1, the fixed point was the first value
tried, and the agreement was about addition rather than about analysis. The set
now used takes three iterations on its lowest-priority task, with the
activation count changing between them.

Full derivation: [`docs/CROSSCHECK.md`](docs/CROSSCHECK.md).

Both implementations are by the same author, so agreement is not
independence — a shared misreading of the recurrence agrees with itself
perfectly. That is why every figure there is derived rather than asserted.

Since 2.0.0 there is a second and better check, which does not share the
recurrence at all. [`tools/differential.py`](tools/differential.py) runs the
schedule the recurrence claims to describe: jobs released from the critical
instant, the processor taking the highest-priority ready one, completions
measured. Over 5,347 generated sets carrying jitter, blocking and deadlines
past the period, the simulation never exceeded the computed bound and equalled
it every time — sound, and on that sample exact rather than merely safe. It
runs in under half a second, so it is a gate on every push rather than an
occasional exercise.

Its first draft reported 83 unsound results. All 83 were a defect in the
simulation, which merged a task's queued jobs into one counter and recorded the
completion of the last as the completion of the first. The crate was right and
the check was wrong, which is the ordinary outcome and worth saying out loud.

---

## What it does not do

The model is stated so that a set relying on something outside it can be
recognised as outside it, rather than quietly analysed anyway.

| Limit | What that means |
|:--|:--|
| **It does not measure** | Execution times are inputs. If they came from a spreadsheet rather than an oscilloscope, this is arithmetic about a guess, and the crate cannot tell the difference |
| **No cache, pipeline, DMA or bus model** | Blocking is an input, not a derivation |
| **Priority-ceiling protocol assumed** | A task is blocked at most once. Without one, the blocking term is not a single number and this analysis does not apply |
| **One core** | No partitioned or global multiprocessor analysis. A set spread over cores needs a different recurrence |
| **A busy period of at most 1024 jobs** | Past that the analysis refuses rather than enumerating. A response spanning a thousand of a task's own periods is not an answer anybody checks |
| **Sixteen tasks maximum** | Not a theoretical limit — the point past which a fixed-priority set on one core stops being checkable by hand, and an analysis nobody can check by hand is an analysis nobody checks |

---

## Case study

**[Embassy #6528 — RP2350 lost-alarm timing analysis](case-studies/embassy-6528.md)**

An intermittent `embassy-time` failure on RP2350, traced through hardware alarm
arming, timer-queue liveness, and a response time with no upper bound. Written
end to end: the failure chain in the source; a clear line between what the
evidence supports and what stays hypothesis; next steps that each name what
would confirm or kill them.

It is here as the standard of delivery, not a sample of one. What arrives looks
like that.

## Timing audit

The arithmetic in this crate is one piece of a practice. Some timing problems
will not resolve: an intermittent failure that survives every fix, a suspected
race, a scheduler or liveness question, a WCET bound that has to hold up in a
review. I take them one at a time, in writing.

### $3,000 — one problem, traced end to end

Written delivery, within five working days. The Embassy analysis above is what
arrives.

**You provide** the problem, the relevant source or a minimal reproducer, and
whatever logs or traces you already have.

**You get** a markdown report: the behaviour chain traced through the source,
evidence separated from hypothesis, and concrete next steps or a proposed fix.

**Where it stops.** Source-level and reasoning-level analysis of one issue. No
hardware instrumentation, no running your build, no multi-issue reviews, no
ongoing support. If the problem turns out larger than it looked, I say so
before starting and quote the rest separately; the fixed price stays fixed.

Deeper audits, hardware-in-the-loop verification, multi-issue reviews and
retained consulting start at **$8,000**, after a short scoping exchange.

**[Full scope](AUDIT.md)** · [connect@axonos.org](mailto:connect@axonos.org)

## The bounty

I wrote the crate and I wrote its tests, which is the one problem I cannot
solve from inside: a test written by whoever wrote the bug asserts what the
implementation produces.

So there is a pool, in Dogecoin, for the first task set where this returns a
bound the recurrence does not support — derived, reproducible, on a set that
actually exercises the recurrence. No adjudicator: your derivation and the
crate's output go side by side, and one of them contains a step that does not
add up.

**[The rules, the conditions, and the address](BOUNTY.md)**

---

## A note on how this was written

While writing the paper tests, the expected value for a three-task set was
stated as 7 from memory. The arithmetic says 6: at R = 6 the middle task has
had exactly one activation, because `⌈6/6⌉ = 1` and a task released at time 6
does not interfere with a response completing at 6.

The recurrence is short enough that being confident about it is easy and being
right about it is not. That is the whole reason this exists, and the case is
now [`three_tasks_settling_at_six`](tests/on_paper.rs).

## Where the analysis comes from

None of the mathematics here is mine. The crate is an implementation, and the
people who worked it out should be named next to the parts they are responsible
for rather than gestured at in a bibliography.

| The part | Whose result it is |
|:--|:--|
| The recurrence itself, `R = C + Σ ⌈R/T⌉·C` | **Joseph & Pandya, 1986.** *Finding response times in a real-time system.* The Computer Journal 29(5), 390–395 |
| Deadlines past the period, and why one job is not enough | **Lehoczky, 1990.** *Fixed priority scheduling of periodic task sets with arbitrary deadlines.* RTSS'90, 201–209. The level-*i* busy period, and the result that every job released inside it must be examined — which is what 2.0.0 added and what the versions before it got wrong |
| Release jitter in the same recurrence | **Tindell, Burns & Wellings, 1994.** *An extendible approach for analyzing fixed priority hard real-time tasks.* Real-Time Systems 6(2), 133–151. The `⌈(w + J)/T⌉` form, and `R = w + J − qT` |
| Optimal priority assignment | **Audsley, 1991.** *Optimal priority assignment and feasibility of static priority tasks with arbitrary start times.* Technical Report YCS-164, University of York |
| One blocking term per job | **Sha, Rajkumar & Lehoczky, 1990.** *Priority inheritance protocols: an approach to real-time synchronization.* IEEE Transactions on Computers 39(9), 1175–1185. The reason `blocking_us` is a single number rather than a sum |

Lehoczky and Tindell were missing from this list until 2.0.0, which is the
release whose headline feature is their result. That was the wrong order.

What is mine is the implementation, the refusals, the integer arithmetic, and
the tests — including the ones that found my own errors.

---

## Licence

Apache-2.0 OR MIT, at your option —
[`LICENSE-APACHE`](LICENSE-APACHE) · [`LICENSE-MIT`](LICENSE-MIT).

---

<div align="center">

**DY Research** — [dyresearch.github.io](https://dyresearch.github.io)

Denis Yermakou · [connect@axonos.org](mailto:connect@axonos.org)

© 2026 Denis Yermakou

</div>
