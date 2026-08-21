<!--
SPDX-License-Identifier: CC-BY-SA-4.0
SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
-->

<div align="center">

# The Wrong Number

### Find a task set where `dy-wcet` gives the wrong answer. Take the pool.

[![Pool](https://img.shields.io/badge/pool-see%20below-c2a633?style=for-the-badge&labelColor=0e141d)](#the-pool)
[![Rules](https://img.shields.io/badge/rules-6-5b8def?style=flat-square&labelColor=0e141d)](#what-counts)
[![Verdict](https://img.shields.io/badge/verdict-arithmetic%2C%20not%20opinion-3ecf8e?style=flat-square&labelColor=0e141d)](#who-decides)

</div>

---

Response-time analysis is a fixed point that fits on a napkin, and the wrong
answers are all quiet. They do not crash. They produce a plausible number that
passes a test written by whoever wrote the bug.

[`dy-wcet`](https://github.com/DYResearch/dy-wcet) computes those bounds in
integer arithmetic. I claim it is correct within its stated model.

**Prove me wrong and the pool is yours.**

## What counts

A task set where the crate returns a response time that the recurrence does not
support. Six conditions, all of them mechanical:

| | |
|:--|:--|
| **1** | At most sixteen tasks, all values fitting in `u64` microseconds |
| **2** | Fixed priority, priority by position, priority-ceiling blocking |
| **3** | The crate returns `Bounded(r)` and the correct answer is not `r` — **or** it returns `Unschedulable` where a finite bound exists |
| **4** | Your answer is **derived**: every iteration of the recurrence, with the activation count that produced it |
| **5** | It reproduces. Same input, same output, on any machine, at the released version |
| **6** | The set exercises the recurrence — the sum of execution times must exceed the shortest period |

Condition six exists because I fell into it myself. A set whose execution times
sum below the shortest period never activates any task twice, every ceiling is
1, and the analysis reduces to addition. A "counterexample" on such a set
disproves nothing, and the write-up of how I discovered that is
[here](https://github.com/DYResearch/dy-wcet/blob/main/docs/CROSSCHECK.md).

## What does not count

**A disagreement with the model.** No jitter term, no cache, no pipeline, no
DMA contention, blocking as an input rather than a derivation. These are stated
limits, not defects, and a set that relies on one of them is outside the claim.

**A panic, a hang, or an overflow.** Those are bugs and I want them — open an
issue and I will fix them — but they are not this. This is about a number that
is wrong while looking right.

**An argument.** Bring arithmetic. If your derivation and mine disagree, one of
them contains a step that does not add up, and finding which is not a matter of
opinion.

## Who decides

Nobody. Every claim is settled by re-adding the arithmetic:

```sh
git clone https://github.com/DYResearch/dy-wcet && cd dy-wcet
cargo test                      # the crate agrees with itself
```

Then your set, your derivation, and the crate's output side by side. If your
derivation is sound and the crate disagrees with it, the crate is wrong. There
is no judgement call in that, which is the point — a bounty adjudicated by the
person paying it is not a bounty.

Every submission and its verdict is published here, including the ones that
fail and why. A challenge whose losses are invisible is a challenge nobody can
calibrate.

## The pool

Dogecoin, one address, balance public on any explorer:

```
DMwHAhqVNWf7dyEznukxCufNS5rjuP5MTp
```

<sub>[View the balance and every transaction →](https://dogechain.info/address/DMwHAhqVNWf7dyEznukxCufNS5rjuP5MTp)</sub>

**Anyone can add to it.** The whole balance goes to the first valid
counterexample. If nobody finds one, it stays and grows.

### What contributors get

Not equity — there is nothing to sell you, and anybody offering a share of a
one-person project for a crypto transfer should be treated with suspicion.

**Your name in this file, permanently**, with the amount, the date and the
transaction hash. A public record on a chain nobody controls, in a repository
that will outlive the challenge.

**And seventy-two hours.** Every release, audit, cross-check and post-mortem in
this project goes to the contributor list three days before it goes public.
That includes the write-ups of my own mistakes, which are the ones worth
reading early.

Send, then email **connect@axonos.org** with the transaction hash and the name
you want recorded. Anonymous is fine — say so and the entry reads *anonymous*
with the hash beside it.

## Who has contributed

| Date | Contributor | Amount | Transaction |
|:--|:--|--:|:--|
| — | *open* | — | — |

<sub>Empty as of publication. It is listed empty rather than hidden, because a
pool that appears only once it is impressive is a pool nobody can trust the
size of.</sub>

## Claims

| Date | Set | Verdict | Why |
|:--|:--|:--|:--|
| — | *none yet* | — | — |

## Why I am doing this

Two reasons, and the second is the honest one.

**Because it is the only test I cannot write.** I wrote both the crate and its
tests. A test written by whoever wrote the bug asserts what the implementation
produces, and there is no way for me to escape that from inside. Somebody else
trying to break it is the only mechanism that reaches what I cannot.

**Because I need the work to be seen.** I build this alone and I would rather
be read than not. A challenge is a reason for a stranger to spend an evening
inside my code, and whether they win or not, they will have read it — which is
worth more to me than the pool.

---

<div align="center">

**DY Research** — Denis Yermakou

[github.com/DYResearch](https://github.com/DYResearch) ·
[connect@axonos.org](mailto:connect@axonos.org)

© 2026 Denis Yermakou · CC-BY-SA-4.0

</div>
