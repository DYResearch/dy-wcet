#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0 OR MIT
# SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
# DY Research — https://dyresearch.github.io
"""Check the recurrence against a simulated schedule.

The crate computes worst-case response time by solving a recurrence. This
computes the same quantity a different way — by running the fixed-priority
preemptive schedule from the critical instant and measuring when jobs actually
finish — and compares the two.

The recurrence is transcribed from src/lib.rs line by line rather than
rewritten, so that a divergence between this file and the crate is a
transcription error and not a second design. The simulation shares nothing with
it beyond the task model.

What this is evidence for, and what it is not: agreement over several thousand
generated sets is not a proof, and the Kani harnesses in kani/ are where the
bounded proofs live. This covers ground the harnesses cannot, because it runs
the whole analysis over sets whose recurrence takes more iterations than any
unwind limit admits.

    python3 tools/differential.py

An early draft of this file reported 83 unsound results. Every one was a defect
in the simulation, which merged a task's queued jobs into a single counter and
so recorded the completion of the last as the completion of the first. The
lesson is kept in the code: jobs are held in a FIFO, each with its own index.
"""

import random
from dataclasses import dataclass


@dataclass(frozen=True)
class T:
    c: int       # WCET
    t: int       # period
    d: int       # deadline
    j: int = 0   # release jitter
    b: int = 0   # blocking


ITERATION_CAP = 10_000
BUSY_PERIOD_CAP = 1_024


def ceil_div(a: int, b: int) -> int:
    """As src/lib.rs writes it: a // b + (a % b != 0)."""
    return a // b + (1 if a % b else 0)


# ── side A: the recurrence, transcribed from src/lib.rs ────────────────────

def response_recurrence(tasks, i):
    """Returns (kind, value). kind in {"bounded", "exceeds", "refused"}."""
    task = tasks[i]

    # utilisation gate
    u = sum(1_000_000 * x.c // x.t for x in tasks[: i + 1])
    if u > 1_000_000:
        return ("refused", "utilisation")

    # level-i busy period
    busy = task.b + sum(x.c for x in tasks[: i + 1])
    settled = False
    for _ in range(ITERATION_CAP):
        nxt = task.b
        for x in tasks[: i + 1]:
            nxt += ceil_div(busy + x.j, x.t) * x.c
        if nxt == busy:
            settled = True
            break
        busy = nxt
    if not settled:
        return ("refused", "busy-period")

    jobs = max(1, ceil_div(busy + task.j, task.t))
    if jobs > BUSY_PERIOD_CAP:
        return ("refused", "job-count")

    worst = 0
    for q in range(jobs):
        base = (q + 1) * task.c + task.b
        w = base
        ok = False
        for _ in range(ITERATION_CAP):
            nxt = base
            for h in tasks[:i]:
                nxt += ceil_div(w + h.j, h.t) * h.c
            if nxt == w:
                ok = True
                break
            w = nxt
        if not ok:
            return ("refused", "job-recurrence")
        r = w + task.j - q * task.t
        worst = max(worst, r)

    if worst > task.d:
        return ("exceeds", worst)
    return ("bounded", worst)


# ── side B: simulate the schedule and measure ──────────────────────────────

def response_simulated(tasks, i, horizon):
    """Run the preemptive fixed-priority schedule from the critical instant.

    Every task releases its first job at t=0 having suffered maximum jitter;
    subsequent jobs of task x arrive at k*T - J. That is the release pattern
    the recurrence assumes.

    Jobs are held in a per-task FIFO, each with its own remaining execution and
    its own index. Merging a task's queued jobs into a single counter — the
    obvious shortcut — reports the completion of the last queued job as the
    completion of the first, which understates nothing and overstates plenty.
    """
    releases = []
    for x in tasks[: i + 1]:
        rs, k = [], 0
        while True:
            r = max(0, k * x.t - x.j)
            if r > horizon:
                break
            rs.append(r)
            k += 1
        releases.append(rs)

    queues = [[] for _ in range(i + 1)]   # each entry: [job_index, remaining]
    next_job = [0] * (i + 1)
    responses = []
    blocking_left = tasks[i].b

    for now in range(horizon):
        for k in range(i + 1):
            while next_job[k] < len(releases[k]) and releases[k][next_job[k]] <= now:
                queues[k].append([next_job[k], tasks[k].c])
                next_job[k] += 1

        if blocking_left > 0:
            blocking_left -= 1
            continue

        run = next((k for k in range(i + 1) if queues[k]), None)
        if run is None:
            continue
        job = queues[run][0]
        job[1] -= 1
        if job[1] == 0:
            q = job[0]
            queues[run].pop(0)
            if run == i:
                responses.append(now + 1 + tasks[i].j - q * tasks[i].t)

    return max(responses) if responses else None


# ── the comparison ─────────────────────────────────────────────────────────

def random_set(rng, n):
    tasks = []
    for _ in range(n):
        t = rng.choice([4, 5, 6, 8, 10, 12, 15, 20, 25, 30])
        c = rng.randint(1, max(1, t // 2))
        d = rng.choice([t, t, t, t + rng.randint(0, t)])   # some D > T
        j = rng.choice([0, 0, 0, 1, 2])
        tasks.append(T(c=c, t=t, d=max(d, c), j=j))
    tasks.sort(key=lambda x: x.t)                          # rate-monotonic order
    return tasks


def main() -> int:
    rng = random.Random(20260913)
    checked = skipped = 0
    mismatches = []
    gaps = []

    for _ in range(4000):
        n = rng.randint(2, 4)
        tasks = random_set(rng, n)
        u = sum(1_000_000 * x.c // x.t for x in tasks)
        if u > 900_000:
            continue                                       # keep horizons small
        for i in range(len(tasks)):
            kind, val = response_recurrence(tasks, i)
            if kind == "refused":
                skipped += 1
                continue
            # simulate over several hyperperiod-ish horizons
            horizon = 0
            lcm = 1
            for x in tasks[: i + 1]:
                a, b = lcm, x.t
                while b:
                    a, b = b, a % b
                lcm = lcm * x.t // a
            horizon = min(lcm * 2, 6000)
            sim = response_simulated(tasks, i, horizon)
            if sim is None:
                skipped += 1
                continue
            checked += 1
            if sim > val:
                mismatches.append((tasks, i, val, sim))
            gaps.append(val - sim)

    print(f"compared:   {checked}")
    print(f"skipped:    {skipped}  (refused by the recurrence, or no completion in horizon)")
    print(f"unsound:    {len(mismatches)}  (simulation exceeded the computed bound)")
    if gaps:
        exact = sum(1 for g in gaps if g == 0)
        print(f"exact:      {exact} of {len(gaps)} ({100*exact/len(gaps):.1f}%) — bound equals the simulated worst case")
        print(f"pessimism:  max {max(gaps)} µs, mean {sum(gaps)/len(gaps):.2f} µs")
    for tasks, i, val, sim in mismatches[:5]:
        print(f"\n  task {i} of {[(x.c, x.t, x.d, x.j) for x in tasks]}")
        print(f"    recurrence says {val}, simulation reached {sim}")
    return 1 if mismatches else 0


if __name__ == "__main__":
    raise SystemExit(main())
