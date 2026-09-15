#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0 OR MIT
# SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
# DY Research — https://dyresearch.github.io
"""An independent model: run the schedule, watch the clock, record completions.

This is not a second copy of the recurrence. It contains no `ceil_div`, no
`W(q)`, no busy-period equation and no interference sum. It releases jobs into
a ready queue, picks the highest-priority ready job, runs it until something
else arrives, and writes down when each job finishes. The response time of a
job is its completion minus its own nominal release. That is the definition
response-time analysis is trying to compute, approached from the definition
rather than from the formula.

The distinction matters because the previous arrangement did not have it. Until
3.0.0 `tools/differential.py` held a Python transcription of the recurrence
*and* a Python simulation, and compared those two to each other. Its docstring
said the transcription was deliberate so that a divergence would be a
transcription error rather than a second design — which is true, and is exactly
why it could not catch a defect in the crate. Nothing in that loop ever read
the Rust.

Arithmetic is Python's arbitrary-precision integers throughout, so a result
here cannot be wrong through a `u64` overflow. Where the crate refuses because
64 bits cannot hold an intermediate value, this model still has an answer, and
the campaign treats that as agreement rather than as a mismatch: refusing to
represent a number is not the same as computing it wrongly.

## The task model, stated rather than assumed

Task *i* has execution `C`, period `T`, relative deadline `D`, release jitter
`J` and blocking `B`, and priority is index order — 0 preempts everything.

Jobs of task *j* have nominal releases at `k·T`; jitter delays the *actual*
release by up to `J`, so the worst case for a lower-priority task is every
higher-priority job arriving as early as its jitter allows. Taking the critical
instant as time zero, job *k* of task *j* becomes ready at `k·T − J`, and the
job with a negative value there is one released before the instant and still
queued at it. The count of releases in `[0, t)` under this arrangement is
`⌈(t + J)/T⌉`, which is the quantity the analytic form uses — arrived at here
by placing arrivals on a timeline, not by writing that expression down.

A job's response is measured from its *nominal* release `k·T − J`, not from
when it actually started queueing, because that is what a deadline is relative
to in this model.

Blocking is charged once, at the head of the level-*i* busy period: `B` units of
lower-priority work that the task under analysis cannot preempt.
"""
from __future__ import annotations

import heapq
import json
from dataclasses import dataclass


@dataclass(frozen=True)
class Task:
    c: int
    t: int
    d: int
    j: int = 0
    b: int = 0


#: How far the simulation will run before it gives up on the busy period ending.
#: A refusal here is reported as a refusal, never as a response time.
HORIZON_JOBS = 4096

#: A hard ceiling on scheduling events per case.
#:
#: The first version of this file had no such ceiling, and the campaign at
#: 40 000 cases ran for over half an hour on a workload that takes two seconds
#: for five hundred. One set was enough: the ready queue was re-sorted on every
#: event, so a busy period holding thousands of queued jobs turned a linear
#: simulation into a quadratic one, and nothing said so — the campaign simply
#: never finished. A reference model with no bound on its own running time
#: cannot be a CI gate, because "still running" and "found nothing" look
#: identical from outside. The queue is a heap now, and this is the bound.
#:
#: The first bound was two million, which terminates and is still useless: a
#: case that reaches it spends tens of seconds here, and a hundred-thousand
#: case campaign inherits that as a tail. A budget for a gate has to bound
#: *time*, not merely guarantee termination. This one costs well under a
#: second in the worst case, and a case that exhausts it is reported as a
#: refusal and counted, never quietly dropped or retried.
EVENT_BUDGET = 150_000


class Refused(Exception):
    """The model declined to produce a number, and says why."""


def response_of(tasks: list[Task], index: int) -> int:
    """Worst response of task `index` over its level-`index` busy period.

    Runs the schedule from the critical instant. Returns the largest response
    seen among the jobs of `index` released before the processor first goes
    idle with respect to this priority level and above.

    The ready set is a heap rather than a list re-sorted at every event. That is
    not a micro-optimisation: a pathological set can hold thousands of queued
    jobs, and re-sorting all of them on every preemption made the simulation
    quadratic in the length of the busy period. A campaign of 40 000 cases ran
    for over half an hour without finishing, on a workload that takes seconds
    for five hundred, and reported nothing while it did — which is the argument
    for [`EVENT_BUDGET`] as much as for the heap. A reference model that can
    stall cannot be a gate, because "still running" and "found nothing" look
    the same from outside.
    """
    me = tasks[index]
    level = tasks[: index + 1]  # this priority and higher; lower cannot preempt

    next_k = [0] * len(level)
    ready: list[tuple[int, int, int, int]] = []  # (priority, job, order, remaining)

    now = 0
    # Blocking: lower-priority work already holding the processor when the busy
    # period opens. It cannot be preempted, so it simply runs.
    if me.b:
        now += me.b

    worst = None
    completed_of_me = 0
    order = 0

    def release_due(t_now: int) -> None:
        nonlocal order
        for i, task in enumerate(level):
            while next_k[i] * task.t - task.j <= t_now:
                heapq.heappush(ready, (i, next_k[i], order, task.c))
                order += 1
                next_k[i] += 1
                if next_k[i] > HORIZON_JOBS:
                    raise Refused("release horizon")

    release_due(now)
    events = 0

    while ready:
        events += 1
        if events > EVENT_BUDGET:
            raise Refused("event budget")

        # Highest priority first; among equals, the earlier job. The key never
        # changes while a job runs, so the heap can be popped and pushed back
        # rather than re-sorted.
        i, k, seq, remaining = heapq.heappop(ready)

        # Run until this job finishes or a higher-priority job arrives.
        next_arrival = None
        for other in range(0, i):  # strictly higher priority
            a = next_k[other] * level[other].t - level[other].j
            if next_arrival is None or a < next_arrival:
                next_arrival = a

        if next_arrival is not None and now + remaining > next_arrival:
            slice_len = next_arrival - now
            if slice_len <= 0:
                raise Refused("time did not advance")
            heapq.heappush(ready, (i, k, seq, remaining - slice_len))
            now = next_arrival
            release_due(now)
            continue

        now += remaining
        if i == index:
            completed_of_me += 1
            nominal = k * me.t - me.j
            r = now - nominal
            if worst is None or r > worst:
                worst = r
        release_due(now)

        # The level-i busy period ends when nothing at this level or above is
        # ready and the next release of this task is still in the future.
        if not ready:
            nxt = next_k[index] * me.t - me.j
            if nxt > now:
                break

        if completed_of_me > HORIZON_JOBS:
            raise Refused("job horizon")

    if worst is None:
        raise Refused("no job of this task completed")
    return worst


def encode(tasks: list[Task], index: int) -> str:
    """The same alphabet `examples/rta_probe.rs` prints, for comparison."""
    try:
        r = response_of(tasks, index)
    except Refused:
        return "REFUSED"
    return f"B:{r}" if r <= tasks[index].d else f"E:{r}"


def case_json(seed: int, tasks: list[Task], mine: list[str], theirs: list[str]) -> str:
    return json.dumps(
        {
            "seed": seed,
            "tasks": [t.__dict__ for t in tasks],
            "production_result": theirs,
            "reference_result": mine,
            "status": "MISMATCH",
        },
        indent=2,
    )
