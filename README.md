<div align="center">

# dy-wcet

### Worst-case response time, in arithmetic that refuses rather than rounds.

[![Version](https://img.shields.io/badge/version-0.1.0-0a4a8f?style=flat-square&labelColor=0e141d)](CHANGELOG.md)
[![no_std](https://img.shields.io/badge/no__std-yes-0d7a5f?style=flat-square&labelColor=0e141d)](src/lib.rs)
[![unsafe](https://img.shields.io/badge/unsafe-forbidden-0d7a5f?style=flat-square&labelColor=0e141d)](src/lib.rs)
[![Dependencies](https://img.shields.io/badge/dependencies-0-0d7a5f?style=flat-square&labelColor=0e141d)](Cargo.toml)
[![License](https://img.shields.io/badge/Apache--2.0%20OR%20MIT-475569?style=flat-square&labelColor=0e141d)](#licence)

</div>

---

Response-time analysis is a fixed point: a task's response time is its own
execution plus the interference from everything that can preempt it, and the
interference depends on the response time. Joseph and Pandya published the
recurrence in 1986 and it is short enough to write on a napkin.

Getting it right is where the work is, and the wrong answers are all quiet.

## Three quiet ways to get it wrong

**In floating point.** A response time computed in `f64` has last bits that
depend on the compiler. Two implementations disagree in the eighth decimal, one
rounds a deadline the other misses, and no test pins either. Everything here is
`u64` microseconds: same input, same bits, any machine.

**By capping the loop.** The recurrence converges only below full utilisation.
Above it the iteration climbs forever, and an implementation that stops after
*n* rounds and returns the last value returns something that looks like an
answer. This returns `Response::Unschedulable`, which fails every deadline
comparison — an infinite response time is the honest value and cannot be
mistaken for a small one.

**By wrapping.** Interference is a sum of ceilings of quotients and it grows
fast. A wrapping add turns an unschedulable set into a schedulable one, which
is the single worst direction for an arithmetic error. Every operation is
checked; overflow is reported as unschedulable.

## The surprise worth knowing

```rust
set.push(Task { wcet_us: 100, period_us: 400,  deadline_us: 400,  blocking_us: 0 })?;
set.push(Task { wcet_us: 200, period_us: 1000, deadline_us: 1000, blocking_us: 0 })?;
set.response_of(1);   // Bounded(300), not 400
```

The obvious guess is 400: two preemptions, one per 400 µs of the low task's
1000 µs period. It is wrong. The window that counts is the **response time**,
not the period — at R = 200 one activation fits, giving 300, and at R = 300
still one fits, so 300 is the fixed point.

An implementation that used the period would over-estimate here and
under-estimate elsewhere. Both are in the test suite.

## What this does not do

**It does not measure.** It computes a bound from execution times somebody else
established. A bound is only as good as its inputs, and if those come from a
spreadsheet rather than a scope, the result is arithmetic about a guess.

**It does not model cache, pipeline, DMA contention or bus stalls.** Blocking
is an input, not a derivation.

**It assumes a priority-ceiling protocol**, under which a task is blocked at
most once. Without one, the blocking term is not a single number and this
analysis does not apply.

## Use

```toml
[dependencies]
dy-wcet = "0.1"
```

`no_std`, no dependencies, no allocation. Sixteen tasks maximum — not a
theoretical limit, but the point past which a fixed-priority set on one core
stops being checkable by hand, and an analysis nobody can check by hand is an
analysis nobody checks.

## Licence

Apache-2.0 OR MIT, at your option.

---

<div align="center">

**DY Research** — systems work with the arithmetic shown

© 2026 Denis Yermakou · [connect@axonos.org](mailto:connect@axonos.org)

</div>

