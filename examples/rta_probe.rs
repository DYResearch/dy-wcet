// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
// DY Research — https://dyresearch.github.io

//! Reads task sets on stdin, prints what the crate says about them.
//!
//! This exists because until 3.0.0 the differential campaign never touched the
//! crate. `tools/differential.py` compared a Python transcription of the
//! recurrence against a Python simulation of the schedule — two Python
//! programs — and its own docstring said the transcription was deliberate so
//! that a divergence would be a transcription error. That is true, and it is
//! also the problem: a transcription that drifts from `src/lib.rs` leaves the
//! campaign green while the shipped analysis is wrong. Nothing in the loop read
//! the Rust.
//!
//! Line format in, one case per line, whitespace separated:
//!
//! ```text
//! n  c1 t1 d1 j1 b1  c2 t2 d2 j2 b2  ...
//! ```
//!
//! Line format out, one per case, one field per task in priority order:
//!
//! ```text
//! B:<r>   bounded at r
//! E:<r>   a finite bound r, past the deadline
//! N       non-convergent: utilisation through this level exceeds one
//! I       the recurrence hit the iteration cap
//! P       the busy period held more jobs than the enumeration cap
//! O       checked arithmetic refused
//! X       no task at that index
//! R:<k>   the set was rejected at construction, k = why
//! ```

use std::io::{self, BufRead, Write};

use dy_wcet::{AnalysisFailure, Response, Task, TaskSet};

fn encode(r: Response) -> String {
    match r {
        Response::Bounded(v) => format!("B:{v}"),
        Response::Refused(AnalysisFailure::ExceedsDeadline(v)) => format!("E:{v}"),
        Response::Refused(AnalysisFailure::NonConvergent) => "N".into(),
        Response::Refused(AnalysisFailure::IterationLimit) => "I".into(),
        Response::Refused(AnalysisFailure::BusyPeriodLimit) => "P".into(),
        Response::Refused(AnalysisFailure::Overflow) => "O".into(),
        Response::Refused(AnalysisFailure::NoSuchTask) => "X".into(),
    }
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<u64> = line
            .split_whitespace()
            .map(|x| x.parse().expect("integer field"))
            .collect();
        let n = f[0] as usize;

        let mut set = TaskSet::new();
        let mut rejected: Option<String> = None;
        for k in 0..n {
            let o = 1 + k * 5;
            let t = Task::new(f[o], f[o + 1])
                .deadline(f[o + 2])
                .jitter(f[o + 3])
                .blocking(f[o + 4]);
            if let Err(e) = set.push(t) {
                rejected = Some(format!("R:{e:?}"));
                break;
            }
        }

        match rejected {
            Some(r) => writeln!(out, "{r}")?,
            None => {
                let fields: Vec<String> = (0..n).map(|i| encode(set.response_of(i))).collect();
                writeln!(out, "{}", fields.join(" "))?;
            }
        }
    }
    out.flush()
}
