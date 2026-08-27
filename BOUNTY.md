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
| **3** | The crate returns `Bounded(r)` and the correct answer is not `r` — **or** it returns `Unschedulable` where a finite bound exists **at or below that task's deadline** |
| **4** | Your answer is **derived**: every iteration of the recurrence, with the activation count that produced it |
| **5** | It reproduces. Same input, same output, on any machine, at the released version |
| **6** | The set exercises the recurrence — the sum of execution times must exceed the shortest period |

**Condition three was narrowed on 27 August 2026.** It previously read "where a
finite bound exists", full stop, and `response_of` stops iterating the moment
`R` passes the deadline — so a set whose fixed point sits just above its
deadline returned `Unschedulable` while a finite bound existed, and claimed the
pool in one line. That is the deadline exit working as designed and it was not
what this challenge meant to offer. The narrowing is dated here rather than
applied quietly, per the section below, and claims sent before that date are
judged against the wording that was live when they were sent.

Condition six exists because I fell into it myself. A set whose execution times
sum below the shortest period never activates any task twice, every ceiling is
1, and the analysis reduces to addition. A "counterexample" on such a set
disproves nothing, and the write-up of how I discovered that is
[here](https://github.com/DYResearch/dy-wcet/blob/main/docs/CROSSCHECK.md).

## Which version

Claims are judged against the **latest published tag** at the moment you send
them. The crate is small and changes rarely, and if a release lands between
your discovery and your email, say which tag you were on — a counterexample
that was valid an hour ago does not stop being interesting because I pushed a
commit.

If a release **fixes** the behaviour you found, that is still a win: the fix is
credited to you and the pool pays. Finding it first is the achievement, not
finding it before I did.

## These rules do not change retroactively

The version of this file that was current when you started is the one your
claim is judged against. Its history is public in this repository, so you can
prove what it said — I cannot quietly add a condition after seeing a
submission, which is the failure mode of every informal bounty.

If the rules change, the change applies to claims sent after it, and the reason
is written in the commit message.

## What does not count

**A disagreement with the model.** No jitter term, no cache, no pipeline, no
DMA contention, blocking as an input rather than a derivation. These are stated
limits, not defects, and a set that relies on one of them is outside the claim.

**The deadline exit.** `response_of` returns `Unschedulable` as soon as the
iteration passes the task's deadline, because continuing tells the caller
nothing they did not already know. A finite bound may exist above the deadline
and the crate does not look for it. That is documented behaviour rather than a
defect, and distinguishing the two is the first candidate for 0.2.0 — but it is
a behaviour change and is not being made against an unrun test suite.

**A panic, a hang, or an overflow.** Those are bugs and I want them — open an
issue and I will fix them — but they are not this. This is about a number that
is wrong while looking right.

**An argument.** Bring arithmetic. If your derivation and mine disagree, one of
them contains a step that does not add up, and finding which is not a matter of
opinion.

## How to submit

Email **connect@axonos.org**, or open an issue if you would rather it be
public from the start. Either is fine; the issue is faster and gets your name
on it sooner.

What to include:

```
Tag:        v0.1.2
Task set:   C, T, D, B per task, highest priority first
Crate says: Bounded(1830)  /  Unschedulable
You say:    Bounded(1710)
Derivation: R = 900   ⌈900/1000⌉·120 + ⌈900/4000⌉·642  →  1710
            R = 1710  ⌈1710/1000⌉·120 + ...            →  1830
            ...
```

The derivation is the part that matters. A set and a number without the
iterations is a claim I would have to reconstruct before I could check it, and
reconstructing somebody else's reasoning is how a disagreement about
arithmetic turns into a disagreement about intent.

If you would rather not be named, say so and the entry reads *anonymous* with
the date.

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

## Payment, and what happens if the pool is small

**Within 72 hours** of a claim being confirmed, to a Dogecoin address you give
me, with the transaction hash published in this file beside your entry.

The pool is whatever the address holds at the moment a claim is confirmed. If
that is a small amount, it is a small amount — I will say so plainly rather
than pretend otherwise, and the finding still gets published under your name
with the derivation intact.

Which is the honest position: **the credit is the durable part.** A repository
that records who broke it, with their reasoning, outlives whatever the balance
happened to be that week.

If two valid claims arrive for the same defect, the earlier timestamp takes the
pool and both are published. If they arrive within an hour of each other, it is
split, because an hour is inside the noise of when somebody happened to hit
send.

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
