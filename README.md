<div align="center">

# dy-wcet

### Worst-case response time for real-time systems,<br>in arithmetic that refuses rather than rounds.

[![CI](https://img.shields.io/github/actions/workflow/status/DYResearch/dy-wcet/ci.yml?branch=main&style=flat-square&label=CI&labelColor=0e141d)](https://github.com/DYResearch/dy-wcet/actions)
[![no_std](https://img.shields.io/badge/no__std-yes-3ecf8e?style=flat-square&labelColor=0e141d)](src/lib.rs)
[![unsafe](https://img.shields.io/badge/unsafe-forbidden-3ecf8e?style=flat-square&labelColor=0e141d)](src/lib.rs)
[![deps](https://img.shields.io/badge/dependencies-0-3ecf8e?style=flat-square&labelColor=0e141d)](Cargo.toml)
[![tests](https://img.shields.io/badge/tests-100-3ecf8e?style=flat-square&labelColor=0e141d)](#the-evidence)
[![proofs](https://img.shields.io/badge/Kani%20harnesses-6%20(advisory%2C%20not%20verifying)-d98b3a?style=flat-square&labelColor=0e141d)](#formal-verification)
[![Licence](https://img.shields.io/badge/Apache--2.0%20OR%20MIT-475569?style=flat-square&labelColor=0e141d)](#licence)

[The puzzle](#two-tasks-one-number) · [Quick start](#quick-start) · [Refusals](#six-ways-to-say-no) · [Evidence](#the-evidence) · [Limits](#what-it-does-not-do) · [Audit](#timing-audit)

</div>

---

## Two tasks, one number

Task A runs 100 µs every 400 µs at higher priority. Task B runs 200 µs every
1000 µs. What is B's worst-case response time?

<details>
<summary><b>Work it out, then open this.</b></summary>

<br>

The common answer is **400 µs**: two activations of A fit inside B's period, so
200 + 2 × 100.

The correct answer is **300 µs**.

```text
R = 200   ⌈200/400⌉ = 1 activation   →  200 + 100 = 300
R = 300   ⌈300/400⌉ = 1 activation   →  200 + 100 = 300   ← fixed point
```

The window that counts is the response time, not the period. B finishes at
300 µs, and only one activation of A falls before it.

Here the mistake is conservative. Change the deadline to 350 µs and it inverts:
the wrong method reports a failure that does not happen. Change the periods and
it inverts the other way, reporting a deadline met that is missed on hardware.
Both answers look plausible, neither is flagged, and a test written by whoever
wrote the bug passes.

</details>

This crate answers `300`. It is the recurrence below, solved exactly:

```text
w⁰   = C + B
wⁿ⁺¹ = C + B + Σ ⌈(wⁿ + Jⱼ) / Tⱼ⌉ · Cⱼ     for every j of higher priority
R    = w + J
```

A task's response time is its own work plus the interference from everything
that can preempt it, and the interference depends on the response time. So the
answer is a fixed point, and the arithmetic has to find it rather than guess
near it.

---

## Quick start

```toml
[dependencies]
dy-wcet = "4.1"
```

```rust
use dy_wcet::{Response, Task, TaskSet};

let mut set = TaskSet::new();
set.push(Task::new(100, 400).named("sensor"))?;
set.push(Task::new(200, 1000).blocking(20).jitter(15).named("control"))?;

match set.response_of(1) {
    Response::Bounded(r) => println!("{r} µs"),          // 335 µs
    Response::Refused(why) => println!("no bound: {why}"),
}
```

Priority is position: index 0 preempts everything. Terms you leave unset are
zero, not whatever was in your head. Every quantity is a whole number of
microseconds.

---

## Why it exists

Response-time analysis fits on a napkin, and its wrong answers are quiet. Three
of them are common enough to name.

**Floating point.** A response time in `f64` has last bits that depend on the
compiler and the optimisation level. Two implementations disagree in the eighth
decimal, one rounds a deadline the other misses, and no test pins either.
Everything here is `u64`. Same input, same bits, on any machine.

**A capped loop.** Above full utilisation the recurrence climbs forever. An
implementation that stops after *n* rounds and returns the last value returns
something that looks like an answer. This one decides convergence from
utilisation before it iterates, and says so when no fixed point can exist.

**Wrapping.** Interference is a sum of ceilings of quotients, and it grows
fast. A wrapping add turns an unschedulable set into a schedulable one — the
single worst direction an arithmetic error can go. Every operation here is
checked, and an overflow is a refusal.

---

## Six ways to say no

An analysis that returns a number when it cannot justify one is worse than one
that returns nothing, because the number gets used. So an unbounded answer
names which of six things happened, and the one a caller acts on carries the
figure it converged to.

| Refusal | What it means | What it prints |
|:--|:--|:--|
| `ExceedsDeadline(r)` | A bound exists and it misses. `r` is how far | converges at 412 us, past the deadline |
| `NonConvergent` | Proved impossible: utilisation leaves no fixed point | utilisation exceeds one; no fixed point exists |
| `IterationLimit` | The search reached its cap before settling | the recurrence did not settle within the iteration cap |
| `BusyPeriodLimit` | A bound exists, but over more jobs than it will enumerate | the busy period is bounded but holds more jobs than the cap admits |
| `Overflow` | The arithmetic would have wrapped, so it stopped | the arithmetic overflowed and was refused |
| `NoSuchTask` | No task at that index | no task at that index |

The first two are answers. The next three are the analysis reaching a limit of
its own model, which is a different fact and is reported as one: a set refused
with `IterationLimit` has not been shown to fail, only left undecided.

Matching all of them is exhaustive, with no wildcard arm to hide a new case:

```rust
use dy_wcet::{AnalysisFailure, Response};

match set.response_of(1) {
    Response::Bounded(r) => println!("{r} µs, and it meets the deadline"),
    Response::Refused(AnalysisFailure::ExceedsDeadline(r)) => println!("{r} µs, and it misses"),
    Response::Refused(AnalysisFailure::NonConvergent)   => println!("impossible: no fixed point"),
    Response::Refused(AnalysisFailure::IterationLimit)  => println!("undecided: the search hit its cap"),
    Response::Refused(AnalysisFailure::BusyPeriodLimit) => println!("undecided: too many jobs to walk"),
    Response::Refused(AnalysisFailure::Overflow)        => println!("the arithmetic was refused"),
    Response::Refused(AnalysisFailure::NoSuchTask)      => println!("no task at that index"),
}
```

---

## More than one number

Ordering tasks rate-monotonically is left to you, because sorting silently would
hide a mistaken assumption about which task wins. To be told whether *any*
fixed-priority ordering works, ask:

```rust
use dy_wcet::PriorityAssignment;

match set.optimal_priority_order() {
    PriorityAssignment::Found(order)       => println!("this order meets every deadline: {order:?}"),
    PriorityAssignment::NoOrdering         => println!("no fixed-priority ordering does"),
    PriorityAssignment::Inconclusive(why)  => println!("the search could not decide: {why}"),
}
```

That is Audsley's assignment, with one qualification a textbook does not have to
make. His proof assumes an exact schedulability test, and this analysis refuses
some inputs rather than answering them. A refused candidate has not been shown
to miss, only left unexamined. So a level that runs out of candidates with a
refusal among them returns `Inconclusive` rather than claiming no ordering
exists. It returns the ordering and does not apply it.

Once a set holds, two more questions have answers:

```rust
set.slack_of(1);                     // Some(665) — deadline minus response time
set.max_provable_wcet_increase(1);   // Some(465) — the most C can grow and still fit
```

The second figure is the last value that fits, not one past it: at 465 extra the
response is exactly 1000 µs against a 1000 µs deadline, and at 466 it is 1001.
The search re-analyses the whole set, not only the task being changed, because
raising one execution time can sink a task below it, and an answer that checked
only the one you touched would err in the flattering direction.

---

## Symbols and words

Nothing on this page uses a symbol it has not defined.

| | Means | In the API |
|:--|:--|:--|
| **C** | Execution time in the worst case. An input you supply; this crate does not measure it | `Task::new(c, t)` |
| **T** | Period, or the minimum time between releases of a sporadic task | `Task::new(c, t)` |
| **D** | Relative deadline. Defaults to `T`; may be shorter or longer | `.deadline(d)` |
| **J** | Release jitter: how late a release may come after its nominal instant | `.jitter(j)` |
| **B** | Blocking: lower-priority work this task cannot preempt, from a shared resource | `.blocking(b)` |
| **R** | Response time, release to completion. The number this crate computes | `Response::Bounded(r)` |
| **w** | The fixed point of the recurrence, before jitter is added back | internal |
| **L** | The level-*i* busy period: the stretch in which only work at priority *i* or above runs | internal |
| **U** | Utilisation, `C / T`, summed over a priority level | `utilisation_ppm()` |

| Acronym | Expanded | Why it appears |
|:--|:--|:--|
| **WCET** | Worst-Case Execution Time | The `C` above, and the crate's name. The one input it trusts you for |
| **RTA** | Response-Time Analysis | The method: solve the recurrence rather than estimate it |
| **FPPS** | Fixed-Priority Preemptive Scheduling | The scheduling model this analysis is valid under |
| **DM / RM** | Deadline-Monotonic / Rate-Monotonic | Priority orderings that are optimal under stated conditions |
| **ppm** | Parts Per Million | How utilisation is reported. `0.75` and `0.7500001` are different numbers, and a float will not keep them apart |
| **MSRV** | Minimum Supported Rust Version | Declared in `Cargo.toml` and checked in CI |
| **CBMC** | C Bounded Model Checker | What Kani runs underneath: it unrolls loops to a bound and hands the result to a solver |
| **SAT / SMT** | Boolean satisfiability / Satisfiability Modulo Theories | The two kinds of solver CBMC can use |
| **DMA** | Direct Memory Access | Peripherals writing memory without the CPU. Not modelled; the contention it causes is yours to fold into blocking |

---

## The evidence

```sh
git clone https://github.com/DYResearch/dy-wcet && cd dy-wcet
cargo test          # the analysis
./audit.sh          # every number this repository states, against its source
```

26 unit tests in `src/lib.rs` check that the implementation does what its author
intended. 15 integration tests in [`tests/on_paper.rs`](tests/on_paper.rs) check
something harder: that what its author intended is what the analysis says.
Every expected value there is derived in the comment above it, iteration by
iteration, so a reader who distrusts the code can settle it with a pencil.

The rest exist because none of that could catch the defect fixed in 2.0.0, when
the analysis was unsound for deadlines past the period and erred in the
flattering direction:

| File | What it checks |
|:--|:--|
| [`tests/oracle.rs`](tests/oracle.rs) | The busy period computed a second way, growing the job count instead of deriving it. Agreed on 153,365 generated sets when it was added |
| [`tests/differential.rs`](tests/differential.rs) | Keeps the recurrence 2.0.0 replaced, and checks the new answer is never smaller |
| [`tests/properties.rs`](tests/properties.rs) | Eight properties across roughly 28,000 generated sets: a response is never below its own work, jitter never shortens one, Audsley's order holds when applied |
| [`tests/exhaustive.rs`](tests/exhaustive.rs) | Every small task set, not a sample. Confirms Audsley's order is optimal and the sensitivity figure a true maximum, which a random test can agree with but never confirm |
| [`tests/adversarial.rs`](tests/adversarial.rs) | Every public entry point at the edges of the integer domain. A refusal has to be a refusal, never a panic or a wrapped number |
| [`tests/boundaries.rs`](tests/boundaries.rs) | The surface around the recurrence: accessors, admission, and the constants that bound the analysis |
| [`tests/saturation.rs`](tests/saturation.rs) | A level full at exactly one is proved impossible rather than reported as a limit reached |
| [`tests/precheck.rs`](tests/precheck.rs) | The utilisation bound held to its own preconditions |
| [`tests/harness_scope.rs`](tests/harness_scope.rs) | Measures the loop counts the Kani harnesses must unroll, so each bound has a reason |
| [`tests/readme_examples.rs`](tests/readme_examples.rs) | Every snippet on this page, compiled and asserted. A README that stops compiling fails the build |

The generators are a few lines of seeded linear congruence rather than a
dependency, because a dependency tree pulled in to produce pseudo-random `u64`
would cost this crate the one thing it advertises.

Four cases are worth reading even if you never use the crate:

| Case | Why it is there |
|:--|:--|
| [`a_deadline_shorter_than_the_period_can_fail_at_low_utilisation`](tests/on_paper.rs) | Utilisation 0.775, and it misses. A utilisation test calls it safe |
| [`full_utilisation_is_schedulable_when_it_lands_exactly`](tests/on_paper.rs) | U = 1.0 and every deadline is met. A strict inequality reports a failure |
| [`the_deadline_boundary_from_both_sides`](tests/on_paper.rs) | One microsecond changes both the sum and the activation count at once |
| [`a_deadline_beyond_the_period_is_permitted`](tests/on_paper.rs) | An implementation that assumes `R ≤ T` is right on most sets |

### A second method, not a second opinion

The AxonOS kernel carries its own response-time analysis, written earlier for a
different purpose, and on a shared set the two agree. Both are by the same
author, though, and a shared misreading of the recurrence agrees with itself
perfectly. So that comparison is recorded, with every figure derived, in
[`docs/CROSSCHECK.md`](docs/CROSSCHECK.md), and it is not the check this crate
leans on.

The one it leans on does not share the recurrence at all.
[`tools/differential.py`](tools/differential.py) runs the schedule the recurrence
claims to describe: jobs released from the critical instant, the processor
taking the highest-priority ready one, completions measured. Over 5,347
generated sets with jitter, blocking and deadlines past the period, the
simulation never exceeded the computed bound and matched it every time. It runs
in under half a second, so it gates every push.

Its first draft reported 83 unsound results. All 83 were a defect in the
simulation, which merged a task's queued jobs into one counter. The crate was
right and the check was wrong, which is the ordinary outcome and worth saying.

---

## Formal verification

Six Kani harnesses in [`kani/`](kani/) are written to bound what the tests only
sample. **They do not currently verify, and this crate claims no proved
property.** CI runs them on every push as an advisory job that reports what
happened and does not gate the build.

What held them back is worth stating exactly, because "the proofs are red" and
"the proofs are slow" are different facts. Three layers of cost were removed in
4.1.0 to 4.1.2, each one found in the log after the last was fixed:

| Release | What CBMC was unrolling | The fix |
|:--|:--|:--|
| 4.1.0 | A `u128` gcd in the saturation test, whose divider circuits no input range could shrink | Cross-multiplication, no division |
| 4.1.1 | `Flatten`, whose own inner loop the solver could not see stop early | The task array walked as a slice |
| 4.1.2 | Every loop to eighteen, when none under `cfg(kani)` runs past six | An unwind bound of eight, guarded at compile time |

Whether the harnesses now close is for CI to say, and until it does the badge,
the job name and this section all say *advisory*. When the job is green it
becomes a gate, and this paragraph changes to match.

---

## What it does not do

The model is stated so that a set relying on something outside it can be
recognised as outside it, rather than quietly analysed anyway.

| Limit | What that means |
|:--|:--|
| **It does not measure** | Execution times are inputs. If they came from a spreadsheet rather than an oscilloscope, this is arithmetic about a guess, and the crate cannot tell the difference |
| **No cache, pipeline, DMA or bus model** | Blocking is an input, not a derivation |
| **A priority-ceiling protocol is assumed** | A task is blocked at most once. Without one, blocking is not a single number and this analysis does not apply |
| **One core** | No partitioned or global multiprocessor analysis. A set spread over cores needs a different recurrence |
| **A busy period of at most 1024 jobs** | Past that it refuses rather than enumerating. A response spanning a thousand of a task's own periods is not an answer anybody checks |
| **Sixteen tasks maximum** | Not a theoretical limit. It is the point past which a fixed-priority set on one core stops being checkable by hand, and an analysis nobody can check by hand is one nobody checks |
| **It is not a qualified tool** | Under no safety standard, and it does not claim to be. Qualification evidence does not exist yet, and nothing here sells it |

---

## Case study

**[Embassy #6528 — an RP2350 timer that stopped for minutes](case-studies/embassy-6528.md)**

An intermittent `embassy-time` failure on RP2350, traced through hardware alarm
arming, timer-queue liveness, and a response time with no upper bound. The
failure chain is traced in the source, evidence is kept apart from hypothesis,
and each next step names what would confirm or rule it out. It is the standard
of delivery for the audit below, not a sample of one.

## Timing audit

The arithmetic here is one piece of a practice. Some timing problems will not
resolve: a failure that survives every fix, a suspected race, a liveness
question, a bound that has to hold up in review. I take them one at a time, in
writing.

| | Scope | Price |
|:--|:--|:--|
| **Timing audit** | One reproducible problem, traced end to end. Written report in five working days | **$3,000** |
| **Priority** | The same, for a failure that will not wait. Report within 48 hours | **$4,800** |
| **System review** | Several sets, a scheduler design, or verification on a bench | **from $8,000** |

The price is fixed and agreed before anything starts. If the analysis will not
tell you anything useful, I say so before you pay. What it does not include:
hardware instrumentation, running your build, and any claim that your system is
certified. Details in [`AUDIT.md`](AUDIT.md) · [connect@axonos.org](mailto:connect@axonos.org)

## The bounty

I wrote the crate and I wrote its tests, and that is the one problem I cannot
solve from inside: a test written by whoever wrote the bug asserts what the
implementation produces.

So there is a pool, in Dogecoin, for the first task set where this returns a
bound the recurrence does not support — derived, reproducible, on a set that
actually exercises the recurrence. There is no adjudicator. Your derivation and
the crate's output go side by side, and one of them contains a step that does
not add up. **[The rules and the address](BOUNTY.md)**

---

## How this was written

While writing the paper tests, the expected value for a three-task set was
stated as 7 from memory. The arithmetic says 6: at R = 6 the middle task has had
exactly one activation, because `⌈6/6⌉ = 1` and a task released at time 6 does
not interfere with a response that completes at 6.

The recurrence is short enough that being confident about it is easy and being
right about it is not. That is the whole reason this exists, and the case is now
[`three_tasks_settling_at_six`](tests/on_paper.rs).

## Where the analysis comes from

None of the mathematics here is mine. The crate is an implementation, and the
people who worked it out should be named beside the parts they are responsible
for.

| The part | Whose result it is |
|:--|:--|
| The recurrence, `R = C + Σ ⌈R/T⌉·C` | **Joseph & Pandya, 1986.** *Finding response times in a real-time system.* The Computer Journal 29(5), 390–395 |
| Deadlines past the period | **Lehoczky, 1990.** *Fixed priority scheduling of periodic task sets with arbitrary deadlines.* RTSS'90, 201–209. The level-*i* busy period, and the result that every job inside it must be examined, which is what 2.0.0 added and what earlier versions got wrong |
| Release jitter in the recurrence | **Tindell, Burns & Wellings, 1994.** *An extendible approach for analyzing fixed priority hard real-time tasks.* Real-Time Systems 6(2), 133–151. The `⌈(w + J)/T⌉` form, and `R = w + J − qT` |
| Optimal priority assignment | **Audsley, 1991.** *Optimal priority assignment and feasibility of static priority tasks with arbitrary start times.* Technical Report YCS-164, University of York |
| One blocking term per job | **Sha, Rajkumar & Lehoczky, 1990.** *Priority inheritance protocols: an approach to real-time synchronization.* IEEE Transactions on Computers 39(9), 1175–1185 |

What is mine is the implementation, the refusals, the integer arithmetic, and
the tests, including the ones that found my own errors.

---

## Licence

Apache-2.0 OR MIT, at your option: [`LICENSE-APACHE`](LICENSE-APACHE) ·
[`LICENSE-MIT`](LICENSE-MIT).

---

<div align="center">

**DY Research** — [dyresearch.github.io](https://dyresearch.github.io)

Denis Yermakou · [connect@axonos.org](mailto:connect@axonos.org) · [LinkedIn](https://www.linkedin.com/in/axonos)

© 2026 Denis Yermakou

</div>
