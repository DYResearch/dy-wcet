#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0 OR MIT
# SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
# DY Research — https://dyresearch.github.io
"""Differential campaign: the crate against the independent scheduler.

    cargo build --release --example rta_probe
    python3 tools/campaign.py --cases 20000 --seed 1

Generated task sets go to `examples/rta_probe`, which is the real crate, and to
`reference/scheduler.py`, which is a discrete-event simulation sharing no
formula with it. Disagreements are written to `artifacts/failures/<n>.json`
with the seed that produced them.

What counts as a disagreement, and what does not:

* Both produce a number and the numbers differ — a defect, always.
* The crate refuses (`N`, `I`, `P`, `O`) and the reference has a number — not
  counted. The crate declining to enumerate or to represent a value in 64 bits
  is a documented refusal, not a wrong answer. These are counted separately so
  that a release can state how much of the campaign actually compared numbers.
* The reference refuses and the crate has a number — counted and reported. The
  reference has arbitrary-precision integers and a 4096-job horizon, so this
  means the simulation ran out of room where the analysis did not, and it is
  worth a look rather than a silent pass.

Boundary generation is deliberate, not incidental. Half the cases are drawn
around the values where integer ceilings change: a period either side of a
multiple of another period, a deadline at exactly the response, jitter at zero
and at a full period. Uniform random sampling almost never lands on those.
"""
from __future__ import annotations

import argparse
import json
import pathlib
import random
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
from reference.scheduler import Task, encode  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parents[1]
PROBE = ROOT / "target" / "release" / "examples" / "rta_probe"
FAILURES = ROOT / "artifacts" / "failures"

REFUSALS = {"N", "I", "P", "O", "X"}


def generate(rng: random.Random, boundary: bool) -> list[Task]:
    n = rng.randint(1, 5)
    tasks: list[Task] = []
    base = rng.choice([10, 100, 1000])
    for k in range(n):
        t = base * rng.randint(1, 20)
        if boundary:
            t += rng.choice([-1, 0, 0, 1])
            t = max(2, t)
        c = max(1, t // rng.randint(2, 12))
        if boundary:
            c = max(1, c + rng.choice([-1, 0, 1]))
        d = t
        if boundary:
            d = max(c, t + rng.choice([-1, 0, 1, t // 2, -t // 3]))
        elif rng.random() < 0.4:
            d = max(c, int(t * rng.choice([0.5, 0.8, 1.5, 2.0])))
        j = 0
        if rng.random() < 0.4:
            j = rng.choice([0, 1, t - 1, t, t * 2]) if boundary else rng.randint(0, t)
        b = 0
        if k == n - 1 and rng.random() < 0.3:
            b = rng.choice([0, 1, d - 1, d]) if boundary else rng.randint(0, max(1, d // 4))
        tasks.append(Task(c=c, t=t, d=max(1, d), j=max(0, j), b=max(0, b)))
    tasks.sort(key=lambda x: x.t)  # a plausible ordering; not required to be optimal
    return tasks


def admissible(tasks: list[Task]) -> bool:
    return all(t.t > 0 and t.c <= t.d for t in tasks)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--cases", type=int, default=10_000)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--quiet", action="store_true")
    args = ap.parse_args()

    if not PROBE.exists():
        print(f"build the probe first:\n  cargo build --release --example rta_probe")
        return 2

    rng = random.Random(args.seed)
    cases: list[list[Task]] = []
    while len(cases) < args.cases:
        t = generate(rng, boundary=len(cases) % 2 == 0)
        if admissible(t):
            cases.append(t)

    stdin = "\n".join(
        " ".join([str(len(c))] + [str(v) for t in c for v in (t.c, t.t, t.d, t.j, t.b)])
        for c in cases
    )
    proc = subprocess.run(
        [str(PROBE)], input=stdin, capture_output=True, text=True, check=True
    )
    lines = proc.stdout.strip().split("\n")
    if len(lines) != len(cases):
        print(f"::error::probe returned {len(lines)} lines for {len(cases)} cases")
        return 2

    compared = mismatched = crate_refused = reference_refused = rejected = 0
    FAILURES.mkdir(parents=True, exist_ok=True)

    for n, (tasks, line) in enumerate(zip(cases, lines)):
        if line.startswith("R:"):
            rejected += 1
            continue
        theirs = line.split()
        mine = [encode(tasks, i) for i in range(len(tasks))]
        for i, (a, b) in enumerate(zip(mine, theirs)):
            if b in REFUSALS:
                crate_refused += 1
                continue
            if a == "REFUSED":
                reference_refused += 1
                continue
            compared += 1
            if a != b:
                mismatched += 1
                path = FAILURES / f"{args.seed}-{n}-{i}.json"
                path.write_text(
                    json.dumps(
                        {
                            "seed": args.seed,
                            "case": n,
                            "task_index": i,
                            "tasks": [t.__dict__ for t in tasks],
                            "production_result": theirs,
                            "reference_result": mine,
                            "status": "MISMATCH",
                        },
                        indent=2,
                    )
                )

    if not args.quiet:
        print(f"cases              {len(cases)}")
        print(f"rejected at push   {rejected}")
        print(f"numbers compared   {compared}")
        print(f"crate refused      {crate_refused}")
        print(f"reference refused  {reference_refused}")
        print(f"MISMATCHES         {mismatched}")
    if mismatched:
        print(f"artifacts in {FAILURES}")
    return 1 if mismatched else 0


if __name__ == "__main__":
    sys.exit(main())
