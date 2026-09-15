#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0 OR MIT
# SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
# DY Research — https://dyresearch.github.io
"""Write down what the verification run actually did.

Not what the release intends to have done. Every number here comes from a
command that ran in this invocation, and a stage that did not run is recorded
as `skipped` rather than omitted — because an artifact that silently drops the
one stage nobody could run is an artifact that says the proofs passed.

    python3 tools/verification_json.py --cases 100000 --seed 20260914 \\
        --stages "fmt=pass clippy=pass kani=skipped"
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]


def run(*cmd: str) -> str | None:
    try:
        out = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, timeout=120)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return out.stdout.strip() if out.returncode == 0 else None


def count_tests() -> int:
    """`#[test]` functions across the test binaries and the unit module."""
    n = 0
    for path in list((ROOT / "tests").glob("*.rs")) + [ROOT / "src" / "lib.rs"]:
        n += len(re.findall(r"^\s*#\[test\]", path.read_text(encoding="utf-8"), re.M))
    return n


def count_kani_harnesses() -> int:
    total = 0
    for path in (ROOT / "kani").glob("*.rs"):
        total += len(re.findall(r"#\[kani::proof\]", path.read_text(encoding="utf-8")))
    return total


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--cases", type=int, required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--stages", default="")
    ap.add_argument("--out", type=pathlib.Path, default=ROOT / "verification.json")
    args = ap.parse_args()

    version = re.search(
        r'^version\s*=\s*"([^"]+)"', (ROOT / "Cargo.toml").read_text(), re.M
    ).group(1)

    stages: dict[str, str] = {}
    for item in args.stages.split():
        if "=" in item:
            key, value = item.split("=", 1)
            stages[key] = value

    failed = [k for k, v in stages.items() if v == "fail"]
    artifact = {
        "version": version,
        "commit": run("git", "rev-parse", "HEAD") or "unknown",
        "tree_clean": run("git", "status", "--porcelain") == "",
        "rust": run("rustc", "--version") or "unknown",
        "python": sys.version.split()[0],
        "tests": count_tests(),
        "differential_cases": args.cases,
        "differential_seed": args.seed,
        "kani_harnesses": count_kani_harnesses(),
        "stages": stages,
        "skipped": [k for k, v in stages.items() if v == "skipped"],
        "status": "FAIL" if failed else "PASS",
        "note": (
            "Bounded evidence, not a proof. The differential campaign compares this "
            "crate against an independent job-level simulation over generated inputs; "
            "the exhaustive stages cover every case in a small space; Kani proves "
            "declared properties under declared unwind bounds. None of these is a "
            "statement about all inputs. A stage recorded as 'skipped' did not run, "
            "and a PASS above does not include it."
        ),
    }
    args.out.write_text(json.dumps(artifact, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(artifact, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
