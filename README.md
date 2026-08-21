<div align="center">

# dy-wcet

### Worst-case response time, in arithmetic that refuses rather than rounds.

[![CI](https://img.shields.io/github/actions/workflow/status/DYResearch/dy-wcet/ci.yml?branch=main&style=flat-square&label=CI&labelColor=0e141d)](https://github.com/DYResearch/dy-wcet/actions)
[![no_std](https://img.shields.io/badge/no__std-yes-3ecf8e?style=flat-square&labelColor=0e141d)](src/lib.rs)
[![unsafe](https://img.shields.io/badge/unsafe-forbidden-3ecf8e?style=flat-square&labelColor=0e141d)](src/lib.rs)
[![deps](https://img.shields.io/badge/dependencies-0-3ecf8e?style=flat-square&labelColor=0e141d)](Cargo.toml)
[![Licence](https://img.shields.io/badge/Apache--2.0%20OR%20MIT-475569?style=flat-square&labelColor=0e141d)](#licence)

</div>

---

Response-time analysis is a fixed point. A task's response time is its own
execution plus the interference from everything that can preempt it, and the
interference depends on the response time:

```text
R⁰   = C + B
Rⁿ⁺¹ = C + B + Σ ⌈Rⁿ / Tⱼ⌉ · Cⱼ      for every j of higher priority
```

Joseph and Pandya published this in 1986. It fits on a napkin, and the wrong
answers are all quiet.

## Try to get it right

Two tasks. Task A runs 100 µs every 400 µs at higher priority. Task B runs
200 µs every 1000 µs. What is B's worst-case response time?

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

## Three ways this is quietly wrong elsewhere

**In floating point.** A response time in `f64` has last bits that depend on
the compiler and the optimisation level. Two implementations disagree in the
eighth decimal, one rounds a deadline the other misses, and no test pins
either. Everything here is `u64` microseconds — same input, same bits, any
machine.

**By capping the loop.** The recurrence converges only below full utilisation.
Above it the iteration climbs forever, and an implementation that stops after
*n* rounds and returns the last value returns something that looks like an
answer. This returns `Response::Unschedulable`, which fails every deadline
comparison it is put into.

**By wrapping.** Interference is a sum of ceilings of quotients and it grows
fast. A wrapping add turns an unschedulable set into a schedulable one — the
single worst direction for an arithmetic error. Every operation is checked, and
overflow is reported as unschedulable.

## Use

```toml
[dependencies]
dy-wcet = "0.1"
```

```rust
use dy_wcet::{Task, TaskSet, Response};

let mut set = TaskSet::new();
set.push(Task { wcet_us: 100, period_us:  400, deadline_us:  400, blocking_us:  0 })?;
set.push(Task { wcet_us: 200, period_us: 1000, deadline_us: 1000, blocking_us: 20 })?;

match set.response_of(1) {
    Response::Bounded(r) => println!("{r} µs"),   // 320
    Response::Unschedulable => println!("no bound exists"),
}
```

Priority is position: index 0 preempts everything. Rate-monotonic ordering is
the caller's job, because sorting silently would hide a mistaken assumption
about which task wins.

## Verify it yourself

```sh
git clone https://github.com/DYResearch/dy-wcet && cd dy-wcet && cargo test
```

Twelve unit tests in `src/lib.rs` check the implementation does what its author
intended. Nine integration tests in [`tests/on_paper.rs`](tests/on_paper.rs)
check something harder: that what its author intended is what the analysis
says. Every expected value there is derived in the comment above it, iteration
by iteration, so a reader who distrusts the code can settle it with a pencil.

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

## There is money on this being wrong

I wrote the crate and I wrote its tests, which is the one problem I cannot
solve from inside: a test written by whoever wrote the bug asserts what the
implementation produces.

So there is a pool, in Dogecoin, for the first task set where this returns a
bound the recurrence does not support — derived, reproducible, on a set that
actually exercises the recurrence. No adjudicator: your derivation and the
crate's output go side by side, and one of them contains a step that does not
add up.

**[The rules, the conditions, and the address](BOUNTY.md)**

## What it does not do

**It does not measure.** Execution times are inputs. If they came from a
spreadsheet rather than an oscilloscope, this is arithmetic about a guess, and
the crate cannot tell the difference.

**It does not model cache, pipeline, DMA contention or bus stalls.** Blocking
is an input, not a derivation.

**It assumes a priority-ceiling protocol**, under which a task is blocked at
most once. Without one, the blocking term is not a single number and this
analysis does not apply.

**It has no jitter term.** Release jitter widens the interference window and is
not modelled here. A set with significant jitter needs the extended recurrence,
which this crate does not implement.

**Sixteen tasks maximum.** Not a theoretical limit — the point past which a
fixed-priority set on one core stops being checkable by hand, and an analysis
nobody can check by hand is an analysis nobody checks.

## A note on how this was written

While writing the paper tests, the expected value for a three-task set was
stated as 7 from memory. The arithmetic says 6: at R = 6 the middle task has
had exactly one activation, because `⌈6/6⌉ = 1` and a task released at time 6
does not interfere with a response completing at 6.

The recurrence is short enough that being confident about it is easy and being
right about it is not. That is the whole reason this exists, and the case is
now [`three_tasks_settling_at_six`](tests/on_paper.rs).

## Licence

Apache-2.0 OR MIT, at your option.

---

<div align="center">

**DY Research** — [dyresearch.github.io](https://dyresearch.github.io)

Denis Yermakou · [connect@axonos.org](mailto:connect@axonos.org)

© 2026 Denis Yermakou

</div>
