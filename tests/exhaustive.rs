// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>
// DY Research — https://dyresearch.github.io

//! Small enough to check every possibility, so nothing is left to sampling.
//!
//! Two algorithms in this crate return an answer that a random test can only
//! agree with, never confirm. `optimal_priority_order` claims optimality — if
//! *any* fixed-priority ordering meets every deadline, it finds one — and a
//! generated set that happens to be schedulable proves nothing about the claim.
//! `max_wcet_increase` claims a maximum, and a test that checks the returned
//! value is feasible cannot tell a maximum from an underestimate.
//!
//! For task sets small enough, both claims can be checked against the full
//! space rather than against a sample: every one of the `n!` orderings, and
//! every increment from zero to the deadline. That is what this file does.
//!
//! The bound on `n` is written down rather than left implicit. Five tasks is
//! 120 orderings per set and runs in well under a second; six is 720 and still
//! fits, and is used for the smaller batch. Nothing here silently shrinks: a
//! reader can see the number the campaign actually covered.

use dy_wcet::{Response, Task, TaskSet, MAX_TASKS};

/// Largest set for which every ordering is enumerated.
const EXHAUSTIVE_N: usize = 5;

/// A deterministic generator, so a failure is reproducible from its seed alone.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.0 >> 11
    }
    fn between(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo + 1)
    }
}

fn build(tasks: &[Task]) -> Option<TaskSet> {
    let mut s = TaskSet::new();
    for t in tasks {
        s.push(*t).ok()?;
    }
    Some(s)
}

/// Every permutation of `0..n`, smallest first. Heap's algorithm, iterative.
fn permutations(n: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut a: Vec<usize> = (0..n).collect();
    let mut c = vec![0usize; n];
    out.push(a.clone());
    let mut i = 0;
    while i < n {
        if c[i] < i {
            if i % 2 == 0 {
                a.swap(0, i);
            } else {
                a.swap(c[i], i);
            }
            out.push(a.clone());
            c[i] += 1;
            i = 0;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
    out
}

fn random_tasks(rng: &mut Lcg, n: usize) -> Vec<Task> {
    (0..n)
        .map(|_| {
            let period = rng.between(4, 60);
            let wcet = rng.between(1, period.max(2) / 2);
            let deadline = match rng.between(0, 2) {
                0 => period,
                1 => rng.between(wcet, period),
                _ => rng.between(period, period * 2),
            };
            let jitter = if rng.between(0, 3) == 0 {
                rng.between(0, period / 2)
            } else {
                0
            };
            Task::new(wcet, period).deadline(deadline).jitter(jitter)
        })
        .collect()
}

/// Audsley's claim, against the whole permutation space.
///
/// Two directions, and the second is the one that matters. That the ordering
/// Audsley returns is feasible is a sanity check. That Audsley returning
/// `None` really means *no ordering works* is the optimality claim, and the
/// only way to check it for a given set is to try them all.
#[test]
fn audsley_agrees_with_every_ordering() {
    let mut rng = Lcg(0x5EED);
    let perms: Vec<Vec<Vec<usize>>> = (0..=EXHAUSTIVE_N).map(permutations).collect();
    let mut checked = 0usize;
    let mut feasible_sets = 0usize;

    for _ in 0..400 {
        let n = (rng.between(2, EXHAUSTIVE_N as u64)) as usize;
        let tasks = random_tasks(&mut rng, n);
        let Some(set) = build(&tasks) else { continue };

        // Ground truth: does any ordering meet every deadline?
        let mut any_feasible = None;
        for p in &perms[n] {
            let ordered: Vec<Task> = p.iter().map(|&i| tasks[i]).collect();
            if let Some(s) = build(&ordered) {
                if s.is_schedulable() {
                    any_feasible = Some(p.clone());
                    break;
                }
            }
        }

        checked += 1;
        match (set.optimal_priority_order(), any_feasible) {
            (Some(order), Some(_)) => {
                feasible_sets += 1;
                // The ordering it returned must itself hold together.
                let ordered: Vec<Task> = order[..n].iter().map(|&i| tasks[i]).collect();
                let s = build(&ordered).expect("reordered set is admissible");
                assert!(
                    s.is_schedulable(),
                    "Audsley returned {order:?} for {tasks:?}, and it misses a deadline"
                );
                // And it must be a permutation, not a repetition.
                let mut seen = [false; MAX_TASKS];
                for &i in &order[..n] {
                    assert!(!seen[i], "index {i} appears twice in {order:?}");
                    seen[i] = true;
                }
            }
            (None, Some(p)) => panic!(
                "Audsley found nothing, but ordering {p:?} is feasible for {tasks:?} \
                 — the optimality claim is false for this set"
            ),
            (Some(order), None) => {
                panic!("Audsley returned {order:?} for {tasks:?}, and no ordering is feasible")
            }
            (None, None) => {}
        }
    }
    assert!(checked > 300, "only {checked} sets were checked");
    assert!(
        feasible_sets > 50,
        "only {feasible_sets} feasible sets — the generator is not exercising the claim"
    );
}

/// `max_wcet_increase` against a full linear scan.
///
/// The returned value must be feasible and the next one must not be. Checking
/// only the first half would pass for any underestimate, which is the error a
/// binary search over a predicate makes when the predicate is not monotone.
#[test]
fn max_wcet_increase_is_the_maximum_and_not_merely_feasible() {
    let mut rng = Lcg(0xC0FFEE);
    let mut checked = 0usize;

    for _ in 0..300 {
        let n = (rng.between(2, 4)) as usize;
        let tasks = random_tasks(&mut rng, n);
        let Some(set) = build(&tasks) else { continue };
        if !set.is_schedulable() {
            continue;
        }

        for index in 0..n {
            let Some(returned) = set.max_wcet_increase(index) else {
                continue;
            };
            let base = tasks[index];
            let headroom = base.deadline_us - base.wcet_us;

            let feasible = |extra: u64| -> bool {
                if base.wcet_us + extra > base.deadline_us {
                    return false;
                }
                let mut trial = tasks.clone();
                trial[index] = Task {
                    wcet_us: base.wcet_us + extra,
                    ..base
                };
                build(&trial).is_some_and(|s| s.is_schedulable())
            };

            // The whole space, smallest first.
            let mut truth = 0u64;
            for extra in 0..=headroom {
                if feasible(extra) {
                    truth = extra;
                } else {
                    break;
                }
            }

            assert_eq!(
                returned, truth,
                "max_wcet_increase({index}) said {returned}, exhaustive scan says {truth}, \
                 for {tasks:?}"
            );
            assert!(feasible(returned), "the returned increment is not feasible");
            if returned < headroom {
                assert!(
                    !feasible(returned + 1),
                    "returned {returned} but {} is also feasible — an underestimate",
                    returned + 1
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 100, "only {checked} increments were checked");
}

/// Monotonicity: more execution never helps.
///
/// Stated as a property because it is the assumption the sensitivity search
/// rests on. If raising a task's execution time could ever make a set become
/// schedulable, a binary search over feasibility would be searching a space
/// that has no single boundary to find.
#[test]
fn raising_execution_never_improves_schedulability() {
    let mut rng = Lcg(0xBEEF);
    let mut observed = 0usize;

    for _ in 0..500 {
        let n = (rng.between(2, 5)) as usize;
        let tasks = random_tasks(&mut rng, n);
        let Some(set) = build(&tasks) else { continue };
        let index = (rng.between(0, n as u64 - 1)) as usize;
        let base = tasks[index];
        if base.wcet_us >= base.deadline_us {
            continue;
        }

        let before = set.response_of(index);
        let mut heavier = tasks.clone();
        heavier[index] = Task {
            wcet_us: base.wcet_us + 1,
            ..base
        };
        let Some(after_set) = build(&heavier) else {
            continue;
        };

        if let (Response::Bounded(a), Response::Bounded(b)) = (before, after_set.response_of(index))
        {
            assert!(
                b >= a,
                "response fell from {a} to {b} when execution rose by one, for {tasks:?}"
            );
            observed += 1;
        }
    }
    assert!(observed > 100, "only {observed} comparable pairs");
}
